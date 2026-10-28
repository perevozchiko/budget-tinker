use super::*;

impl App {
    pub(in crate::ui) fn create_picker_category(
        &mut self,
        request: crate::category_picker::NewCategory,
    ) -> Result<String> {
        let mut next = self.b.clone();
        let id = next.add_category(&request.group, &request.name)?;
        self.commit(next)?;
        Ok(id)
    }

    pub(in crate::ui) fn finish_category_creation(&mut self, editing: &mut RegisterDraft) {
        if let Some((request, split)) = editing.pending_category.take() {
            match self.create_picker_category(request) {
                Ok(id) => {
                    if let Some(index) = split {
                        editing.set_split_category(index, id);
                    } else {
                        editing.category = id;
                        editing.income_next = None;
                    }
                    editing.error.clear();
                }
                Err(error) => editing.error = error,
            }
        }
    }

    pub(in crate::ui) fn start_register_edit(&mut self, transaction: Option<&str>) {
        if self.transaction_editor.is_some() || self.scheduled_editor.is_some() {
            self.notify("Сначала сохраните или отмените текущую строку", true);
            return;
        }
        match RegisterDraft::new(&self.b, self.account.as_deref(), transaction) {
            Ok(draft) => {
                self.transaction_editor = Some(draft);
                self.page = Page::Accounts;
                self.search.clear();
            }
            Err(e) => self.notify(e, true),
        }
    }
    pub(in crate::ui) fn run_register_command(
        &mut self,
        ids: &HashSet<String>,
        command: RegisterCommand,
    ) {
        let mut next = self.b.clone();
        match next
            .register_command(ids, command)
            .and_then(|_| self.commit(next))
        {
            Ok(()) => self
                .selected
                .retain(|id| self.b.transactions.iter().any(|t| &t.id == id)),
            Err(error) => self.notify(error, true),
        }
    }

