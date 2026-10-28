use super::*;

impl Budget {
    pub fn balance(&self, account: &str, through: Option<&str>, cleared_only: bool) -> i64 {
        self.transactions
            .iter()
            .filter(|t| {
                t.account == account
                    && through.is_none_or(|date| t.date.as_str() <= date)
                    && (!cleared_only || t.cleared)
            })
            .map(|t| t.amount)
            .sum()
    }
    pub fn allocated(&self, month: &str, category: &str) -> i64 {
        *self
            .allocations
            .get(&format!("{month}/{category}"))
            .unwrap_or(&0)
    }
    pub fn allocate(&mut self, month: &str, category: &str, value: i64) -> Result<()> {
        if !valid_month(month) || !self.categories.iter().any(|c| c.id == category) {
            return Err("Некорректный месяц или категория".into());
        }
        self.allocations
            .insert(format!("{month}/{category}"), value);
        Ok(())
    }
}
