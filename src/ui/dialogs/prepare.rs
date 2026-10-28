use super::*;

impl App {
    pub(in crate::ui) fn open(&mut self, kind: DialogKind) {
        self.dragging_category = None;
        let mut d = Dialog::new(kind.clone());
        match kind {
            DialogKind::Account => {
                d.fields[1] = self.b.base.clone();
                d.fields[2] = "0.00".into();
                d.fields[3] = today();
                d.fields[8] = chrono::NaiveDate::parse_from_str(&d.fields[3], "%Y-%m-%d")
                    .unwrap()
                    .format("%m/%d/%Y")
                    .to_string();
                d.check = true;
            }
            DialogKind::Transaction(tid) => {
                d.fields[0] = self
                    .account
                    .clone()
                    .or_else(|| {
                        self.b
                            .accounts
                            .iter()
                            .find(|a| !a.closed)
                            .map(|a| a.id.clone())
                    })
                    .unwrap_or_default();
                d.fields[1] = today();
                d.fields[4] = "0.00".into();
                if let Some(t) = tid.and_then(|id| self.b.transactions.iter().find(|t| t.id == id))
                {
                    d.fields[0] = t.account.clone();
                    d.fields[1] = t.date.clone();
                    d.fields[2] = t.payee.clone();
                    d.fields[3] = t.category.clone().unwrap_or_default();
                    d.fields[7] = t
                        .income_month
                        .as_ref()
                        .map(|m| {
                            if m.as_str() > &t.date[..7] {
                                "next"
                            } else {
                                "current"
                            }
                            .into()
                        })
                        .unwrap_or_else(|| {
                            if t.category.is_none() && t.splits.is_empty() && t.amount > 0 {
                                "current".into()
                            } else {
                                String::new()
                            }
                        });
                    d.fields[4] = plain(t.amount, &self.b.account(&t.account).unwrap().currency);
                    d.fields[5] = t.memo.clone();
                    d.check = t.cleared;
                    d.splits = t
                        .splits
                        .iter()
                        .map(|s| {
                            (
                                s.category.clone().unwrap_or_default(),
                                plain(s.amount, &self.b.account(&t.account).unwrap().currency),
                            )
                        })
                        .collect();
                }
            }
            DialogKind::Transfer => {
                d.fields[0] = self.account.clone().unwrap_or_default();
                d.fields[2] = today();
            }
            DialogKind::NewGroup => {
                d.fields[0] = "New Master Category".into();
            }
            DialogKind::Category => {
                d.fields[1] = "New Category".into();
                d.fields[0] = i18n::tr_language(self.preferences.language, "Повседневные расходы");
                d.fields[7] = d.fields[0].clone();
            }
            DialogKind::EditCategory(cid) => {
                if let Some(c) = self.b.categories.iter().find(|c| c.id == cid) {
                    d.fields[0] = i18n::tr_language(self.preferences.language, &c.group);
                    d.fields[1] = i18n::tr_language(self.preferences.language, &c.name);
                    d.fields[7] = d.fields[0].clone();
                    d.fields[8] = d.fields[1].clone();
                    d.second = c.hidden;
                }
            }
            DialogKind::EditGroup(group) => {
                d.fields[0] = i18n::tr_language(self.preferences.language, &group);
                d.fields[7] = d.fields[0].clone();
                d.second = self
                    .b
                    .category_groups
                    .iter()
                    .find(|g| g.name == group)
                    .map(|g| g.hidden)
                    .unwrap_or_else(|| {
                        self.b
                            .categories
                            .iter()
                            .filter(|c| c.group == group)
                            .all(|c| c.hidden)
                    });
            }
            DialogKind::Overspending { month, category } => {
                d.check = *self.b.overspending_rule(&month, &category)
                    == OverspendingRule::CategoryBalance;
            }
            DialogKind::Reconcile(_) => {
                d.fields[1] = today();
                if let Some(session) = &self.reconciliation {
                    d.fields[1] = session.date.clone();
                    d.fields[0] = plain(
                        session.statement,
                        &self.b.account(&session.account).unwrap().currency,
                    );
                }
                d.fields[8] = chrono::NaiveDate::parse_from_str(&d.fields[1], "%Y-%m-%d")
                    .unwrap()
                    .format("%m/%d/%Y")
                    .to_string();
            }
            DialogKind::Import => {
                d.fields[0] = self.account.clone().unwrap_or_default();
            }
            DialogKind::AccountSettings(a) => {
                if let Ok(a) = self.b.account(&a) {
                    d.fields[0] = a.name.clone();
                    d.fields[1] = a.account_type.clone();
                    d.fields[2] = self
                        .b
                        .notes
                        .get(&format!("account:{}", a.id))
                        .cloned()
                        .unwrap_or_default();
                    d.check = a.closed;
                    d.second = a.on_budget;
                }
            }
            DialogKind::AdjustBalance(a) => {
                if let Ok(account) = self.b.account(&a) {
                    d.fields[0] = plain(self.b.balance(&a, None, false), &account.currency);
                    d.fields[1] = today();
                }
            }
            DialogKind::Register {
                ref ids,
                ref command,
            } => match command {
                RegisterCommand::RenamePayees(_) => {
                    d.fields[0] = self
                        .b
                        .transactions
                        .iter()
                        .find(|t| ids.contains(&t.id))
                        .map(|t| t.payee.clone())
                        .unwrap_or_default();
                }
                RegisterCommand::Schedule { move_original, .. } => {
                    d.fields[0] = chrono::Local::now()
                        .date_naive()
                        .checked_add_signed(chrono::Duration::days(1))
                        .unwrap()
                        .to_string();
                    d.fields[1] = if *move_original { "" } else { "monthly" }.into();
                }
                _ => {}
            },
            DialogKind::Restore | DialogKind::ImportLegacyBudget => {}
            DialogKind::Note(key) => {
                d.fields[0] = self.b.notes.get(&key).cloned().unwrap_or_default();
            }
        }
        self.dialog = Some(d);
    }
    pub(in crate::ui) fn open_at(&mut self, kind: DialogKind, rect: egui::Rect) {
        self.open(kind);
        if let Some(dialog) = &mut self.dialog {
            dialog.anchor = Some(egui::pos2(rect.left() + 40., rect.bottom() + 12.));
        }
    }
}
