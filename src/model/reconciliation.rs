use super::*;

impl Budget {
    pub fn reconcile(&mut self, account: &str, statement: i64) -> Result<()> {
        self.reconcile_through(account, statement, &today())
    }
    /// Amount needed to bring the dated cleared balance to the statement balance.
    pub fn reconciliation_difference(
        &self,
        account: &str,
        statement: i64,
        date: &str,
    ) -> Result<i64> {
        if self.account(account)?.closed {
            return Err("The account is closed.".into());
        }
        if !valid_date(date) || date > today().as_str() {
            return Err("Некорректная дата или дата в будущем".into());
        }
        if statement.unsigned_abs() > 100_000_000_000_000 {
            return Err("Слишком большая сумма".into());
        }
        let cleared = self
            .transactions
            .iter()
            .filter(|t| t.account == account && t.cleared && t.date.as_str() <= date)
            .try_fold(0_i64, |total, t| {
                total.checked_add(t.amount).ok_or("Слишком большая сумма")
            })?;
        statement
            .checked_sub(cleared)
            .ok_or_else(|| "Слишком большая сумма".into())
    }
    pub fn reconcile_through(&mut self, account: &str, statement: i64, date: &str) -> Result<()> {
        if self.reconciliation_difference(account, statement, date)? != 0 {
            return Err("Остаток не совпадает с подтверждёнными операциями".into());
        }
        for t in &mut self.transactions {
            if t.account == account && t.cleared && t.date.as_str() <= date {
                t.reconciled = true;
            }
        }
        Ok(())
    }
    /// Insert a cleared adjustment and lock eligible rows as one atomic operation.
    /// Pending transactions and the other side of transfers keep their own status.
    pub fn reconcile_with_adjustment(
        &mut self,
        account: &str,
        statement: i64,
        date: &str,
    ) -> Result<()> {
        let difference = self.reconciliation_difference(account, statement, date)?;
        let mut next = self.clone();
        if difference != 0 {
            next.put_transaction(Transaction {
                check_number: None,
                flag: None,
                id: id(),
                account: account.into(),
                date: date.into(),
                payee: "Reconciliation Balance Adjustment".into(),
                memo: "Adjustment to match the bank statement".into(),
                amount: difference,
                base_amount: 0,
                snapshot: None,
                income_month: None,
                category: None,
                cleared: true,
                reconciled: false,
                transfer: None,
                splits: vec![],
            })?;
        }
        next.reconcile_through(account, statement, date)?;
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }
    /// Record the difference as a new transaction, preserving the account's history.
    pub fn adjust_balance(&mut self, account: &str, target: i64, date: &str) -> Result<()> {
        let a = self.account(account)?;
        if a.closed {
            return Err("Счёт закрыт".into());
        }
        if !valid_date(date) || date > today().as_str() {
            return Err("Некорректная дата или дата в будущем".into());
        }
        if target.unsigned_abs() > 100_000_000_000_000 {
            return Err("Слишком большая сумма".into());
        }
        let difference = target
            .checked_sub(self.balance(account, Some(date), false))
            .ok_or("Слишком большая сумма")?;
        if difference == 0 {
            return Ok(());
        }
        self.put_transaction(Transaction {
            check_number: None,
            flag: None,
            id: id(),
            account: account.into(),
            date: date.into(),
            payee: "Корректировка остатка".into(),
            memo: String::new(),
            amount: difference,
            base_amount: 0,
            snapshot: None,
            income_month: None,
            category: None,
            cleared: true,
            reconciled: false,
            transfer: None,
            splits: vec![],
        })
    }
}
