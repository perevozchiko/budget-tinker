//! Atomic register commands. UI availability is derived from the same validation as execution.
use crate::model::*;
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImportState {
    pub pending: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default)]
    pub matched: Option<MatchBackup>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchBackup {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_state: Option<Box<ImportState>>,
    pub manual: Transaction,
    pub imported: Transaction,
    pub pending: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Repeat {
    #[default]
    Never,
    Daily,
    Weekly,
    EveryOtherWeek,
    TwiceAMonth,
    Every4Weeks,
    Monthly,
    EveryOtherMonth,
    Every3Months,
    Every4Months,
    TwiceAYear,
    Yearly,
    EveryOtherYear,
}
impl Repeat {
    pub const ALL: [Self; 13] = [
        Self::Never,
        Self::Daily,
        Self::Weekly,
        Self::EveryOtherWeek,
        Self::TwiceAMonth,
        Self::Every4Weeks,
        Self::Monthly,
        Self::EveryOtherMonth,
        Self::Every3Months,
        Self::Every4Months,
        Self::TwiceAYear,
        Self::Yearly,
        Self::EveryOtherYear,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Never => "Never",
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::EveryOtherWeek => "Every other week",
            Self::TwiceAMonth => "Twice a month",
            Self::Every4Weeks => "Every 4 weeks",
            Self::Monthly => "Monthly",
            Self::EveryOtherMonth => "Every other month",
            Self::Every3Months => "Every 3 months",
            Self::Every4Months => "Every 4 months",
            Self::TwiceAYear => "Twice a year",
            Self::Yearly => "Yearly",
            Self::EveryOtherYear => "Every other year",
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Never => "",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::EveryOtherWeek => "biweekly",
            Self::TwiceAMonth => "twice_monthly",
            Self::Every4Weeks => "four_weeks",
            Self::Monthly => "monthly",
            Self::EveryOtherMonth => "two_months",
            Self::Every3Months => "quarterly",
            Self::Every4Months => "four_months",
            Self::TwiceAYear => "six_months",
            Self::Yearly => "yearly",
            Self::EveryOtherYear => "two_years",
        }
    }
    pub fn from_code(code: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|r| r.code() == code)
            .unwrap_or_default()
    }
    pub fn next(self, date: &str, anchor: u32) -> Result<Option<String>> {
        let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| "Invalid schedule date")?;
        let days = match self {
            Self::Never => return Ok(None),
            Self::Daily => 1,
            Self::Weekly => 7,
            Self::EveryOtherWeek => 14,
            Self::Every4Weeks => 28,
            _ => 0,
        };
        if days > 0 {
            return d
                .checked_add_signed(Duration::days(days))
                .map(|d| Some(d.to_string()))
                .ok_or_else(|| "Schedule date is out of range".into());
        }
        let months = match self {
            Self::EveryOtherMonth => 2,
            Self::Every3Months => 3,
            Self::Every4Months => 4,
            Self::TwiceAYear => 6,
            Self::Yearly => 12,
            Self::EveryOtherYear => 24,
            _ => 1,
        };
        let make = |month: &str, day: u32| -> Result<String> {
            if !valid_month(month) {
                return Err("Schedule date is out of range".into());
            }
            let last = NaiveDate::parse_from_str(&Budget::month_end(month), "%Y-%m-%d")
                .map_err(|_| "Schedule date is out of range")?
                .day();
            Ok(format!("{month}-{:02}", day.min(last)))
        };
        if self == Self::TwiceAMonth {
            // Preserve the two days of month, including after a short February.
            let first = if anchor > 15 { anchor - 15 } else { anchor };
            let second = if anchor > 15 { anchor } else { anchor + 15 };
            if d.day() < second && d.day() <= first {
                return make(&date[..7], second).map(Some);
            }
            return make(&month_shift(&date[..7], 1), first).map(Some);
        }
        make(&month_shift(&date[..7], months), anchor).map(Some)
    }
}
impl Scheduled {
    pub fn frequency(&self) -> Repeat {
        self.repeat.unwrap_or(if self.monthly {
            Repeat::Monthly
        } else {
            Repeat::Never
        })
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Flag(Option<crate::model::TransactionFlag>),
    Cleared(bool),
    Approve,
    Reject,
    Delete,
    Categorize {
        category: Option<String>,
        replace_splits: bool,
    },
    IncomeMonth {
        next_month: bool,
        replace_splits: bool,
    },
    MoveAccount(String),
    Match,
    Unmatch,
    RenamePayees(String),
    Schedule {
        date: String,
        repeat: Repeat,
        move_original: bool,
    },
}
impl Budget {
    pub fn pending_import(&self, tid: &str) -> bool {
        self.register_meta.get(tid).is_some_and(|m| m.pending)
    }
    pub fn imported_id_exists(&self, tid: &str) -> bool {
        self.transactions.iter().any(|t| t.id == tid)
            || self.scheduled.iter().any(|s| s.transaction.id == tid)
            || self.register_meta.values().any(|m| {
                m.source_id.as_deref() == Some(tid)
                    || m.matched
                        .as_ref()
                        .and_then(|p| p.manual_state.as_ref())
                        .is_some_and(|s| s.source_id.as_deref() == Some(tid))
                    || m.matched
                        .as_ref()
                        .is_some_and(|p| p.imported.id == tid || p.manual.id == tid)
            })
    }
    pub fn import_history(&self) -> impl Iterator<Item = &Transaction> {
        self.transactions
            .iter()
            .filter(|t| {
                self.register_meta
                    .get(&t.id)
                    .map_or_else(|| t.id.starts_with("bank:"), |m| m.matched.is_none())
            })
            .chain(
                self.register_meta
                    .values()
                    .filter_map(|m| m.matched.as_ref())
                    .flat_map(|p| {
                        std::iter::once(&p.imported)
                            .chain(p.manual_state.as_ref().map(|_| &p.manual))
                    }),
            )
    }
    /// Merge into the existing identity so transfer links and manual dates stay intact.
    /// Callers apply this on a disposable budget and validate before committing.
    pub(crate) fn match_import(&mut self, imported_id: &str, manual_id: &str) -> Result<()> {
        let imported = self
            .transactions
            .iter()
            .find(|t| t.id == imported_id)
            .cloned()
            .ok_or("Missing imported transaction")?;
        let manual = self
            .transactions
            .iter()
            .find(|t| t.id == manual_id)
            .cloned()
            .ok_or("Missing matching transaction")?;
        let state = self
            .register_meta
            .get(imported_id)
            .cloned()
            .ok_or("Select an imported transaction")?;
        let manual_state = self.register_meta.get(manual_id).cloned();
        if imported_id == manual_id
            || imported.account != manual.account
            || imported.amount != manual.amount
            || imported.has_transfers()
            || imported.reconciled
            || state.matched.is_some()
            || manual_state.as_ref().is_some_and(|m| m.matched.is_some())
        {
            return Err(
                "Match requires equal amounts in the same account and unmatched originals".into(),
            );
        }
        let result = self
            .transactions
            .iter_mut()
            .find(|t| t.id == manual_id)
            .unwrap();
        result.cleared = true;
        self.transactions.retain(|t| t.id != imported_id);
        self.register_meta.remove(imported_id);
        self.register_meta.insert(
            manual_id.into(),
            ImportState {
                source_id: state.source_id,
                pending: state.pending && !manual.reconciled,
                matched: Some(MatchBackup {
                    manual_state: manual_state.map(Box::new),
                    manual,
                    imported,
                    pending: state.pending,
                }),
            },
        );
        Ok(())
    }
    pub fn register_command(&mut self, ids: &HashSet<String>, command: Command) -> Result<()> {
        let mut next = self.clone();
        next.register_command_inner(ids, command)?;
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }
    pub fn can_register_command(&self, ids: &HashSet<String>, command: Command) -> bool {
        // This intentionally shares the actual command validation; disabled items cannot diverge.
        let mut next = self.clone();
        next.register_command_inner(ids, command).is_ok() && crate::store::validate(&next).is_ok()
    }
    fn selected_rows(&self, ids: &HashSet<String>) -> Result<Vec<Transaction>> {
        let rows: Vec<_> = self
            .transactions
            .iter()
            .filter(|t| ids.contains(&t.id))
            .cloned()
            .collect();
        if rows.is_empty() || rows.len() != ids.len() {
            return Err("Select existing transactions".into());
        }
        Ok(rows)
    }
    fn register_command_inner(&mut self, ids: &HashSet<String>, command: Command) -> Result<()> {
        let rows = self.selected_rows(ids)?;
        let mutable = || -> Result<()> {
            if rows.iter().any(|t| t.reconciled) {
                Err("Reconciled transactions cannot be changed".into())
            } else {
                Ok(())
            }
        };
        match command {
            Command::Flag(flag) => {
                for t in &mut self.transactions {
                    if ids.contains(&t.id) {
                        t.flag = flag;
                    }
                }
            }
            Command::Cleared(value) => {
                mutable()?;
                if rows.iter().all(|t| t.cleared == value) {
                    return Err("Already in this state".into());
                }
                for t in &mut self.transactions {
                    if ids.contains(&t.id) {
                        t.cleared = value;
                    }
                }
            }
            Command::Approve => {
                if !rows.iter().any(|t| self.pending_import(&t.id)) {
                    return Err("No imports to approve".into());
                }
                for tid in ids {
                    if let Some(m) = self.register_meta.get_mut(tid) {
                        m.pending = false;
                    }
                }
            }
            Command::Reject => {
                mutable()?;
                if !rows.iter().all(|t| self.pending_import(&t.id)) {
                    return Err("Select imports awaiting approval".into());
                }
                for t in rows {
                    if let Some(backup) = self
                        .register_meta
                        .get(&t.id)
                        .and_then(|m| m.matched.clone())
                    {
                        *self.transactions.iter_mut().find(|x| x.id == t.id).unwrap() =
                            backup.manual;
                        self.register_meta.remove(&t.id);
                        if let Some(state) = backup.manual_state {
                            self.register_meta.insert(t.id.clone(), *state);
                        }
                    } else if self.transactions.iter().any(|x| x.id == t.id) {
                        self.delete_transaction(&t.id)?;
                    }
                }
            }
            Command::Delete => {
                for t in rows {
                    if self.transactions.iter().any(|x| x.id == t.id) {
                        self.delete_transaction(&t.id)?;
                    }
                }
            }
            Command::Categorize {
                category,
                replace_splits,
            } => {
                mutable()?;
                if category
                    .as_ref()
                    .is_some_and(|id| !self.categories.iter().any(|c| &c.id == id))
                {
                    return Err("Category not found".into());
                }
                if rows.iter().any(|t| t.transfer.is_some()) {
                    return Err("Transfers do not have a category".into());
                }
                if !replace_splits && rows.iter().any(|t| !t.splits.is_empty()) {
                    return Err("Confirm replacing split categories first".into());
                }
                for mut t in rows {
                    t.category = category.clone();
                    t.income_month = None;
                    t.splits.clear();
                    self.put_transaction(t)?;
                }
            }
            Command::IncomeMonth {
                next_month,
                replace_splits,
            } => {
                mutable()?;
                if rows.iter().any(|t| t.transfer.is_some()) {
                    return Err("Transfers do not have an income month".into());
                }
                if !replace_splits && rows.iter().any(|t| !t.splits.is_empty()) {
                    return Err("Confirm replacing split categories first".into());
                }
                for mut t in rows {
                    t.income_month = Some(month_shift(&t.date[..7], i32::from(next_month)));
                    t.category = None;
                    t.splits.clear();
                    self.put_transaction(t)?;
                }
            }
            Command::MoveAccount(account) => {
                mutable()?;
                let target = self.account(&account)?.clone();
                if target.closed {
                    return Err("Account is closed".into());
                }
                if rows.iter().all(|t| t.account == account) {
                    return Err("Already in this account".into());
                }
                for t in &rows {
                    if self.account(&t.account)?.currency != target.currency {
                        return Err("Account currencies must match".into());
                    }
                    for link in t.transfer_links() {
                        let peer = self
                            .transactions
                            .iter()
                            .find(|x| x.id != t.id && x.transfer_links().any(|other| other == link))
                            .ok_or("Transfer pair is missing")?;
                        if peer.reconciled || peer.account == account || ids.contains(&peer.id) {
                            return Err("This move would invalidate the transfer".into());
                        }
                    }
                }
                for t in &rows {
                    let (base, snap) = self.transaction_value(t)?;
                    let row = self.transactions.iter_mut().find(|x| x.id == t.id).unwrap();
                    row.account = account.clone();
                    row.base_amount = base;
                    row.snapshot = snap;
                    let source_name = self.account(&account)?.name.clone();
                    for link in t.transfer_links() {
                        let peer = self
                            .transactions
                            .iter_mut()
                            .find(|x| x.id != t.id && x.transfer_links().any(|other| other == link))
                            .ok_or("Transfer pair is missing")?;
                        if peer.transfer.as_deref() == Some(link) {
                            peer.payee = format!("Transfer : {source_name}");
                        } else {
                            let part = peer
                                .splits
                                .iter_mut()
                                .find(|s| s.transfer.as_ref().is_some_and(|x| x.link == link))
                                .ok_or("Transfer part is missing")?;
                            part.transfer.as_mut().unwrap().account = account.clone();
                        }
                    }
                    if let Some(mut backup) = self
                        .register_meta
                        .get(&t.id)
                        .and_then(|m| m.matched.clone())
                    {
                        for original in [&mut backup.manual, &mut backup.imported] {
                            original.account = account.clone();
                            let (base, snapshot) = self.transaction_value(original)?;
                            original.base_amount = base;
                            original.snapshot = snapshot;
                        }
                        self.register_meta.get_mut(&t.id).unwrap().matched = Some(backup);
                    }
                }
            }
            Command::Match => {
                if rows.len() != 2 {
                    return Err("Select one imported and one manual transaction".into());
                }
                let imported = rows
                    .iter()
                    .find(|t| self.register_meta.contains_key(&t.id))
                    .ok_or("Select an imported transaction")?;
                let manual = rows
                    .iter()
                    .find(|t| !self.register_meta.contains_key(&t.id))
                    .ok_or("Select a manual transaction")?;
                self.match_import(&imported.id, &manual.id)?;
            }
            Command::Unmatch => {
                if !rows.iter().all(|t| {
                    self.register_meta.get(&t.id).is_some_and(|m| {
                        m.matched
                            .as_ref()
                            .is_some_and(|pair| !t.reconciled || pair.manual.reconciled)
                    })
                }) {
                    return Err("Select matched transactions without a later reconciliation".into());
                }
                for t in rows {
                    let m = self.register_meta.remove(&t.id).unwrap();
                    let backup = m.matched.unwrap();
                    *self.transactions.iter_mut().find(|x| x.id == t.id).unwrap() =
                        backup.manual.clone();
                    if let Some(state) = backup.manual_state {
                        self.register_meta.insert(backup.manual.id.clone(), *state);
                    }
                    self.register_meta.insert(
                        backup.imported.id.clone(),
                        ImportState {
                            source_id: m.source_id,
                            pending: backup.pending,
                            matched: None,
                        },
                    );
                    self.transactions.push(backup.imported);
                }
            }
            Command::RenamePayees(name) => {
                let name = name.trim();
                if name.is_empty() {
                    return Err("Enter a payee name".into());
                }
                if rows
                    .iter()
                    .any(|t| t.transfer.is_some() || t.payee.is_empty())
                {
                    return Err("Select named payees".into());
                }
                let names: HashSet<_> = rows.iter().map(|t| t.payee.clone()).collect();
                for old in names {
                    self.rename_payee(&old, name)?;
                }
            }
            Command::Schedule {
                date,
                repeat,
                move_original,
            } => {
                mutable()?;
                if !valid_date(&date) || (move_original && date <= today()) {
                    return Err("Choose a future schedule date".into());
                }
                let mut all = HashSet::new();
                for t in &rows {
                    all.extend(crate::splits::connected(&self.transactions, &t.id));
                }
                let to_schedule: Vec<_> = self
                    .transactions
                    .iter()
                    .filter(|t| all.contains(&t.id))
                    .cloned()
                    .collect();
                if to_schedule.iter().any(|t| t.reconciled) {
                    return Err("Reconciled transfers cannot be scheduled".into());
                }
                let mut links = std::collections::BTreeMap::new();
                for mut t in to_schedule.clone() {
                    if self.account(&t.account)?.closed {
                        return Err("Account is closed".into());
                    }
                    if let Some(link) = &t.transfer {
                        t.transfer = Some(links.entry(link.clone()).or_insert_with(id).clone());
                    }
                    for part in &mut t.splits {
                        if let Some(transfer) = &mut part.transfer {
                            transfer.link = links
                                .entry(transfer.link.clone())
                                .or_insert_with(id)
                                .clone();
                        }
                    }
                    if !move_original {
                        t.id = id();
                    }
                    t.set_date(date.clone());
                    t.cleared = false;
                    t.reconciled = false;
                    t.operation_fx = None;
                    t.snapshot = None;
                    t.base_amount = 0;
                    let anchor = NaiveDate::parse_from_str(&date, "%Y-%m-%d").unwrap().day();
                    self.scheduled.push(Scheduled {
                        id: id(),
                        transaction: t,
                        next_date: date.clone(),
                        monthly: repeat == Repeat::Monthly,
                        repeat: Some(repeat),
                        anchor_day: Some(anchor),
                    });
                }
                if move_original {
                    let moved: HashSet<_> = to_schedule.iter().map(|t| t.id.clone()).collect();
                    self.transactions.retain(|t| !moved.contains(&t.id));
                    self.register_meta.retain(|id, _| !moved.contains(id));
                }
            }
        }
        Ok(())
    }
    pub(crate) fn post_schedule_inner(&mut self, sid: &str) -> Result<()> {
        self.post_schedule_inner_at(sid, None)
    }
    pub(crate) fn post_schedule_inner_at(
        &mut self,
        sid: &str,
        posted_date: Option<&str>,
    ) -> Result<()> {
        let selected = self
            .scheduled
            .iter()
            .find(|s| s.id == sid)
            .ok_or("Schedule not found")?
            .clone();
        if posted_date.is_none() && selected.next_date > today() {
            return Err("Scheduled date has not arrived".into());
        }
        let ids = self.schedule_ids(sid)?;
        let batch: Vec<_> = self
            .scheduled
            .iter()
            .filter(|s| ids.contains(&s.id))
            .cloned()
            .collect();
        let mut links = std::collections::BTreeMap::new();
        for s in batch {
            let mut t = s.transaction.clone();
            t.id = id();
            t.set_date(posted_date.unwrap_or(&s.next_date).into());
            if let Some(link) = &mut t.transfer {
                *link = links.entry(link.clone()).or_insert_with(id).clone();
            }
            for part in &mut t.splits {
                if let Some(transfer) = &mut part.transfer {
                    transfer.link = links
                        .entry(transfer.link.clone())
                        .or_insert_with(id)
                        .clone();
                }
            }
            let account = self.account(&t.account)?;
            if account.closed {
                return Err("Account is closed".into());
            }
            let (base, snapshot) = self.transaction_value(&t)?;
            t.base_amount = base;
            t.snapshot = snapshot;
            self.freeze_posting(&mut t)?;
            self.transactions.push(t);
            let anchor = s.anchor_day.unwrap_or(
                NaiveDate::parse_from_str(&s.next_date, "%Y-%m-%d")
                    .map_err(|_| "Invalid schedule date")?
                    .day(),
            );
            if let Some(date) = s.frequency().next(&s.next_date, anchor)? {
                let row = self.scheduled.iter_mut().find(|x| x.id == s.id).unwrap();
                row.next_date = date.clone();
                row.transaction.set_date(date);
                row.anchor_day = Some(anchor);
            } else {
                self.scheduled.retain(|x| x.id != s.id);
            }
        }
        Ok(())
    }
}
