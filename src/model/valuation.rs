use super::*;

impl Budget {
    pub fn account(&self, id: &str) -> Result<&Account> {
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .ok_or("Счёт не найден".into())
    }
    /// Internal transfers have no category flow. A categorized budget side of
    /// a transfer across the budget boundary does, just like an ordinary row.
    pub fn has_category_flow(&self, t: &Transaction) -> bool {
        t.transfer.is_none()
            || (self.account(&t.account).is_ok_and(|a| a.on_budget)
                && (t.category.is_some() || t.income_month.is_some()))
    }

    pub fn imported_payee(&self, name: &str) -> String {
        self.payee_rules
            .iter()
            .find(|(_, rule)| {
                !rule.rename_contains.trim().is_empty()
                    && name
                        .to_lowercase()
                        .contains(&rule.rename_contains.to_lowercase())
            })
            .map(|(payee, _)| payee.clone())
            .unwrap_or_else(|| name.into())
    }
    pub fn category_name(&self, id: &Option<String>) -> String {
        id.as_ref()
            .and_then(|id| self.categories.iter().find(|c| c.id == *id))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Доход / без категории".into())
    }
    pub fn snapshot(&self, month: &str, currency: &str) -> Result<&Snapshot> {
        self.snapshots.iter().find(|s|s.month==month && s.currency==currency).ok_or_else(||format!("Невозможно рассчитать суммы старого бюджета за {month}: не хватает сохранённых данных."))
    }
    pub fn to_base(
        &self,
        value: i64,
        currency: &str,
        month: &str,
    ) -> Result<(i64, Option<String>)> {
        if currency == self.base || value == 0 {
            return Ok((value, None));
        }
        let s = self.snapshot(month, currency)?;
        Ok((
            convert(value, currency, &self.base, s.rate)?,
            Some(s.id.clone()),
        ))
    }
    /// A zero-net split can still move money between categories. Freeze its
    /// exchange rate even though the account movement itself is zero.
    pub fn transaction_value(&self, t: &Transaction) -> Result<(i64, Option<String>)> {
        let account = self.account(&t.account)?;
        if t.amount == 0 && account.currency != self.base && t.splits.iter().any(|s| s.amount != 0)
        {
            return Ok((
                0,
                Some(self.snapshot(&t.date[..7], &account.currency)?.id.clone()),
            ));
        }
        self.to_base(t.amount, &account.currency, &t.date[..7])
    }

    /// Convert parts using the transaction's frozen rate. The final nonzero
    /// part absorbs rounding; zero parts never acquire a rounding adjustment.
    pub fn split_base_amounts(&self, t: &Transaction) -> Result<Vec<i64>> {
        let account = self.account(&t.account)?;
        let last = t.splits.iter().rposition(|s| s.amount != 0);
        let mut remainder = t.base_amount;
        t.splits
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let value = if s.amount == 0 {
                    0
                } else if Some(i) == last {
                    remainder
                } else if account.currency == self.base {
                    s.amount
                } else {
                    let snapshot = self
                        .snapshots
                        .iter()
                        .find(|snapshot| Some(&snapshot.id) == t.snapshot.as_ref())
                        .ok_or("Missing exchange snapshot for split transaction")?;
                    convert(s.amount, &account.currency, &self.base, snapshot.rate)?
                };
                remainder -= value;
                Ok(value)
            })
            .collect()
    }
    pub fn add_snapshot(
        &mut self,
        month: &str,
        currency: &str,
        rate: i64,
        rate_date: &str,
        source: &str,
    ) -> Result<()> {
        if !valid_month(month)
            || !CURRENCIES.contains(&currency)
            || currency == self.base
            || rate <= 0
            || rate > 100_000_000_000_000
        {
            return Err("Проверьте месяц, валюту и курс".into());
        }
        let date = NaiveDate::parse_from_str(rate_date, "%Y-%m-%d")
            .map_err(|_| "Некорректная дата курса")?;
        let end = NaiveDate::parse_from_str(&format!("{}-01", month_shift(month, 1)), "%Y-%m-%d")
            .unwrap();
        if date >= end || date > Local::now().date_naive() {
            return Err(
                "Дата курса должна быть не позднее выбранного месяца и сегодняшнего дня".into(),
            );
        }
        if self.snapshot(month, currency).is_ok() {
            return Err("Снимок за этот месяц уже зафиксирован и не перезаписывается".into());
        }
        self.snapshots.push(Snapshot {
            id: id(),
            month: month.into(),
            currency: currency.into(),
            rate,
            rate_date: rate_date.into(),
            captured_at: Local::now().to_rfc3339(),
            source: source.into(),
        });
        Ok(())
    }
}
