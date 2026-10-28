use super::*;

impl Budget {
    pub fn activity(&self, month: &str, category: &str) -> i64 {
        self.transactions
            .iter()
            .filter(|t| {
                t.date.starts_with(month)
                    && self.has_category_flow(t)
                    && self.account(&t.account).is_ok_and(|a| a.on_budget)
            })
            .map(|t| {
                if t.splits.is_empty() {
                    if t.category.as_deref() == Some(category) {
                        t.base_amount
                    } else {
                        0
                    }
                } else {
                    t.splits
                        .iter()
                        .zip(self.split_base_amounts(t).expect("validated split"))
                        .filter(|(s, _)| {
                            s.category.as_deref() == Some(category)
                                && self.split_has_category_flow(t, s)
                        })
                        .map(|(_, value)| value)
                        .sum()
                }
            })
            .sum()
    }
    pub fn category_balance(&self, month: &str, category: &str) -> i64 {
        let mut months: std::collections::BTreeSet<&str> = self
            .allocations
            .keys()
            .chain(self.overspending_rules.keys())
            .filter_map(|k| k.split_once('/'))
            .filter(|(m, c)| *c == category && *m <= month)
            .map(|(m, _)| m)
            .collect();
        months.extend(
            self.transactions
                .iter()
                .filter(|t| &t.date[..7] <= month)
                .map(|t| &t.date[..7]),
        );
        months.insert(month);
        let mut balance = 0;
        for m in months {
            balance += self.allocated(m, category) + self.activity(m, category);
            if m < month
                && matches!(
                    self.overspending_rule(m, category),
                    OverspendingRule::AvailableToBudget
                )
            {
                balance = balance.max(0);
            }
        }
        balance
    }
    pub fn overspending_rule(&self, month: &str, category: &str) -> &OverspendingRule {
        self.overspending_rules
            .iter()
            .rev()
            .find(|(key, _)| {
                key.split_once('/')
                    .is_some_and(|(m, c)| c == category && m <= month)
            })
            .map(|(_, rule)| rule)
            .unwrap_or(&self.default_overspending_rule)
    }
    pub fn set_overspending_rule(
        &mut self,
        month: &str,
        category: &str,
        rule: OverspendingRule,
    ) -> Result<()> {
        if !valid_month(month) || !self.categories.iter().any(|c| c.id == category) {
            return Err("Некорректный месяц или категория".into());
        }
        self.overspending_rules
            .insert(format!("{month}/{category}"), rule);
        Ok(())
    }
    pub fn overspent(&self, month: &str) -> i64 {
        self.categories
            .iter()
            .filter(|c| {
                matches!(
                    self.overspending_rule(month, &c.id),
                    OverspendingRule::AvailableToBudget
                )
            })
            .map(|c| self.category_balance(month, &c.id).min(0))
            .sum()
    }
    pub fn month_summary(&self, month: &str) -> Result<MonthSummary> {
        if !valid_month(month) {
            return Err("Некорректный месяц или категория".into());
        }
        let previous = month_shift(month, -1);
        let not_budgeted = self.available(&previous)?;
        let available = self.available(month)?;
        let overspent = self.overspent(&previous);
        let budgeted = self
            .categories
            .iter()
            .map(|c| self.allocated(month, &c.id))
            .sum();
        let activity: i64 = self
            .categories
            .iter()
            .map(|c| self.activity(month, &c.id))
            .sum();
        let income = self.net_worth(month, true)? - self.net_worth(&previous, true)? - activity
            + self.deferred_income(&previous)
            - self.deferred_income(month);
        Ok(MonthSummary {
            not_budgeted,
            overspent,
            budgeted,
            available,
            income,
        })
    }
    pub fn month_end(month: &str) -> String {
        let next = NaiveDate::parse_from_str(&format!("{}-01", month_shift(month, 1)), "%Y-%m-%d")
            .unwrap();
        next.pred_opt().unwrap().to_string()
    }
    pub fn net_worth(&self, month: &str, on_budget_only: bool) -> Result<i64> {
        let end = Self::month_end(month);
        let mut sum = 0;
        for a in &self.accounts {
            if on_budget_only && !a.on_budget {
                continue;
            }
            let balance = self.balance(&a.id, Some(&end), false);
            if balance != 0 {
                sum += self.to_base(balance, &a.currency, month)?.0;
            }
        }
        Ok(sum)
    }
    pub fn available(&self, month: &str) -> Result<i64> {
        Ok(self.net_worth(month, true)?
            - self.deferred_income(month)
            - self
                .categories
                .iter()
                .map(|c| self.category_balance(month, &c.id))
                .sum::<i64>())
    }
}
