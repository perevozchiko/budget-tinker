use super::*;

impl Draft {
    pub fn new(b: &Budget, account: Option<&str>, transaction: Option<&str>) -> Result<Self> {
        let mut draft = Self {
            id: None,
            account: account
                .and_then(|id| b.account(id).ok())
                .filter(|a| !a.closed)
                .or_else(|| b.accounts.iter().find(|a| !a.closed))
                .map(|a| a.id.clone())
                .unwrap_or_default(),
            date: today(),
            payee: String::new(),
            category: String::new(),
            income_next: None,
            memo: String::new(),
            outflow: String::new(),
            inflow: String::new(),
            cleared: false,
            error: String::new(),
            splits: vec![],
            split_details: vec![],
            transfer_to: None,
            pending_category: None,
            date_input: Default::default(),
            focus: true,
            complete_payee: false,
            completed_account: String::new(),
            automatic_amount: None,
        };
        if let Some(id) = transaction {
            let t = b
                .transactions
                .iter()
                .find(|t| t.id == id)
                .ok_or("Операция не найдена")?;
            if t.transfer.is_some() || t.reconciled {
                return Err("Нельзя редактировать перевод или сверенную операцию".into());
            }
            let currency = &b.account(&t.account)?.currency;
            draft.id = Some(id.into());
            draft.account = t.account.clone();
            draft.date = t.date.clone();
            draft.payee = t.payee.clone();
            draft.category = t.category.clone().unwrap_or_default();
            draft.income_next = t
                .income_month
                .as_ref()
                .map(|m| m.as_str() > &t.date[..7])
                .or_else(|| {
                    (t.category.is_none() && t.splits.is_empty() && t.amount > 0).then_some(false)
                });
            draft.memo = t.memo.clone();
            draft.cleared = t.cleared;
            if t.amount < 0 {
                draft.outflow = plain(-t.amount, currency);
            } else {
                draft.inflow = plain(t.amount, currency);
            }
            draft.split_details = t
                .splits
                .iter()
                .map(|p| SplitDetails::from_part(p, &t.date))
                .collect();
            draft.splits = t
                .splits
                .iter()
                .map(|s| {
                    (
                        s.category.clone().unwrap_or_default(),
                        plain(s.amount, currency),
                    )
                })
                .collect();
        }
        Ok(draft)
    }
    pub fn from_scheduled(b: &Budget, schedule: &Scheduled) -> Result<Self> {
        let mut t = schedule.transaction.clone();
        t.set_date(schedule.next_date.clone());
        let currency = &b.account(&t.account)?.currency;
        let mut draft = Self::new(b, Some(&t.account), None)?;
        draft.id = Some(schedule.id.clone());
        draft.account = t.account.clone();
        draft.date = t.date.clone();
        draft.payee = t.payee.clone();
        draft.category = t.category.clone().unwrap_or_default();
        draft.income_next = t
            .income_month
            .as_ref()
            .map(|m| m.as_str() > &t.date[..7])
            .or_else(|| {
                (t.transfer.is_none()
                    && t.category.is_none()
                    && t.splits.is_empty()
                    && t.amount > 0)
                    .then_some(false)
            });
        draft.memo = t.memo.clone();
        if t.amount < 0 {
            draft.outflow = plain(-t.amount, currency);
        } else {
            draft.inflow = plain(t.amount, currency);
        }
        draft.split_details = t
            .splits
            .iter()
            .map(|p| SplitDetails::from_part(p, &t.date))
            .collect();
        draft.splits = t
            .splits
            .iter()
            .map(|s| {
                (
                    s.category.clone().unwrap_or_default(),
                    plain(s.amount, currency),
                )
            })
            .collect();
        if let Some(link) = &t.transfer {
            let peer = b
                .scheduled
                .iter()
                .find(|s| s.id != schedule.id && s.transaction.transfer_links().any(|x| x == link))
                .ok_or("Scheduled transfer partner not found")?;
            draft.transfer_to = Some(peer.transaction.account.clone());
        }
        Ok(draft)
    }

    pub fn set_split_category(&mut self, index: usize, category: String) {
        if let Some(part) = self.splits.get_mut(index) {
            part.0 = category;
        }
        if let Some(details) = self.split_details.get_mut(index) {
            details.income_next = None;
        }
    }

