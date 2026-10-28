//! Split semantics shared by import, editing, reports and persistence.
use crate::model::*;
use std::collections::{BTreeMap, HashSet};

impl Transaction {
    pub fn transfer_links(&self) -> impl Iterator<Item = &str> {
        self.transfer.as_deref().into_iter().chain(
            self.splits
                .iter()
                .filter_map(|s| s.transfer.as_ref().map(|t| t.link.as_str())),
        )
    }
    pub fn has_transfers(&self) -> bool {
        self.transfer_links().next().is_some()
    }
}

/// Entire logical operation, including peers attached to individual parts.
pub fn connected<'a>(
    rows: impl IntoIterator<Item = &'a Transaction>,
    tid: &str,
) -> HashSet<String> {
    let rows: Vec<_> = rows.into_iter().collect();
    let mut ids = HashSet::from([tid.to_owned()]);
    loop {
        let links: HashSet<_> = rows
            .iter()
            .filter(|t| ids.contains(&t.id))
            .flat_map(|t| t.transfer_links())
            .collect();
        let before = ids.len();
        ids.extend(
            rows.iter()
                .filter(|t| t.transfer_links().any(|link| links.contains(link)))
                .map(|t| t.id.clone()),
        );
        if ids.len() == before {
            return ids;
        }
    }
}

impl Budget {
    pub fn split_category_name(&self, part: &Split) -> String {
        if let Some(month) = &part.income_month {
            return crate::income::income_label(month);
        }
        part.category
            .as_ref()
            .and_then(|id| self.categories.iter().find(|c| &c.id == id))
            .map(|c| format!("{}: {}", c.group, c.name))
            .unwrap_or_else(|| "Uncategorized".into())
    }
    pub fn split_payee_name(&self, t: &Transaction, part: &Split) -> String {
        if let Some(transfer) = &part.transfer {
            return self
                .account(&transfer.account)
                .map(|a| format!("Transfer : {}", a.name))
                .unwrap_or_else(|_| "Transfer".into());
        }
        part.payee.as_ref().unwrap_or(&t.payee).clone()
    }

    pub fn validate_splits(&self, t: &Transaction) -> Result<()> {
        if t.splits.is_empty() {
            return Ok(());
        }
        if t.income_month.is_some()
            || t.transfer.is_some()
            || t.splits.iter().map(|s| s.amount as i128).sum::<i128>() != t.amount as i128
        {
            return Err(
                "Split parts must total the transaction, without a parent category or transfer"
                    .into(),
            );
        }
        for part in &t.splits {
            if part.amount.unsigned_abs() > 100_000_000_000_000
                || part
                    .category
                    .as_ref()
                    .is_some_and(|c| !self.categories.iter().any(|x| x.id == *c))
            {
                return Err("Invalid split category or amount".into());
            }
            if let Some(month) = &part.income_month
                && (!valid_date(&t.date)
                    || !valid_month(month)
                    || part.category.is_some()
                    || (month != &t.date[..7] && *month != month_shift(&t.date[..7], 1)))
            {
                return Err("Split income must use this or next month without a category".into());
            }
            if let Some(transfer) = &part.transfer {
                let source = self.account(&t.account)?;
                let target = self.account(&transfer.account)?;
                if transfer.link.is_empty()
                    || source.id == target.id
                    || part.amount == 0
                    || source.currency != target.currency
                    || (source.on_budget == target.on_budget
                        && (part.category.is_some() || part.income_month.is_some()))
                {
                    return Err("Invalid split transfer accounts, amount or category".into());
                }
            }
        }
        Ok(())
    }

    pub fn split_has_category_flow(&self, t: &Transaction, part: &Split) -> bool {
        part.transfer.is_none()
            || (self.account(&t.account).is_ok_and(|a| a.on_budget)
                && self
                    .account(&part.transfer.as_ref().unwrap().account)
                    .is_ok_and(|a| !a.on_budget))
    }

