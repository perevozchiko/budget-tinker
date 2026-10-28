use super::*;

impl Budget {
    pub fn add_account(
        &mut self,
        name: &str,
        currency: &str,
        on_budget: bool,
        opening: i64,
        date: &str,
    ) -> Result<String> {
        self.add_typed_account(name, currency, on_budget, opening, date, "checking")
    }
    pub fn add_typed_account(
        &mut self,
        name: &str,
        currency: &str,
        on_budget: bool,
        opening: i64,
        date: &str,
        account_type: &str,
    ) -> Result<String> {
        if name.trim().is_empty() || !CURRENCIES.contains(&currency) {
            return Err("Введите название счёта и валюту".into());
        }
        if !valid_date(date) || date > today().as_str() {
            return Err("Некорректная дата или дата в будущем".into());
        }
        if opening.unsigned_abs() > 100_000_000_000_000 {
            return Err("Слишком большая сумма".into());
        }
        let (base_amount, snapshot) = self.to_base(opening, currency, &date[..7])?;
        let account = id();
        self.accounts.push(Account {
            id: account.clone(),
            name: name.trim().into(),
            currency: currency.into(),
            on_budget,
            closed: false,
            account_type: account_type.into(),
        });
        // legacy categorizes a negative on-budget starting balance as old
        // debt. A zero-balance debt account also gets its own debt category.
        let category = if on_budget
            && (opening < 0
                || (opening == 0
                    && matches!(
                        account_type,
                        "credit" | "line_of_credit" | "mortgage" | "liability"
                    ))) {
            Some(self.create_initial_debt_category(&account, name, &date[..7]))
        } else {
            None
        };
        if opening != 0 {
            self.transactions.push(Transaction {
                check_number: None,
                flag: None,
                id: id(),
                account: account.clone(),
                date: date.into(),
                payee: "Starting Balance".into(),
                memo: String::new(),
                amount: opening,
                base_amount,
                snapshot,
                income_month: None,
                category,
                cleared: true,
                reconciled: false,
                transfer: None,
                splits: vec![],
            });
        }
        Ok(account)
    }
}