    pub fn autocomplete(&mut self, b: &Budget, prefs: &crate::preferences::Preferences) {
        if self.id.is_some() || self.transfer_to.is_some() {
            return;
        }
        let account_changed = self.completed_account != self.account;
        if !self.complete_payee && !account_changed {
            return;
        }
        if self
            .automatic_amount
            .as_ref()
            .is_some_and(|v| v == &(self.outflow.clone(), self.inflow.clone()))
        {
            self.outflow.clear();
            self.inflow.clear();
        }
        self.automatic_amount = None;
        self.completed_account = self.account.clone();
        let accepted = std::mem::take(&mut self.complete_payee);
        if accepted {
            self.payee = b.imported_payee(self.payee.trim());
        }
        let name = &self.payee;
        let rule = b.payee_rules.get(name);
        if rule.is_some_and(|r| !r.listed) {
            return;
        }
        let history = || {
            b.transactions
                .iter()
                .filter(|t| t.payee == *name && t.transfer.is_none())
        };
        let last = history().max_by_key(|t| &t.date);
        if accepted
            && prefs.autocomplete_category
            && self.splits.is_empty()
            && (last.is_some() || rule.is_some_and(|r| r.category.is_some()))
        {
            self.category = rule
                .and_then(|r| r.category.clone())
                .or_else(|| last.and_then(|t| t.category.clone()))
                .unwrap_or_default();
            self.income_next = None;
        }
        if accepted
            && prefs.autocomplete_memo
            && let Some(last) = last
        {
            self.memo = last.memo.clone();
        }
        if prefs.autocomplete_amount
            && self.splits.is_empty()
            && let Some(last) = history()
                .filter(|t| t.account == self.account)
                .max_by_key(|t| &t.date)
            && let Ok(account) = b.account(&self.account)
            && (accepted || (account_changed && self.outflow.is_empty() && self.inflow.is_empty()))
        {
            self.outflow = if last.amount < 0 {
                plain(-last.amount, &account.currency)
            } else {
                String::new()
            };
            self.inflow = if last.amount > 0 {
                plain(last.amount, &account.currency)
            } else {
                String::new()
            };
            self.automatic_amount = Some((self.outflow.clone(), self.inflow.clone()));
        }
    }
    pub fn build(&self, b: &Budget) -> Result<Transaction> {
        self.build_inner(b, false)
    }
    pub fn build_scheduled(&self, b: &Budget) -> Result<Transaction> {
        self.build_inner(b, true)
    }
    pub fn signed_amount(&self, b: &Budget) -> Result<i64> {
        let a = b.account(&self.account)?;
        let parse = |text: &str| {
            if text.trim().is_empty() {
                Ok(0)
            } else {
                amount(text, &a.currency)
            }
        };
        let out = parse(&self.outflow)?;
        let incoming = parse(&self.inflow)?;
        if out < 0
            || incoming < 0
            || (out > 0 && incoming > 0)
            || (self.outflow.trim().is_empty() && self.inflow.trim().is_empty())
        {
            return Err("Enter a nonnegative amount in either Outflow or Inflow.".into());
        }
        Ok(incoming - out)
    }
    pub(super) fn build_inner(&self, b: &Budget, scheduled: bool) -> Result<Transaction> {
        let a = b.account(&self.account)?;
        let value = self.signed_amount(b)?;
        let splits = self
            .splits
            .iter()
            .enumerate()
            .map(|(i, (c, v))| {
                let details = self.split_details.get(i).cloned().unwrap_or_default();
                let transfer = details.transfer_to.map(|account| {
                    let link = details
                        .transfer
                        .filter(|old| old.account == account)
                        .map(|old| old.link)
                        .unwrap_or_else(id);
                    SplitTransfer { link, account }
                });
                Ok(Split {
                    category: (!c.is_empty()).then(|| c.clone()),
                    amount: amount(v, &a.currency)?,
                    income_month: if c.is_empty() && valid_date(&self.date) {
                        details
                            .income_next
                            .map(|next| month_shift(&self.date[..7], i32::from(next)))
                    } else {
                        None
                    },
                    payee: (!details.payee.is_empty() && transfer.is_none())
                        .then_some(details.payee),
                    memo: details.memo,
                    transfer,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let t = Transaction {
            check_number: self
                .id
                .as_ref()
                .and_then(|id| {
                    b.transactions.iter().find(|t| &t.id == id).or_else(|| {
                        b.scheduled
                            .iter()
                            .find(|s| &s.id == id)
                            .map(|s| &s.transaction)
                    })
                })
                .and_then(|t| t.check_number.clone()),
            flag: self
                .id
                .as_ref()
                .and_then(|id| {
                    b.transactions.iter().find(|t| &t.id == id).or_else(|| {
                        b.scheduled
                            .iter()
                            .find(|s| &s.id == id)
                            .map(|s| &s.transaction)
                    })
                })
                .and_then(|t| t.flag),
            id: self.id.clone().unwrap_or_else(id),
            account: self.account.clone(),
            date: self.date.clone(),
            payee: self.payee.trim().into(),
            category: if self.category.is_empty() || !splits.is_empty() {
                None
            } else {
                Some(self.category.clone())
            },
            memo: self.memo.clone(),
            amount: value,
            base_amount: 0,
            snapshot: None,
            income_month: if self.category.is_empty() && splits.is_empty() && valid_date(&self.date)
            {
                self.income_next
                    .map(|next| month_shift(&self.date[..7], i32::from(next)))
            } else {
                None
            },
            cleared: self.cleared,
            reconciled: false,
            transfer: None,
            splits,
        };
        if scheduled {
            b.prepare_scheduled_transaction(t)
        } else {
            b.prepare_transaction(t)
        }
    }
}