    /// Validate ordinary and split endpoints together, so no dangling or reused
    /// links can be persisted, even when two splits share a transfer.
    pub fn validate_transfer_rows<'a>(
        &self,
        rows: impl IntoIterator<Item = &'a Transaction>,
    ) -> Result<()> {
        struct End<'a> {
            t: &'a Transaction,
            part: Option<&'a Split>,
        }
        impl End<'_> {
            fn amount(&self) -> i64 {
                self.part.map_or(self.t.amount, |s| s.amount)
            }
            fn category(&self) -> bool {
                self.part.map_or(
                    self.t.category.is_some() || self.t.income_month.is_some(),
                    |s| s.category.is_some() || s.income_month.is_some(),
                )
            }
        }
        let mut links = BTreeMap::<&str, Vec<End<'_>>>::new();
        for t in rows {
            if let Some(link) = &t.transfer {
                links.entry(link).or_default().push(End { t, part: None });
            }
            for part in &t.splits {
                if let Some(transfer) = &part.transfer {
                    links.entry(&transfer.link).or_default().push(End {
                        t,
                        part: Some(part),
                    });
                }
            }
        }
        for ends in links.values() {
            if ends.len() != 2 {
                return Err("Transfer must have exactly two endpoints".into());
            }
            let (a, b) = (&ends[0], &ends[1]);
            let (aa, ba) = (self.account(&a.t.account)?, self.account(&b.t.account)?);
            if a.t.id == b.t.id
                || aa.id == ba.id
                || a.t.date != b.t.date
                || a.amount() == 0
                || a.amount().signum() == b.amount().signum()
                || (aa.currency == ba.currency && a.amount() != -b.amount())
                || (aa.on_budget == ba.on_budget && (a.category() || b.category()))
            {
                return Err("Invalid transfer pair".into());
            }
            for (end, other) in [(a, b), (b, a)] {
                if let Some(part) = end.part
                    && part.transfer.as_ref().unwrap().account != other.t.account
                {
                    return Err("Split transfer counterpart account differs".into());
                }
            }
        }
        Ok(())
    }

    /// Remove the opposite endpoint while preserving unrelated parts of a split.
    pub(crate) fn remove_transfer_peer(
        &mut self,
        link: &str,
        except: &str,
        scheduled: bool,
    ) -> Result<()> {
        let index = self
            .transactions
            .iter()
            .position(|p| p.id != except && p.transfer_links().any(|x| x == link))
            .ok_or("Missing transfer counterpart")?;
        let mut peer = self.transactions[index].clone();
        if peer.reconciled {
            return Err("Reconciled transfer counterpart cannot be removed".into());
        }
        if peer.transfer.as_deref() == Some(link) {
            self.transactions.remove(index);
        } else {
            let part = peer
                .splits
                .iter()
                .position(|p| p.transfer.as_ref().is_some_and(|x| x.link == link))
                .ok_or("Missing transfer part")?;
            let amount = peer.splits.remove(part).amount;
            peer.amount = peer
                .amount
                .checked_sub(amount)
                .ok_or("Split amount overflow")?;
            if peer.splits.is_empty() {
                self.transactions.remove(index);
            } else {
                if !scheduled {
                    (peer.base_amount, peer.snapshot) = self.transaction_value(&peer)?;
                }
                self.transactions[index] = peer;
            }
        }
        self.register_meta
            .retain(|id, _| self.transactions.iter().any(|t| &t.id == id));
        Ok(())
    }

    pub(crate) fn sync_component_date(
        &mut self,
        tid: &str,
        date: &str,
        scheduled: bool,
    ) -> Result<()> {
        let ids = connected(&self.transactions, tid);
        for i in 0..self.transactions.len() {
            if !ids.contains(&self.transactions[i].id) || self.transactions[i].date == date {
                continue;
            }
            let mut t = self.transactions[i].clone();
            if t.reconciled {
                return Err("Reconciled transfer counterpart cannot be changed".into());
            }
            t.set_date(date.into());
            if !scheduled {
                (t.base_amount, t.snapshot) = self.transaction_value(&t)?;
            }
            self.transactions[i] = t;
        }
        Ok(())
    }

    /// Update or create counterparts of an edited split. A caller
    /// applies this to a disposable document and validates before committing.
    pub(crate) fn sync_split_peers(
        &mut self,
        old: Option<&Transaction>,
        t: &Transaction,
    ) -> Result<()> {
        self.sync_split_peers_inner(old, t, false)
    }
    fn sync_split_peers_inner(
        &mut self,
        old: Option<&Transaction>,
        t: &Transaction,
        scheduled: bool,
    ) -> Result<()> {
        let old_links: HashSet<_> = old
            .into_iter()
            .flat_map(|t| t.splits.iter())
            .filter_map(|s| s.transfer.as_ref().map(|x| x.link.clone()))
            .collect();
        let new_links: HashSet<_> = t
            .splits
            .iter()
            .filter_map(|s| s.transfer.as_ref().map(|x| x.link.clone()))
            .collect();
        for link in old_links.difference(&new_links) {
            self.remove_transfer_peer(link, &t.id, scheduled)?;
        }
        for part in &t.splits {
            let Some(transfer) = &part.transfer else {
                continue;
            };
            let index = self
                .transactions
                .iter()
                .position(|p| p.id != t.id && p.transfer_links().any(|x| x == transfer.link));
            if let Some(index) = index {
                let peer = &self.transactions[index];
                if peer.transfer.as_deref() != Some(&transfer.link) {
                    let mut updated = peer.clone();
                    let opposite = updated
                        .splits
                        .iter_mut()
                        .find(|p| p.transfer.as_ref().is_some_and(|x| x.link == transfer.link))
                        .ok_or("Missing transfer part")?;
                    let delta = (-part.amount)
                        .checked_sub(opposite.amount)
                        .ok_or("Split amount overflow")?;
                    opposite.amount = -part.amount;
                    opposite.transfer.as_mut().unwrap().account = t.account.clone();
                    let prior = old.and_then(|t| {
                        t.splits
                            .iter()
                            .find(|p| p.transfer.as_ref().is_some_and(|x| x.link == transfer.link))
                    });
                    if prior.is_none_or(|p| p.memo != part.memo) {
                        opposite.memo = part.memo.clone();
                    }
                    updated.amount = updated
                        .amount
                        .checked_add(delta)
                        .ok_or("Split amount overflow")?;
                    updated.set_date(t.date.clone());
                    if peer.account != transfer.account {
                        return Err("Split counterpart account differs".into());
                    }
                    if peer.reconciled
                        && serde_json::to_value(peer).ok() != serde_json::to_value(&updated).ok()
                    {
                        return Err("Reconciled split counterpart cannot be changed".into());
                    }
                    if !scheduled {
                        (updated.base_amount, updated.snapshot) =
                            self.transaction_value(&updated)?;
                    }
                    self.transactions[index] = updated;
                    continue;
                }
                if peer.reconciled {
                    if peer.account == transfer.account
                        && peer.amount == -part.amount
                        && peer.date == t.date
                        && old.is_none_or(|old| old.account == t.account)
                    {
                        continue;
                    }
                    return Err("Reconciled transfer counterpart cannot be changed".into());
                }
            }
            let mut peer = index
                .map(|i| self.transactions[i].clone())
                .unwrap_or_else(|| Transaction {
                    id: id(),
                    account: transfer.account.clone(),
                    date: t.date.clone(),
                    payee: String::new(),
                    memo: String::new(),
                    amount: 0,
                    base_amount: 0,
                    snapshot: None,
                    category: None,
                    cleared: false,
                    check_number: None,
                    flag: None,
                    income_month: None,
                    reconciled: false,
                    transfer: Some(transfer.link.clone()),
                    splits: vec![],
                });
            if self.account(&transfer.account)?.closed {
                return Err("Transfer account is closed".into());
            }
            peer.account = transfer.account.clone();
            peer.set_date(t.date.clone());
            peer.amount = -part.amount;
            peer.payee = format!("Transfer : {}", self.account(&t.account)?.name);
            let prior = old.and_then(|t| {
                t.splits
                    .iter()
                    .find(|p| p.transfer.as_ref().is_some_and(|x| x.link == transfer.link))
            });
            if prior.is_none_or(|p| p.memo != part.memo) {
                peer.memo = part.memo.clone();
            }
            if scheduled {
                peer.base_amount = 0;
                peer.snapshot = None;
            } else {
                (peer.base_amount, peer.snapshot) = self.transaction_value(&peer)?;
            }
            if let Some(index) = index {
                self.transactions[index] = peer;
            } else {
                self.transactions.push(peer);
            }
        }
        Ok(())
    }
}

