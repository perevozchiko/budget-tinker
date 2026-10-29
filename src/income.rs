//! Budget timing is independent of the date cash reaches the account.
use crate::model::*;

pub fn income_label(month: &str) -> String {
    chrono::NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d")
        .map(|date| format!("Income for {}", date.format("%B")))
        .unwrap_or_else(|_| "Income / Uncategorized".into())
}

/// Our CSV records an exact month; legacyexports may describe this/next month.
pub fn imported_month(category: &str, explicit: &str, date: &str) -> Result<Option<String>> {
    if !explicit.is_empty() {
        if !valid_month(explicit) {
            return Err("Invalid income month".into());
        }
        return Ok(Some(explicit.into()));
    }
    if matches!(
        category,
        "Income: Available next month" | "Income:Available next month"
    ) {
        return Ok(Some(month_shift(&date[..7], 1)));
    }
    if category.starts_with("Income for ") {
        for delta in [0, 1] {
            let month = month_shift(&date[..7], delta);
            if category == income_label(&month) {
                return Ok(Some(month));
            }
        }
        return Err("Income category must name this or next month".into());
    }
    Ok(None)
}

impl Transaction {
    pub fn validate_income_month(&self) -> Result<()> {
        if let Some(month) = &self.income_month {
            if !valid_date(&self.date) || !valid_month(month) {
                return Err("Invalid income month".into());
            }
            let current = &self.date[..7];
            if self.category.is_some()
                || !self.splits.is_empty()
                || (month != current && *month != month_shift(current, 1))
            {
                return Err("Income must use the transaction month or the following month, without a category or split".into());
            }
        }
        Ok(())
    }

    /// Preserve current/next month intent when changing a date or posting a recurrence.
    pub fn set_date(&mut self, date: String) {
        if let Some(month) = &self.income_month
            && valid_date(&self.date)
            && valid_date(&date)
        {
            let next = month.as_str() > &self.date[..7];
            self.income_month = Some(month_shift(&date[..7], i32::from(next)));
        }
        if valid_date(&self.date) && valid_date(&date) {
            for part in &mut self.splits {
                if let Some(month) = &part.income_month {
                    part.income_month = Some(month_shift(
                        &date[..7],
                        i32::from(month.as_str() > &self.date[..7]),
                    ));
                }
            }
        }
        self.date = date;
    }
}

impl Budget {
    /// Frozen base-currency income contributions, including split income and deferrals.
    pub fn income_rows(&self, month: &str) -> Result<Vec<(&Transaction, i64)>> {
        if !valid_month(month) {
            return Err("Invalid month".into());
        }
        let mut rows = Vec::new();
        for t in &self.transactions {
            if let Some(value) = self.income_contribution(t, month)? {
                rows.push((t, value));
            }
        }
        rows.sort_by(|(a, _), (b, _)| a.date.cmp(&b.date).then(a.id.cmp(&b.id)));
        Ok(rows)
    }

    pub fn income_contribution(&self, t: &Transaction, month: &str) -> Result<Option<i64>> {
        if !valid_month(month) {
            return Err("Invalid month".into());
        }
        if !self.account(&t.account)?.on_budget {
            return Ok(None);
        }
        Ok(if t.splits.is_empty() {
            let external = t.transfer.as_ref().is_none_or(|link| {
                self.transactions.iter().any(|peer| {
                    peer.id != t.id
                        && peer.transfer.as_ref() == Some(link)
                        && self.account(&peer.account).is_ok_and(|a| !a.on_budget)
                })
            });
            if t.category.is_none()
                && (external || t.income_month.is_some())
                && t.income_month.as_deref().unwrap_or(&t.date[..7]) == month
            {
                Some(t.base_amount)
            } else {
                None
            }
        } else {
            let values = self.split_base_amounts(t)?;
            let parts: Vec<_> = t
                .splits
                .iter()
                .zip(values)
                .filter(|(part, _)| {
                    part.category.is_none()
                        && self.split_has_category_flow(t, part)
                        && part.income_month.as_deref().unwrap_or(&t.date[..7]) == month
                })
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.iter().map(|(_, value)| value).sum())
            }
        })
    }

    /// Hold only the frozen base value; exchange revaluation keeps its existing timing.
    pub fn deferred_income(&self, month: &str) -> i64 {
        self.transactions
            .iter()
            .filter(|t| {
                t.date.get(..7).is_some_and(|date| date <= month)
                    && self.account(&t.account).is_ok_and(|a| a.on_budget)
            })
            .map(|t| {
                if t.splits.is_empty() {
                    if t.income_month
                        .as_deref()
                        .is_some_and(|assigned| assigned > month)
                    {
                        t.base_amount
                    } else {
                        0
                    }
                } else {
                    t.splits
                        .iter()
                        .zip(self.split_base_amounts(t).expect("validated split"))
                        .filter(|(part, _)| {
                            self.split_has_category_flow(t, part)
                                && part
                                    .income_month
                                    .as_deref()
                                    .is_some_and(|assigned| assigned > month)
                        })
                        .map(|(_, value)| value)
                        .sum()
                }
            })
            .sum()
    }

    pub fn transaction_category_name(&self, t: &Transaction) -> String {
        if !t.splits.is_empty() {
            return "Split (Multiple Categories)".into();
        }
        if t.category.is_none()
            && t.splits.is_empty()
            && (t.transfer.is_none() || t.income_month.is_some())
            && (t.amount > 0 || t.income_month.is_some())
        {
            income_label(t.income_month.as_deref().unwrap_or(&t.date[..7]))
        } else {
            self.category_name(&t.category)
        }
    }
}