    pub(in crate::ui) fn register_edit_menu(&mut self, ui: &mut egui::Ui, context: bool) {
        let selected: Vec<_> = self
            .b
            .transactions
            .iter()
            .filter(|t| self.selected.contains(&t.id))
            .cloned()
            .collect();
        let count = selected.len();
        let selected_ids = self.selected.clone();
        let idle = self.transaction_editor.is_none();
        let editable = idle && count > 0 && selected.iter().all(|t| !t.reconciled);
        classic::register_menu_style(ui);
        if context {
            let label = if count == 1 {
                "1 selected transaction:".to_owned()
            } else {
                format!("{count} selected transactions:")
            };
            ui.add_enabled(false, egui::Button::new(label).frame_when_inactive(false));
            ui.separator();
        }
        for (label, cleared) in [("Mark as Cleared", true), ("Mark as Uncleared", false)] {
            let command = RegisterCommand::Cleared(cleared);
            if ui
                .add_enabled(
                    self.b.can_register_command(&selected_ids, command.clone()),
                    egui::Button::new(label)
                        .frame_when_inactive(false)
                        .min_size(Vec2::new(184., 24.)),
                )
                .clicked()
            {
                self.run_register_command(&selected_ids, command);
                ui.close();
            }
        }
        ui.separator();
        for (label, command) in [
            ("Approve", RegisterCommand::Approve),
            ("Reject (Delete)", RegisterCommand::Reject),
        ] {
            if ui
                .add_enabled(
                    self.b.can_register_command(&selected_ids, command.clone()),
                    egui::Button::new(label).frame_when_inactive(false),
                )
                .clicked()
            {
                self.run_register_command(&selected_ids, command);
                ui.close();
            }
        }
        ui.separator();
        ui.add_enabled_ui(
            editable && selected.iter().all(|t| t.transfer.is_none()),
            |ui| {
                ui.menu_button("Categorize As", |ui| {
                    let common_month = selected
                        .first()
                        .map(|t| &t.date[..7])
                        .filter(|month| selected.iter().all(|t| &t.date[..7] == *month));
                    for (relative_label, next_month) in [
                        ("Income for this month", false),
                        ("Income for next month", true),
                    ] {
                        let label = common_month
                            .map(|month| {
                                budgettinker::income::income_label(&month_shift(
                                    month,
                                    i32::from(next_month),
                                ))
                            })
                            .unwrap_or_else(|| relative_label.into());
                        if ui.button(label).clicked() {
                            let command = RegisterCommand::IncomeMonth {
                                next_month,
                                replace_splits: true,
                            };
                            if selected.iter().any(|t| !t.splits.is_empty()) {
                                self.open(DialogKind::Register {
                                    ids: selected_ids.clone(),
                                    command,
                                });
                            } else {
                                self.run_register_command(&selected_ids, command);
                            }
                            ui.close();
                        }
                    }
                    ui.separator();
                    let choices = std::iter::once((None, "Income / Uncategorized".to_owned()))
                        .chain(self.b.categories.iter().filter(|c| !c.hidden).map(|c| {
                            (
                                Some(c.id.clone()),
                                format!(
                                    "{}: {}",
                                    i18n::tr(ui.ctx(), &c.group),
                                    i18n::tr(ui.ctx(), &c.name)
                                ),
                            )
                        }))
                        .collect::<Vec<_>>();
                    for (category, name) in choices {
                        if ui.button(name).clicked() {
                            let command = RegisterCommand::Categorize {
                                category,
                                replace_splits: true,
                            };
                            if selected.iter().any(|t| !t.splits.is_empty()) {
                                self.open(DialogKind::Register {
                                    ids: selected_ids.clone(),
                                    command,
                                });
                            } else {
                                self.run_register_command(&selected_ids, command);
                            }
                            ui.close();
                        }
                    }
                });
            },
        );
        ui.separator();
        let named = selected
            .iter()
            .all(|t| t.transfer.is_none() && !t.payee.is_empty());
        if ui
            .add_enabled(
                named,
                egui::Button::new("Rename like payees...").frame_when_inactive(false),
            )
            .clicked()
        {
            self.open(DialogKind::Register {
                ids: selected_ids.clone(),
                command: RegisterCommand::RenamePayees(String::new()),
            });
            ui.close();
        }
        let payee = selected
            .first()
            .map(|t| t.payee.as_str())
            .unwrap_or_default();
        if ui
            .add_enabled(
                named,
                egui::Button::new("Payee settings...").frame_when_inactive(false),
            )
            .clicked()
        {
            self.chrome.payee = payee.to_owned();
            self.chrome.renamed_payee = payee.to_owned();
            self.chrome.rename_pattern = self
                .b
                .payee_rules
                .get(payee)
                .map(|rule| rule.rename_contains.clone())
                .unwrap_or_default();
            self.chrome.error.clear();
            self.chrome.overlay = Some(chrome::Overlay::Payees);
            ui.close();
        }
        ui.separator();
        for (label, move_original) in [("Make recurring...", false), ("Move to scheduler...", true)]
        {
            let command = RegisterCommand::Schedule {
                date: chrono::Local::now()
                    .date_naive()
                    .checked_add_signed(chrono::Duration::days(1))
                    .unwrap()
                    .to_string(),
                repeat: if move_original {
                    Repeat::Never
                } else {
                    Repeat::Monthly
                },
                move_original,
            };
            if ui
                .add_enabled(
                    self.b.can_register_command(&selected_ids, command.clone()),
                    egui::Button::new(label).frame_when_inactive(false),
                )
                .clicked()
            {
                self.open(DialogKind::Register {
                    ids: selected_ids.clone(),
                    command,
                });
                ui.close();
            }
        }
        ui.separator();
        ui.add_enabled_ui(editable, |ui| {
            ui.menu_button("Move to account", |ui| {
                for account in self.b.accounts.clone().into_iter().filter(|a| !a.closed) {
                    let command = RegisterCommand::MoveAccount(account.id.clone());
                    if ui
                        .add_enabled(
                            self.b.can_register_command(&selected_ids, command.clone()),
                            egui::Button::new(&account.name),
                        )
                        .clicked()
                    {
                        self.run_register_command(&selected_ids, command);
                        ui.close();
                    }
                }
            });
        });
        ui.separator();
        for (label, command) in [
            ("Match", RegisterCommand::Match),
            ("Unmatch", RegisterCommand::Unmatch),
        ] {
            if ui
                .add_enabled(
                    self.b.can_register_command(&selected_ids, command.clone()),
                    egui::Button::new(label).frame_when_inactive(false),
                )
                .clicked()
            {
                self.run_register_command(&selected_ids, command);
                ui.close();
            }
        }
        ui.separator();
        if ui
            .add_enabled(
                self.b
                    .can_register_command(&selected_ids, RegisterCommand::Delete),
                egui::Button::new("Delete").frame_when_inactive(false),
            )
            .clicked()
        {
            self.run_register_command(&selected_ids, RegisterCommand::Delete);
            ui.close();
        }
    }
}
