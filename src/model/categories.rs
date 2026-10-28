use super::*;

impl Budget {
    pub fn group_names(&self) -> Vec<String> {
        let mut names = self
            .category_groups
            .iter()
            .map(|g| g.name.clone())
            .collect::<Vec<_>>();
        for category in &self.categories {
            if !names.contains(&category.group) {
                names.push(category.group.clone());
            }
        }
        names
    }
    pub fn add_category(&mut self, group: &str, name: &str) -> Result<String> {
        let group = group.trim();
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a category name".into());
        }
        if !self.group_names().iter().any(|g| g == group) {
            return Err("Master category not found".into());
        }
        if self
            .categories
            .iter()
            .any(|c| c.group == group && c.name.eq_ignore_ascii_case(name))
        {
            return Err("A category with this name already exists in this group".into());
        }
        let category = Category {
            id: id(),
            group: group.into(),
            name: name.into(),
            hidden: false,
        };
        let id = category.id.clone();
        let index = self
            .categories
            .iter()
            .rposition(|c| c.group == group)
            .map_or(self.categories.len(), |i| i + 1);
        self.categories.insert(index, category);
        Ok(id)
    }
    pub fn add_category_group(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a master category name".into());
        }
        if self.group_names().iter().any(|g| g == name) {
            return Err("A master category with this name already exists".into());
        }
        self.category_groups.insert(
            0,
            CategoryGroup {
                name: name.into(),
                hidden: false,
            },
        );
        Ok(())
    }
    pub fn rename_group(&mut self, old: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() || !self.group_names().iter().any(|g| g == old) {
            return Err("Введите название существующей группы".into());
        }
        if old == name {
            return Ok(());
        }
        if self.group_names().iter().any(|g| g == name) {
            return Err("Группа с таким названием уже существует".into());
        }
        for group in &mut self.category_groups {
            if group.name == old {
                group.name = name.into();
            }
        }
        for category in &mut self.categories {
            if category.group == old {
                category.group = name.into();
            }
        }
        if let Some(note) = self.notes.remove(&format!("group:{old}")) {
            self.notes.insert(format!("group:{name}"), note);
        }
        Ok(())
    }
    /// Move a category before or after another category, or to the end of a group.
    /// The vector order is the visible budget order and is persisted with the budget.
    pub fn move_category(
        &mut self,
        id: &str,
        group: &str,
        neighbor: Option<(&str, bool)>,
    ) -> Result<()> {
        if !self.group_names().iter().any(|g| g == group) {
            return Err("Группа не найдена".into());
        }
        let source = self
            .categories
            .iter()
            .position(|c| c.id == id)
            .ok_or("Категория не найдена")?;
        if neighbor.is_some_and(|(target, _)| target == id) {
            return Ok(());
        }
        if let Some((target, _)) = neighbor {
            let target_category = self
                .categories
                .iter()
                .find(|c| c.id == target)
                .ok_or("Категория назначения не найдена")?;
            if target_category.group != group || target_category.hidden {
                return Err("Категория назначения находится в другой группе".into());
            }
        }
        let mut category = self.categories.remove(source);
        category.group = group.into();
        category.hidden = false;
        let index = if let Some((target, after)) = neighbor {
            let index = self.categories.iter().position(|c| c.id == target).unwrap();
            index + usize::from(after)
        } else {
            self.categories
                .iter()
                .rposition(|c| c.group == group)
                .map_or(0, |i| i + 1)
        };
        self.categories.insert(index, category);
        Ok(())
    }
    pub fn set_group_hidden(&mut self, group: &str, hidden: bool) -> Result<()> {
        if !self.group_names().iter().any(|g| g == group) {
            return Err("Группа не найдена".into());
        }
        for metadata in &mut self.category_groups {
            if metadata.name == group {
                metadata.hidden = hidden;
            }
        }
        for category in &mut self.categories {
            if category.group == group {
                category.hidden = hidden;
            }
        }
        Ok(())
    }
    pub fn delete_group(&mut self, group: &str) -> Result<()> {
        let ids: std::collections::HashSet<_> = self
            .categories
            .iter()
            .filter(|c| c.group == group)
            .map(|c| c.id.clone())
            .collect();
        if ids.is_empty() && !self.category_groups.iter().any(|g| g.name == group) {
            return Err("Группа не найдена".into());
        }
        let uses_category = |t: &Transaction| {
            t.category.as_ref().is_some_and(|c| ids.contains(c))
                || t.splits
                    .iter()
                    .any(|s| s.category.as_ref().is_some_and(|c| ids.contains(c)))
        };
        if self.transactions.iter().any(uses_category)
            || self.scheduled.iter().any(|s| uses_category(&s.transaction))
            || self.allocations.iter().any(|(key, amount)| {
                *amount != 0 && key.split_once('/').is_some_and(|(_, c)| ids.contains(c))
            })
        {
            return Err("Группа содержит используемые категории".into());
        }
        self.categories.retain(|c| c.group != group);
        self.category_groups.retain(|g| g.name != group);
        self.allocations
            .retain(|key, _| key.split_once('/').is_none_or(|(_, c)| !ids.contains(c)));
        self.overspending_rules
            .retain(|key, _| key.split_once('/').is_none_or(|(_, c)| !ids.contains(c)));
        self.notes.remove(&format!("group:{group}"));
        for id in &ids {
            self.notes.remove(&format!("category:{id}"));
        }
        for rule in self.payee_rules.values_mut() {
            if rule.category.as_ref().is_some_and(|id| ids.contains(id)) {
                rule.category = None;
            }
        }
        Ok(())
    }
}
