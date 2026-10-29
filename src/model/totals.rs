use super::*;

impl Budget {
    /// Up to twelve complete months, starting with the first recorded budget activity.
    pub fn average_category_outflow(&self, month: &str, category: &str) -> (i64, usize) {
        self.average_category_outflow_over(month, category, 12)
    }

    /// Average over available complete months, capped at `limit` (1–12).
    pub fn average_category_outflow_over(
        &self,
        month: &str,
        category: &str,
        limit: usize,
    ) -> (i64, usize) {
        let first = self
            .transactions
            .iter()
            .map(|t| &t.date[..7])
            .chain(
                self.allocations
                    .keys()
                    .filter_map(|key| key.split_once('/').map(|(m, _)| m)),
            )
            .filter(|m| *m < month)
            .min();
        let Some(first) = first else { return (0, 0) };
        let months: Vec<_> = (1..=limit.clamp(1, 12))
            .map(|n| month_shift(month, -(n as i32)))
            .take_while(|m| m.as_str() >= first)
            .collect();
        let total: i128 = months
            .iter()
            .map(|m| i128::from((-self.activity(m, category)).max(0)))
            .sum();
        let count = months.len();
        (((total + count as i128 / 2) / count as i128) as i64, count)
    }

    /// Estimate upcoming category outflows using the most recent available exchange rate.
    pub fn scheduled_category_outflows(&self, month: &str, category: &str) -> Result<i64> {
        if !valid_month(month) {
            return Err("Invalid month".into());
        }
        let end = Self::month_end(month);
        let mut total = 0_i64;
        for schedule in &self.scheduled {
            let t = &schedule.transaction;
            let account = self.account(&t.account)?;
            if !account.on_budget || !self.has_category_flow(t) {
                continue;
            }
            let amount: i64 = if t.splits.is_empty() {
                if t.category.as_deref() == Some(category) {
                    t.amount.min(0)
                } else {
                    0
                }
            } else {
                t.splits
                    .iter()
                    .filter(|s| {
                        s.category.as_deref() == Some(category)
                            && self.split_has_category_flow(t, s)
                    })
                    .map(|s| s.amount.min(0))
                    .try_fold(0_i64, |a, v| {
                        a.checked_add(v).ok_or("Scheduled outflows are too large")
                    })?
            };
            if amount == 0 {
                continue;
            }
            let mut date = schedule.next_date.clone();
            let anchor = schedule
                .anchor_day
                .unwrap_or_else(|| date[8..].parse().unwrap_or(1));
            while date <= end {
                if &date[..7] == month {
                    let converted = if account.currency == self.base {
                        amount
                    } else {
                        let snapshot = self
                            .snapshots
                            .iter()
                            .filter(|s| s.currency == account.currency && s.month.as_str() <= month)
                            .max_by_key(|s| &s.month)
                            .ok_or("No exchange rate available for scheduled outflows")?;
                        convert(amount, &account.currency, &self.base, snapshot.rate)?
                    };
                    total = total
                        .checked_sub(converted)
                        .ok_or("Scheduled outflows are too large")?;
                }
                match schedule.frequency().next(&date, anchor)? {
                    Some(next) if next > date => date = next,
                    Some(_) => return Err("Invalid schedule recurrence".into()),
                    None => break,
                }
            }
        }
        Ok(total)
    }
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
