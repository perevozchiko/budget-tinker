use super::*;

impl Budget {
    pub fn prepare_transaction(&self, t: Transaction) -> Result<Transaction> {
        self.prepare_transaction_inner(t, false)
    }
    pub fn prepare_scheduled_transaction(&self, t: Transaction) -> Result<Transaction> {
        self.prepare_transaction_inner(t, true)
    }
    pub(super) fn prepare_transaction_inner(
        &self,
        mut t: Transaction,
        scheduled: bool,
    ) -> Result<Transaction> {
        if !valid_date(&t.date) {
            return Err("Дата должна быть в формате ГГГГ-ММ-ДД".into());
        }
        t.validate_income_month()?;
        if t.amount.unsigned_abs() > 100_000_000_000_000 {
            return Err("Сумма операции превышает лимит".into());
        }
        if !scheduled && t.date > today() {
            return Err("Будущие операции добавляйте как запланированные".into());
        }
        let a = self.account(&t.account)?;
        if a.closed {
            return Err("Счёт закрыт".into());
        }
        if t.transfer.is_some() {
            return Err("Для перевода используйте отдельную форму".into());
        }
        if let Some(c) = &t.category
            && !self.categories.iter().any(|x| x.id == *c)
        {
            return Err("Категория не найдена".into());
        }
        self.validate_splits(&t)?;
        if scheduled {
            t.cleared = false;
            t.base_amount = 0;
            t.snapshot = None;
            t.operation_fx = None;
            return Ok(t);
        }
        if let Some(old) = self.transactions.iter().find(|old| old.id == t.id)
            && old.reconciled
        {
            return Err("Сверенную операцию нельзя изменять".into());
        }
        // A fully prepared v2 posting may carry explicitly confirmed operation FX.
        // Keep it through the editor -> command second validation pass.
        if self.version == 2
            && let Some(fx) = &t.operation_fx
            && fx.validate_transaction(self, &t).is_ok()
        {
            return Ok(t);
        }
        if let Some(old) = self.transactions.iter().find(|old| old.id == t.id)
            && old.amount == t.amount
            && old.date[..7] == t.date[..7]
            && old.account == t.account
            && (t.amount != 0
                || old.splits.iter().any(|s| s.amount != 0)
                    == t.splits.iter().any(|s| s.amount != 0))
        {
            t.base_amount = old.base_amount;
            t.snapshot = old.snapshot.clone();
            t.operation_fx = old.operation_fx.clone();
            self.price_posting(&mut t)?;
            return Ok(t);
        }
        if let Some(old) = self.transactions.iter().find(|old| old.id == t.id)
            && old.account == t.account
            && self.version == 2
        {
            t.operation_fx = old.operation_fx.clone();
        }
        self.price_posting(&mut t)?;
        Ok(t)
    }
    pub fn put_transaction(&mut self, t: Transaction) -> Result<()> {
        let t = self.prepare_transaction(t)?;
        let old = self.transactions.iter().find(|x| x.id == t.id).cloned();
        if t.has_transfers() || old.as_ref().is_some_and(|t| t.has_transfers()) {
            let mut next = self.clone();
            next.sync_split_peers(old.as_ref(), &t)?;
            let tid = t.id.clone();
            let date = t.date.clone();
            if let Some(row) = next.transactions.iter_mut().find(|x| x.id == t.id) {
                *row = t;
            } else {
                next.transactions.push(t);
            }
            next.sync_component_date(&tid, &date, false)?;
            crate::store::validate(&next)?;
            *self = next;
        } else if let Some(row) = self.transactions.iter_mut().find(|x| x.id == t.id) {
            *row = t;
        } else {
            self.transactions.push(t);
        }
        Ok(())
    }
    pub fn transfer(
        &mut self,
        from: &str,
        to: &str,
        out: i64,
        incoming: i64,
        date: &str,
        memo: &str,
    ) -> Result<()> {
        if from == to || out <= 0 || incoming <= 0 || !valid_date(date) || date > today().as_str() {
            return Err("Проверьте счета, дату и положительные суммы перевода".into());
        }
        let a = self.account(from)?.clone();
        let b = self.account(to)?.clone();
        if a.closed || b.closed {
            return Err("Счёт закрыт".into());
        }
        if a.currency == b.currency && out != incoming {
            return Err("Суммы перевода в одной валюте должны совпадать".into());
        }
        let (out_base, out_snap) = self.to_base(-out, &a.currency, &date[..7])?;
        let (in_base, in_snap) = self.to_base(incoming, &b.currency, &date[..7])?;
        let link = id();
        for (account, amount, base_amount, snapshot, payee) in [
            (
                a.id,
                -out,
                out_base,
                out_snap,
                format!("Перевод → {}", b.name),
            ),
            (
                b.id,
                incoming,
                in_base,
                in_snap,
                format!("Перевод ← {}", a.name),
            ),
        ] {
            let mut posting = Transaction {
                operation_fx: None,
                check_number: None,
                flag: None,
                id: id(),
                account,
                date: date.into(),
                payee,
                memo: memo.into(),
                amount,
                base_amount,
                snapshot,
                income_month: None,
                category: None,
                cleared: false,
                reconciled: false,
                transfer: Some(link.clone()),
                splits: vec![],
            };
            self.freeze_posting(&mut posting)?;
            self.transactions.push(posting);
        }
        Ok(())
    }
    pub fn delete_transaction(&mut self, tid: &str) -> Result<()> {
        let t = self
            .transactions
            .iter()
            .find(|t| t.id == tid)
            .ok_or("Transaction not found")?
            .clone();
        if t.reconciled {
            return Err("Reconciled transaction cannot be deleted".into());
        }
        let mut next = self.clone();
        for link in t.transfer_links() {
            next.remove_transfer_peer(link, tid, false)?;
        }
        next.transactions.retain(|t| t.id != tid);
        next.register_meta
            .retain(|id, _| next.transactions.iter().any(|t| &t.id == id));
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }
}
