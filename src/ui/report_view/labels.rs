use super::*;

impl App {
    /// Render the actual account scope; BudgetAccounts and All are distinct.
    pub(in crate::ui) fn selection_label(&self, sel: &Selection, _kind: &str) -> String {
        match sel {
            Selection::BudgetAccounts => "Budget Accounts".to_owned(),
            Selection::All => "All Accounts".to_owned(),
            Selection::NoneSelected => "No Accounts".to_owned(),
            Selection::Selected(ids) if ids.is_empty() => "No Accounts".to_owned(),
            Selection::Selected(ids) => {
                let names: Vec<_> = ids
                    .iter()
                    .filter_map(|id| self.b.account(id).ok().map(|a| a.name.clone()))
                    .collect();
                if names.is_empty() {
                    "No Accounts".to_owned()
                } else {
                    names.join(", ")
                }
            }
        }
    }

    /// Render category names with their groups and distinguish empty payees.
    pub(in crate::ui) fn selection_display_names(&self, sel: &Selection, kind: &str) -> String {
        match sel {
            Selection::All => match kind {
                "categories" => "All Categories".to_owned(),
                "payees" => "All Payees".to_owned(),
                _ => "All".to_owned(),
            },
            Selection::BudgetAccounts => "Budget Accounts".to_owned(),
            Selection::NoneSelected => format!(
                "No {}",
                if kind == "categories" {
                    "Categories"
                } else {
                    "Payees"
                }
            ),
            Selection::Selected(ids) if ids.is_empty() => format!(
                "No {}",
                if kind == "categories" {
                    "Categories"
                } else {
                    "Payees"
                }
            ),
            Selection::Selected(ids) => {
                let names: Vec<_> = if kind == "categories" {
                    ids.iter()
                        .filter_map(|id| {
                            if id == budgettinker::model::UNCATEGORIZED_CATEGORY {
                                return Some("Uncategorized Transactions".to_owned());
                            }
                            self.b
                                .categories
                                .iter()
                                .find(|c| c.id == *id)
                                .map(|c| format!("{}: {}", c.group, c.name))
                        })
                        .collect()
                } else {
                    ids.iter()
                        .map(|name| {
                            if name.is_empty() {
                                "No Payee".to_owned()
                            } else {
                                name.clone()
                            }
                        })
                        .collect()
                };
                if names.is_empty() {
                    "None".to_owned()
                } else {
                    names.join(", ")
                }
            }
        }
    }
}
