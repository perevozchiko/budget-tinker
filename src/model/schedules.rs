use super::*;

impl Budget {
    pub fn post_scheduled(&mut self, sid: &str) -> Result<()> {
        let mut next = self.clone();
        next.post_schedule_inner(sid)?;
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }

    /// Skip one logical occurrence, including both sides of a transfer.
    pub fn skip_scheduled(&mut self, sid: &str) -> Result<()> {
        let mut next = self.clone();
        crate::store::validate(&next)?;
        let ids = next.schedule_ids(sid)?;
        for id in ids {
            let s = next.scheduled.iter().find(|s| s.id == id).unwrap().clone();
            let anchor = s.anchor_day.unwrap_or(
                NaiveDate::parse_from_str(&s.next_date, "%Y-%m-%d")
                    .map_err(|_| "Invalid schedule date")?
                    .day(),
            );
            if let Some(date) = s.frequency().next(&s.next_date, anchor)? {
                let row = next.scheduled.iter_mut().find(|s| s.id == id).unwrap();
                row.next_date = date.clone();
                row.transaction.set_date(date);
                row.anchor_day = Some(anchor);
            } else {
                next.scheduled.retain(|s| s.id != id);
            }
        }
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }

    pub fn schedule_ids(&self, sid: &str) -> Result<Vec<String>> {
        let selected = self
            .scheduled
            .iter()
            .find(|s| s.id == sid)
            .ok_or("Schedule not found")?;
        let ids = crate::splits::connected(
            self.scheduled.iter().map(|s| &s.transaction),
            &selected.transaction.id,
        );
        Ok(self
            .scheduled
            .iter()
            .filter(|s| ids.contains(&s.transaction.id))
            .map(|s| s.id.clone())
            .collect())
    }

    /// Post today, advancing from the nominal due date, as one atomic change.
    pub fn enter_scheduled_now(&mut self, sid: &str) -> Result<()> {
        let mut next = self.clone();
        crate::store::validate(&next)?;
        next.post_schedule_inner_at(sid, Some(&today()))?;
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }

    /// Move the selected side; the transfer's other account remains the destination.
    /// Amounts retain their currency, so moving between currencies is not implicit.
    pub fn move_scheduled_account(&mut self, sid: &str, account: &str) -> Result<()> {
        let selected = self
            .scheduled
            .iter()
            .find(|s| s.id == sid)
            .ok_or("Schedule not found")?
            .clone();
        let target = self.account(account)?;
        if target.closed || target.currency != self.account(&selected.transaction.account)?.currency
        {
            return Err("Choose an open account with the same currency".into());
        }
        let mut t = selected.transaction.clone();
        t.account = account.into();
        t.set_date(selected.next_date.clone());
        self.edit_scheduled(sid, t, selected.frequency(), selected.anchor_day)
    }

    /// Edit a logical schedule without changing any already posted records.
    pub fn edit_scheduled(
        &mut self,
        sid: &str,
        transaction: Transaction,
        repeat: crate::register::Repeat,
        anchor: Option<u32>,
    ) -> Result<()> {
        self.edit_schedule(sid, transaction, repeat, anchor, None)
    }

    pub fn edit_scheduled_transfer(
        &mut self,
        sid: &str,
        transaction: Transaction,
        peer_account: &str,
        repeat: crate::register::Repeat,
    ) -> Result<()> {
        self.edit_schedule(sid, transaction, repeat, None, Some(peer_account))
    }

    pub(super) fn edit_schedule(
        &mut self,
        sid: &str,
        mut transaction: Transaction,
        repeat: crate::register::Repeat,
        anchor: Option<u32>,
        peer_account: Option<&str>,
    ) -> Result<()> {
        crate::store::validate(self)?;
        let ids = self.schedule_ids(sid)?;
        let original = self.scheduled.iter().find(|s| s.id == sid).unwrap();
        if self.account(&original.transaction.account)?.currency
            != self.account(&transaction.account)?.currency
        {
            return Err("Choose an account with the same currency".into());
        }
        if self.scheduled.iter().any(|s| {
            ids.contains(&s.id) && s.transaction.splits.iter().any(|p| p.transfer.is_some())
        }) {
            return self.edit_split_schedule(sid, transaction, repeat, anchor, peer_account);
        }
        // Validate the template with the normal scheduled-transaction rules.
        let link = original.transaction.transfer.clone();
        if transaction.transfer != link {
            return Err("Cannot change the type of a scheduled transaction".into());
        }
        transaction.transfer = None;
        transaction = self.prepare_scheduled_transaction(transaction)?;
        transaction.transfer = link.clone();
        transaction.id = original.transaction.id.clone();
        let anchor = if transaction.date == original.next_date {
            anchor.or(original.anchor_day)
        } else {
            Some(
                NaiveDate::parse_from_str(&transaction.date, "%Y-%m-%d")
                    .map_err(|_| "Invalid schedule date")?
                    .day(),
            )
        };
        let mut next = self.clone();
        if link.is_some() {
            if ids.len() != 2 || !transaction.splits.is_empty() {
                return Err("Invalid scheduled transfer".into());
            }
            let mut peer = next
                .scheduled
                .iter()
                .find(|s| s.id != sid && ids.contains(&s.id))
                .unwrap()
                .clone();
            if let Some(account) = peer_account {
                if next.account(account)?.currency
                    != next.account(&peer.transaction.account)?.currency
                {
                    return Err("Choose an account with the same currency".into());
                }
                peer.transaction.account = account.into();
            }
            let a = next.account(&transaction.account)?;
            let other = next.account(&peer.transaction.account)?;
            if a.id == other.id || other.closed || a.currency != other.currency {
                return Err("Choose two different open accounts with the same currency".into());
            }
            let mut partner = peer.transaction.clone();
            partner.set_date(transaction.date.clone());
            partner.amount = -transaction.amount;
            partner.memo = transaction.memo.clone();
            partner.payee = format!("Transfer : {}", a.name);
            partner.base_amount = 0;
            partner.snapshot = None;
            partner.cleared = false;
            transaction.payee = format!("Transfer : {}", other.name);
            let row = next.scheduled.iter_mut().find(|s| s.id == peer.id).unwrap();
            row.transaction = partner;
        }
        for id in ids {
            let row = next.scheduled.iter_mut().find(|s| s.id == id).unwrap();
            row.next_date = transaction.date.clone();
            row.repeat = Some(repeat);
            row.monthly = repeat == crate::register::Repeat::Monthly;
            row.anchor_day = anchor;
            if id == sid {
                row.transaction = transaction.clone();
            }
        }
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }
}
