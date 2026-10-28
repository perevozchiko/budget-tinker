//! legacy old debt is a negative category balance, reduced by budgeting.
//! Payments are ordinary paired transfers; new purchases keep their category.
use crate::model::*;

pub const INITIAL_DEBT_GROUP: &str = concat!("Pre-Y", "NAB Debt");
pub const INITIAL_DEBT_PREFIX: &str = concat!("Category/PreY", "NABDebt/");

pub fn debt_account(category: &str) -> Option<&str> {
    category.strip_prefix(INITIAL_DEBT_PREFIX)
}

impl Budget {
    pub fn initial_debt_category(&self, account: &str) -> Option<&Category> {
        self.categories
            .iter()
            .find(|c| debt_account(&c.id) == Some(account))
    }

    pub(crate) fn create_initial_debt_category(
        &mut self,
        account: &str,
        name: &str,
        month: &str,
    ) -> String {
        let category = format!("{INITIAL_DEBT_PREFIX}{account}");
        self.overspending_rules.insert(
            format!("{month}/{category}"),
            OverspendingRule::CategoryBalance,
        );
        if self.initial_debt_category(account).is_some() {
            return category;
        }
        if !self.group_names().iter().any(|g| g == INITIAL_DEBT_GROUP) {
            self.category_groups.insert(
                0,
                CategoryGroup {
                    name: INITIAL_DEBT_GROUP.into(),
                    hidden: false,
                },
            );
        }
        self.categories.insert(
            0,
            Category {
                id: category.clone(),
                group: INITIAL_DEBT_GROUP.into(),
                name: name.trim().replace(':', "-"),
                hidden: false,
            },
        );
        category
    }

    pub fn rename_account(&mut self, account: &str, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err("Enter an account name".into());
        }
        let a = self
            .accounts
            .iter_mut()
            .find(|a| a.id == account)
            .ok_or("Account not found")?;
        a.name = name.trim().into();
        if let Some(c) = self
            .categories
            .iter_mut()
            .find(|c| debt_account(&c.id) == Some(account))
        {
            c.name = name.trim().replace(':', "-");
        }
        Ok(())
    }
}