impl Budget {
    pub fn add_split_schedule(
        &mut self,
        t: Transaction,
        repeat: crate::register::Repeat,
    ) -> Result<()> {
        let t = self.prepare_scheduled_transaction(t)?;
        let mut templates = self.clone();
        templates.transactions.clear();
        templates.sync_split_peers_inner(None, &t, true)?;
        templates.transactions.push(t.clone());
        let mut next = self.clone();
        for transaction in templates.transactions {
            next.scheduled.push(Scheduled {
                id: id(),
                next_date: t.date.clone(),
                transaction,
                monthly: repeat == crate::register::Repeat::Monthly,
                repeat: Some(repeat),
                anchor_day: None,
            });
        }
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }

    pub(crate) fn edit_split_schedule(
        &mut self,
        sid: &str,
        mut t: Transaction,
        repeat: crate::register::Repeat,
        anchor: Option<u32>,
        peer_account: Option<&str>,
    ) -> Result<()> {
        let ids = self.schedule_ids(sid)?;
        let original = self.scheduled.iter().find(|s| s.id == sid).unwrap();
        if t.transfer != original.transaction.transfer {
            return Err("Cannot change schedule type".into());
        }
        let mut templates = self.clone();
        templates.transactions = self
            .scheduled
            .iter()
            .filter(|s| ids.contains(&s.id))
            .map(|s| s.transaction.clone())
            .collect();
        t.id = original.transaction.id.clone();
        if let Some(link) = &t.transfer {
            let pi = templates
                .transactions
                .iter()
                .position(|p| {
                    p.splits
                        .iter()
                        .any(|s| s.transfer.as_ref().is_some_and(|x| &x.link == link))
                })
                .ok_or("Missing split schedule counterpart")?;
            if peer_account.is_some_and(|account| account != templates.transactions[pi].account) {
                return Err("Change the source account in the split schedule editor".into());
            }
            let parent = &mut templates.transactions[pi];
            parent.set_date(t.date.clone());
            let part = parent
                .splits
                .iter_mut()
                .find(|s| s.transfer.as_ref().is_some_and(|x| &x.link == link))
                .unwrap();
            let delta = (-t.amount)
                .checked_sub(part.amount)
                .ok_or("Split amount overflow")?;
            part.amount = -t.amount;
            part.transfer.as_mut().unwrap().account = t.account.clone();
            if t.memo != original.transaction.memo {
                part.memo = t.memo.clone();
            }
            parent.amount = parent
                .amount
                .checked_add(delta)
                .ok_or("Split amount overflow")?;
            let parent = parent.clone();
            templates.validate_splits(&parent)?;
            let link = t.transfer.take();
            t = templates.prepare_scheduled_transaction(t)?;
            t.transfer = link;
            *templates
                .transactions
                .iter_mut()
                .find(|p| p.id == t.id)
                .unwrap() = t.clone();
        } else {
            t = templates.prepare_scheduled_transaction(t)?;
            templates.sync_split_peers_inner(Some(&original.transaction), &t, true)?;
            *templates
                .transactions
                .iter_mut()
                .find(|p| p.id == t.id)
                .unwrap() = t.clone();
        }
        let day =
            chrono::NaiveDate::parse_from_str(&t.date, "%Y-%m-%d").map_err(|_| "Invalid date")?;
        use chrono::Datelike;
        let anchor = if t.date == original.next_date {
            anchor.or(original.anchor_day)
        } else {
            Some(day.day())
        };
        let mut next = self.clone();
        next.scheduled.retain(|s| !ids.contains(&s.id));
        let component = connected(&templates.transactions, &t.id);
        for mut transaction in templates.transactions {
            if !component.contains(&transaction.id) {
                if let Some(previous) = self
                    .scheduled
                    .iter()
                    .find(|s| s.transaction.id == transaction.id)
                {
                    let mut detached = previous.clone();
                    detached.transaction = transaction;
                    next.scheduled.push(detached);
                }
                continue;
            }
            transaction.set_date(t.date.clone());
            let previous = self
                .scheduled
                .iter()
                .find(|s| s.transaction.id == transaction.id);
            next.scheduled.push(Scheduled {
                id: previous.map(|s| s.id.clone()).unwrap_or_else(id),
                transaction,
                next_date: t.date.clone(),
                monthly: repeat == crate::register::Repeat::Monthly,
                repeat: Some(repeat),
                anchor_day: anchor,
            });
        }
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }
}

