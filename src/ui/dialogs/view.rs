use super::*;

impl App {
    pub(in crate::ui) fn dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.dialog.take() else {
            return;
        };
        if matches!(d.kind, DialogKind::ImportLegacyBudget) {
            self.legacy_import_form(ctx, d);
            return;
        }
        if matches!(d.kind, DialogKind::Import) {
            self.bank_form(ctx, d);
            return;
        }
        if matches!(
            d.kind,
            DialogKind::AccountSettings(_)
                | DialogKind::AdjustBalance(_)
                | DialogKind::Reconcile(_)
                | DialogKind::Category
                | DialogKind::NewGroup
                | DialogKind::EditCategory(_)
                | DialogKind::EditGroup(_)
                | DialogKind::Note(_)
                | DialogKind::Overspending { .. }
        ) {
            self.detail_form(ctx, d);
            return;
        }
        if matches!(d.kind, DialogKind::Account) {
            self.classic_form(ctx, d);
            return;
        }
        let title = match d.kind {
            DialogKind::Register { ref command, .. } => match command {
                RegisterCommand::RenamePayees(_) => "Rename like payees",
                RegisterCommand::Categorize { .. } | RegisterCommand::IncomeMonth { .. } => {
                    "Replace split categories?"
                }
                RegisterCommand::Schedule {
                    move_original: true,
                    ..
                } => "Move to scheduler",
                RegisterCommand::Schedule { .. } => "Make recurring",
                _ => "Edit Transactions",
            },
            DialogKind::Transaction(_) => "Операция",
            DialogKind::Transfer => "Перевод между счетами",
            DialogKind::Restore => "Восстановление бюджета",
            _ => unreachable!("dedicated form"),
        };
        let mut keep = true;
        let mut save = false;
        chrome::modal(ctx, "dialog").show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            chrome::header(ui, title, 580.);
            Frame::new().inner_margin(20).show(ui, |ui| {
                ui.set_width(540.);
                ui.spacing_mut().item_spacing.y = 8.;
                egui::Grid::new("dialog_fields")
                    .num_columns(2)
                    .spacing([15., 11.])
                    .show(ui, |ui| match &d.kind {
                        DialogKind::Register { ids, command } => {
                            match command {
                                RegisterCommand::RenamePayees(_) => {
                                    ui.label("Rename all transactions with these payees, including scheduled transactions.");
                                    ui.end_row();
                                    field(ui, "New payee name", &mut d.fields[0]);
                                }
                                RegisterCommand::Categorize { .. } | RegisterCommand::IncomeMonth { .. } => {
                                    let count = self.b.transactions.iter().filter(|t| ids.contains(&t.id) && !t.splits.is_empty()).count();
                                    ui.label(format!("Replace the splits in {count} transaction(s) with one category? Amounts will stay the same."));
                                    ui.end_row();
                                }
                                RegisterCommand::Schedule { move_original, .. } => {
                                    ui.label(if *move_original { "Move the selected transactions out of the register into the schedule." } else { "Create scheduled copies. The selected transactions stay in the register." });
                                    ui.end_row();
                                    field(ui, "First date (YYYY-MM-DD)", &mut d.fields[0]);
                                    ui.label("Repeat");
                                    repeat_combo(ui, "register_repeat", &mut d.fields[1]);
                                    ui.end_row();
                                }
                                _ => {}
                            }
                        }
                        DialogKind::Transaction(tid) => {
                            ui.l_label("Счёт");
                            account_combo(ui, "tx_account", &mut d.fields[0], &self.b);
                            ui.end_row();
                            field(ui, "Дата · ГГГГ-ММ-ДД", &mut d.fields[1]);
                            field(ui, "Получатель", &mut d.fields[2]);
                            ui.l_label("Категория");
                            let mut income_next = match d.fields[7].as_str() { "current" => Some(false), "next" => Some(true), _ => None };
                            let date = d.fields[1].clone();
                            if let Some(selection) = crate::transaction_editor::category_combo(ui, "tx_category", &mut d.fields[3], &mut income_next, &date, !d.splits.is_empty(), &self.b, 240.) {
                                match selection.choice {
                                    crate::category_picker::Choice::Split if d.splits.is_empty() => d.splits.push((d.fields[3].clone(), String::new())),
                                    crate::category_picker::Choice::Create(request) => match self.create_picker_category(request) {
                                        Ok(id) => { d.fields[3] = id; income_next = None; }
                                        Err(error) => d.error = error,
                                    },
                                    _ => {}
                                }
                            }
                            d.fields[7] = match income_next { Some(false) => "current", Some(true) => "next", None => "" }.into();
                            ui.end_row();
                            field(ui, "Сумма · − расход, + доход", &mut d.fields[4]);
                            field(ui, "Заметка", &mut d.fields[5]);
                            ui.l_label("Статус");
                            ui.l_checkbox(&mut d.check, "Подтверждена банком");
                            ui.end_row();
                            if tid.is_none() {
                                ui.l_label("План");
                                ui.l_checkbox(&mut d.second, "Запланировать, не проводить сейчас");
                                ui.end_row();
                                if d.second {
                                    ui.l_label("Повторение");
                                    repeat_combo(ui, "transaction_repeat", &mut d.fields[6]);
                                    ui.end_row();
                                }
                            }
                        }
                        DialogKind::Transfer => {
                            ui.l_label("Со счёта");
                            account_combo(ui, "transfer_from", &mut d.fields[0], &self.b);
                            ui.end_row();
                            ui.l_label("На счёт");
                            account_combo(ui, "transfer_to", &mut d.fields[1], &self.b);
                            ui.end_row();
                            field(ui, "Дата", &mut d.fields[2]);
                            field(ui, "Сумма", &mut d.fields[3]);
                            field(ui, "Заметка", &mut d.fields[5]);
                            ui.l_label("План");
                            ui.l_checkbox(&mut d.second, "Запланировать, не проводить сейчас");
                            ui.end_row();
                            if d.second {
                                ui.l_label("Повторение");
                                repeat_combo(ui, "transfer_repeat", &mut d.fields[6]);
                                ui.end_row();
                            }
                        }
                        DialogKind::Restore => {
                            field(ui, "Путь к JSON", &mut d.fields[0]);
                            ui.l_label("");
                            if classic::button(ui, "Выбрать файл…").clicked()
                                && let Some(p) = self.choose_file(false, "json", ui.ctx())
                            {
                                d.fields[0] = p;
                            }
                            ui.end_row();
                            ui.l_label("Подтверждение");
                            ui.l_checkbox(&mut d.check, "Заменить текущий бюджет (можно отменить)");
                            ui.end_row();
                        }
                        _ => unreachable!("dedicated form"),
                    });
                match &d.kind {
                    DialogKind::Transaction(_) => {
                        ui.add_space(10.);
                        ui.separator();
                        if classic::small_button(ui, "+ Разбить по категориям").clicked()
                        {
                            d.splits.push((String::new(), String::new()));
                        }
                        let mut remove = None;
                        for (i, (category, value)) in d.splits.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                category_combo(ui, &format!("split_{i}"), category, &self.b);
                                ui.add(egui::TextEdit::singleline(value).desired_width(90.));
                                if classic::small_button(ui, "×").clicked() {
                                    remove = Some(i);
                                }
                            });
                        }
                        if let Some(i) = remove {
                            d.splits.remove(i);
                        }
                    }
                    DialogKind::Transfer => {
                        ui.add_space(10.);
                        ui.l_label(
                            "Укажите сумму перевода. Перевод не считается доходом или расходом.",
                        );
                    }
                    _ => {}
                }
                if !d.error.is_empty() {
                    ui.add_space(8.);
                    text(ui, &d.error, 12., RED);
                }
            });
            chrome::footer(ui, 580., |ui| {
                if ui
                    .add(classic::Button::new("Сохранить").icon(classic::Glyph::Check))
                    .clicked()
                {
                    save = true;
                }
                if classic::button(ui, "Отмена").clicked() {
                    keep = false;
                }
            });
        });
        if save {
            match self.save_dialog(&d) {
                Ok(()) => keep = false,
                Err(e) => d.error = e,
            }
        }
        if !egui::Popup::is_any_open(ctx)
            && !crate::category_picker::is_open(ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            keep = false;
        }
        if keep {
            self.dialog = Some(d);
        }
    }
}
