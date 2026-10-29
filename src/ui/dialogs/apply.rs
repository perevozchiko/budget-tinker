use super::*;

impl App {
    pub(in crate::ui) fn save_dialog(&mut self, d: &Dialog) -> Result<()> {
        let f = &d.fields;
        let mut b = self.b.clone();
        match &d.kind {
            DialogKind::Register { ids, command } => {
                let command = match command {
                    RegisterCommand::RenamePayees(_) => RegisterCommand::RenamePayees(f[0].clone()),
                    RegisterCommand::Schedule { move_original, .. } => RegisterCommand::Schedule {
                        date: f[0].clone(),
                        repeat: Repeat::from_code(&f[1]),
                        move_original: *move_original,
                    },
                    other => other.clone(),
                };
                b.register_command(ids, command)?;
            }
            DialogKind::Account => {
                b.add_typed_account(
                    &f[0],
                    &b.base.clone(),
                    d.check,
                    amount(&f[2], &b.base)?,
                    &f[3],
                    &f[4],
                )?;
            }
            DialogKind::Transaction(tid) => {
                let currency = b.account(&f[0])?.currency.clone();
                let splits = d
                    .splits
                    .iter()
                    .map(|(c, a)| {
                        Ok(Split {
                            category: if c.is_empty() { None } else { Some(c.clone()) },
                            amount: amount(a, &currency)?,
                            ..Default::default()
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let t = Transaction {
                    operation_fx: None,
                    check_number: tid
                        .as_ref()
                        .and_then(|id| b.transactions.iter().find(|t| &t.id == id))
                        .and_then(|t| t.check_number.clone()),
                    flag: tid
                        .as_ref()
                        .and_then(|id| b.transactions.iter().find(|t| &t.id == id))
                        .and_then(|t| t.flag),
                    id: tid.clone().unwrap_or_else(id),
                    account: f[0].clone(),
                    date: f[1].clone(),
                    payee: f[2].clone(),
                    memo: f[5].clone(),
                    amount: amount(&f[4], &currency)?,
                    base_amount: 0,
                    snapshot: None,
                    income_month: if f[3].is_empty() && splits.is_empty() && valid_date(&f[1]) {
                        match f[7].as_str() {
                            "current" => Some(f[1][..7].into()),
                            "next" => Some(month_shift(&f[1][..7], 1)),
                            _ => None,
                        }
                    } else {
                        None
                    },
                    category: if f[3].is_empty() || !splits.is_empty() {
                        None
                    } else {
                        Some(f[3].clone())
                    },
                    cleared: d.check,
                    reconciled: false,
                    transfer: None,
                    splits,
                };
                if d.second {
                    if tid.is_some() {
                        return Err("Создайте отдельную операцию для плана".into());
                    }
                    if chrono::NaiveDate::parse_from_str(&f[1], "%Y-%m-%d").is_err() {
                        return Err("Некорректная дата плана".into());
                    }
                    if !t.splits.is_empty()
                        && t.splits.iter().map(|s| s.amount).sum::<i64>() != t.amount
                    {
                        return Err("Сумма частей не совпадает".into());
                    }
                    b.scheduled.push(Scheduled {
                        id: id(),
                        next_date: f[1].clone(),
                        transaction: t,
                        monthly: d.fields[6] == "monthly",
                        repeat: Some(budgettinker::register::Repeat::from_code(&d.fields[6])),
                        anchor_day: None,
                    });
                } else {
                    b.put_transaction(t)?;
                }
            }
            DialogKind::Transfer => {
                let from = b.account(&f[0])?.currency.clone();
                if b.account(&f[1])?.currency != from {
                    return Err("Перевод между этими счетами пока недоступен".into());
                }
                if d.second {
                    scheduled_panel::schedule_transfer(
                        &mut b,
                        scheduled_panel::TransferDraft {
                            from: &f[0],
                            to: &f[1],
                            amount: amount(&f[3], &from)?,
                            date: &f[2],
                            memo: &f[5],
                            repeat: Repeat::from_code(&f[6]),
                        },
                    )?;
                } else {
                    b.transfer(
                        &f[0],
                        &f[1],
                        amount(&f[3], &from)?,
                        amount(&f[3], &from)?,
                        &f[2],
                        &f[5],
                    )?;
                }
            }
            DialogKind::NewGroup => {
                b.add_category_group(&f[0])?;
            }
            DialogKind::Category => {
                b.add_category(&f[0], &f[1])?;
            }
            DialogKind::EditCategory(cid) => {
                if budgettinker::credit::debt_account(cid).is_some()
                    && (d.check || f[1] != f[8] || f[0] != f[7])
                {
                    return Err("Starting Debt categories follow the account name. Rename the account, or hide this category when the old debt is paid off.".into());
                }
                if f[0].trim().is_empty() || f[1].trim().is_empty() {
                    return Err("Введите группу и название категории".into());
                }
                if d.check {
                    if b.transactions.iter().any(|t| {
                        t.category.as_ref() == Some(cid)
                            || t.splits.iter().any(|s| s.category.as_ref() == Some(cid))
                    }) || b.scheduled.iter().any(|s| {
                        s.transaction.category.as_ref() == Some(cid)
                            || s.transaction
                                .splits
                                .iter()
                                .any(|s| s.category.as_ref() == Some(cid))
                    }) || b
                        .allocations
                        .iter()
                        .any(|(key, v)| key.ends_with(&format!("/{cid}")) && *v != 0)
                    {
                        return Err(
                            "Категория используется. Сначала перенесите операции и назначения."
                                .into(),
                        );
                    }
                    b.categories.retain(|c| c.id != *cid);
                    for rule in b.payee_rules.values_mut() {
                        if rule.category.as_ref() == Some(cid) {
                            rule.category = None;
                        }
                    }
                    b.allocations
                        .retain(|key, _| !key.ends_with(&format!("/{cid}")));
                    b.overspending_rules
                        .retain(|key, _| !key.ends_with(&format!("/{cid}")));
                } else {
                    let c = b
                        .categories
                        .iter_mut()
                        .find(|c| c.id == *cid)
                        .ok_or("Категория не найдена")?;
                    if f[1] != f[8] {
                        c.name = f[1].trim().into();
                    }
                    if f[0] != f[7] {
                        c.group = f[0].trim().into();
                    }
                    c.hidden = d.second;
                }
            }
            DialogKind::EditGroup(old) => {
                if d.check {
                    b.delete_group(old)?;
                } else {
                    let name = if f[0] == f[7] {
                        old.as_str()
                    } else {
                        f[0].trim()
                    };
                    b.rename_group(old, name)?;
                    b.set_group_hidden(name, d.second)?;
                }
            }
            DialogKind::Reconcile(a) => {
                self.begin_reconciliation(a, amount(&f[0], &b.account(a)?.currency)?, &f[1])?;
                return Ok(());
            }
            DialogKind::AdjustBalance(a) => {
                b.adjust_balance(a, amount(&f[0], &b.account(a)?.currency)?, &f[1])?;
            }
            DialogKind::Import => {
                let preview = d
                    .bank_preview
                    .as_ref()
                    .ok_or("Run Check and Preview first.")?;
                let (next, count) = preview.apply(&b, Path::new(&f[1]), &f[0])?;
                if count > 0 {
                    self.commit(next)?;
                }
                self.notify(
                    format!("Imported {count} transactions. Repeated rows were skipped."),
                    false,
                );
                return Ok(());
            }
            DialogKind::ImportLegacyBudget => {
                if !d.check {
                    return Err(
                        "Confirm replacement of the current budget. Undo can restore it.".into(),
                    );
                }
                let preview = d
                    .legacy_preview
                    .as_ref()
                    .ok_or("Run Check and Preview first.")?;
                preview.verify_source(Path::new(&f[0]))?;
                self.commit(preview.budget.clone())?;
                self.reconciliation = None;
                self.selected.clear();
                self.scheduled_selected.clear();
                self.transaction_editor = None;
                self.scheduled_editor = None;
                self.budget_editor = BudgetEditor::default();
                self.collapsed.clear();
                self.search.clear();
                self.report_filter = Default::default();
                self.report_accounts = Default::default();
                self.report_navigation.clear();
                self.report_selected_month = None;
                self.page = Page::Budget;
                self.month = self
                    .b
                    .transactions
                    .iter()
                    .map(|t| &t.date[..7])
                    .min()
                    .unwrap_or(&current_month())
                    .to_string();
                self.notify(
                    "Legacy budget imported. Undo restores the previous budget.",
                    false,
                );
                return Ok(());
            }
            DialogKind::Restore => {
                if !d.check {
                    return Err("Подтвердите замену текущего бюджета".into());
                }
                b = store::import_json(Path::new(&f[0]))?;
            }
            DialogKind::AccountSettings(a) => {
                if f[0].trim().is_empty() {
                    return Err("Введите название счёта".into());
                }
                if d.check && b.balance(a, None, false) != 0 {
                    return Err("Для закрытия счёта его остаток должен быть нулевым".into());
                }
                b.rename_account(a, &f[0])?;
                let a = b.accounts.iter_mut().find(|x| x.id == *a).unwrap();
                a.closed = d.check;
                a.account_type = f[1].clone();
                a.on_budget = d.second;
                let key = format!("account:{}", a.id);
                if f[2].trim().is_empty() {
                    b.notes.remove(&key);
                } else {
                    b.notes.insert(key, f[2].clone());
                }
            }
            DialogKind::Overspending { month, category } => {
                b.set_overspending_rule(
                    month,
                    category,
                    if d.check {
                        OverspendingRule::CategoryBalance
                    } else {
                        OverspendingRule::AvailableToBudget
                    },
                )?;
            }
            DialogKind::Note(key) => {
                if f[0].trim().is_empty() {
                    b.notes.remove(key);
                } else {
                    b.notes.insert(key.clone(), f[0].clone());
                }
            }
        }
        self.commit(b)?;
        self.selected
            .retain(|id| self.b.transactions.iter().any(|t| &t.id == id));
        if let DialogKind::EditGroup(old) = &d.kind
            && self.collapsed.remove(old)
            && !d.check
        {
            self.collapsed.insert(if f[0] == f[7] {
                old.clone()
            } else {
                f[0].trim().into()
            });
        }
        Ok(())
    }
}