impl Budget {
    pub fn delete_scheduled(&mut self, ids: &HashSet<String>) -> Result<()> {
        let mut next = self.clone();
        let mut templates = self.clone();
        templates.transactions = self
            .scheduled
            .iter()
            .map(|s| s.transaction.clone())
            .collect();
        templates.register_meta.clear();
        for selected in self.scheduled.iter().filter(|s| ids.contains(&s.id)) {
            let Some(t) = templates
                .transactions
                .iter()
                .find(|t| t.id == selected.transaction.id)
                .cloned()
            else {
                continue;
            };
            for link in t.transfer_links() {
                templates.remove_transfer_peer(link, &t.id, true)?;
            }
            templates.transactions.retain(|p| p.id != t.id);
        }
        next.scheduled.retain_mut(|s| {
            if let Some(t) = templates
                .transactions
                .iter()
                .find(|t| t.id == s.transaction.id)
            {
                s.transaction = t.clone();
                true
            } else {
                false
            }
        });
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }
}

impl Budget {
    pub fn rename_payee(&mut self, old: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a payee name".into());
        }
        let rename = |t: &mut Transaction| {
            if t.transfer.is_none() && t.payee == old {
                t.payee = name.into();
            }
            for part in &mut t.splits {
                if part.transfer.is_none() && part.payee.as_deref() == Some(old) {
                    part.payee = Some(name.into());
                }
            }
        };
        for t in &mut self.transactions {
            rename(t);
        }
        for s in &mut self.scheduled {
            rename(&mut s.transaction);
        }
        for state in self.register_meta.values_mut() {
            if let Some(pair) = &mut state.matched {
                rename(&mut pair.manual);
                rename(&mut pair.imported);
            }
        }
        if let Some(rule) = self.payee_rules.remove(old) {
            self.payee_rules.entry(name.into()).or_insert(rule);
        }
        Ok(())
    }
}
