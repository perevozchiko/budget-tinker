use super::*;

impl App {
    pub(in crate::ui) fn accounts(&mut self, ui: &mut egui::Ui) {
        self.reconciliation_panel(ui);
        let reconciling = self
            .reconciliation
            .as_ref()
            .filter(|s| self.account.as_ref() == Some(&s.account))
            .is_some();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.;
            let count = self
                .b
                .transactions
                .iter()
                .filter(|t| self.selected.contains(&t.id))
                .count();
            let idle = self.transaction_editor.is_none();
            let label = match count {
                0 => "Edit Transactions ▾".to_owned(),
                1 => "Edit (1) Transaction ▾".to_owned(),
                n => format!("Edit ({n}) Transactions ▾"),
            };
            let edit = ui.add_enabled(
                idle && count > 0,
                classic::Button::new(label).icon(classic::Glyph::Check),
            );
            egui::Popup::menu(&edit)
                .frame(classic::register_menu_frame())
                .show(|ui| {
                    self.register_edit_menu(ui, false);
                });
            if ui
                .add(classic::Button::new("Импорт").icon(classic::Glyph::Import))
                .clicked()
            {
                self.open(DialogKind::Import);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(
                            self.account
                                .as_ref()
                                .and_then(|id| self.b.account(id).ok())
                                .map(|a| format!("Search {}", a.name))
                                .unwrap_or_else(|| "Search All Accounts".into()),
                        )
                        .desired_width(190.),
                );
                if !reconciling && self.register_timeframe.show(ui) {
                    self.selected.clear();
                }
                if !reconciling
                    && crate::register_view::reconciled_buttons(
                        ui,
                        &mut self.preferences.hide_reconciled,
                        !self.search.trim().is_empty(),
                    )
                {
                    self.selected.clear();
                    self.save_preferences();
                }
            });
        });
        ui.add_space(6.);
        let mut rows: Vec<_> = self
            .b
            .transactions
            .iter()
            .filter(|t| {
                self.account.as_ref().is_none_or(|a| *a == t.account)
                    && (if reconciling {
                        let session = self.reconciliation.as_ref().unwrap();
                        !t.reconciled && t.date <= session.date
                    } else {
                        crate::register_search::shows_reconciled(
                            t,
                            self.preferences.hide_reconciled,
                            &self.search,
                        ) && self.register_timeframe.includes(&t.date)
                    })
                    && crate::register_search::matches(&self.b, t, &self.search)
            })
            .cloned()
            .collect();
        let search_total = (!self.search.trim().is_empty()).then(|| {
            (
                rows.len(),
                crate::register_search::total(&self.b, rows.iter(), self.account.is_some()).ok(),
            )
        });
        let selected_rows: Vec<_> = rows
            .iter()
            .filter(|t| self.selected.contains(&t.id))
            .collect();
        let selected_total = (!selected_rows.is_empty()).then(|| {
            (
                selected_rows.len(),
                crate::register_search::total(
                    &self.b,
                    selected_rows.into_iter(),
                    self.account.is_some(),
                )
                .ok(),
            )
        });
        if search_total.is_some() {
            let (banner, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.), egui::Sense::hover());
            classic::gradient(
                ui.painter(),
                banner,
                0.,
                Color32::from_rgb(230, 242, 193),
                Color32::from_rgb(211, 230, 155),
            );
            let clear = egui::Rect::from_min_size(
                egui::pos2(banner.right() - 104., banner.top()),
                Vec2::new(104., 24.),
            );
            ui.painter()
                .with_clip_rect(egui::Rect::from_min_max(banner.min, clear.left_bottom()))
                .text(
                    banner.left_center() + Vec2::new(8., 0.),
                    egui::Align2::LEFT_CENTER,
                    format!("Showing {} results for \"{}\"", rows.len(), self.search),
                    FontId::proportional(12.),
                    Color32::from_rgb(58, 86, 23),
                );
            let response = ui
                .interact(
                    clear,
                    ui.id().with("clear_register_search"),
                    egui::Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            ui.painter().text(
                clear.center(),
                egui::Align2::CENTER_CENTER,
                "Clear Search ×",
                FontId::proportional(12.),
                Color32::from_rgb(58, 86, 23),
            );
            if response.clicked() {
                self.search.clear();
                ui.ctx().request_repaint();
            }
        }
        let mut draft = self.transaction_editor.take();
        let mut row_action = RegisterAction::None;
        if let Some(edit) = draft.as_ref().and_then(|d| d.id.as_ref())
            && !rows.iter().any(|t| t.id == *edit)
            && let Some(t) = self.b.transactions.iter().find(|t| t.id == *edit)
        {
            rows.push(t.clone());
        }
        let schedule_height = if self.register_scheduled_open {
            self.preferences.scheduled_height(ui.available_height())
        } else {
            0.
        };
        let schedule_space = if self.register_scheduled_open {
            schedule_height + 14.
        } else {
            0.
        };
        let table_height = (ui.available_height() - 74. - schedule_space).max(0.);
        let table_width = ui
            .available_width()
            .max(if reconciling { 703. } else { 850. });
        let account_column = if self.account.is_none() { 124. } else { 0. };
        let mut widths = [
            28.,
            account_column,
            83.,
            if reconciling { 150. } else { 170. },
            if reconciling { 125. } else { 150. },
            if reconciling { 100. } else { 140. },
            79.,
            79.,
            31.,
            28.,
        ];
        let extra = (table_width - widths.iter().sum::<f32>()).max(0.);
        widths[3] += extra * 0.45;
        widths[4] += extra * 0.28;
        widths[5] += extra * 0.27;
        let total: f32 = widths.iter().sum();
        egui::ScrollArea::horizontal()
            .id_salt("register_horizontal")
            .max_height(table_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                let mut reveal_editor = false;
                let (header, _) =
                    ui.allocate_exact_size(Vec2::new(total, 29.), egui::Sense::hover());
                classic::gradient(
                    ui.painter(),
                    header,
                    0.,
                    Color32::from_rgb(106, 123, 130),
                    Color32::from_rgb(65, 83, 91),
                );
                let mut x = header.left();
                let labels = [
                    "",
                    "Счёт",
                    "Дата",
                    "Получатель",
                    "Категория",
                    "Заметка",
                    "Расход",
                    "Приход",
                    "C",
                    "",
                ];
                for i in crate::register_view::ORDER {
                    let label = labels[i];
                    if widths[i] <= 0. {
                        continue;
                    }
                    let cell = egui::Rect::from_min_size(
                        egui::pos2(x, header.top()),
                        Vec2::new(widths[i], 29.),
                    );
                    let header_response = ui.interact(
                        cell,
                        ui.id().with(("register_header", i)),
                        egui::Sense::click(),
                    );
                    if i != 0 && header_response.clicked() {
                        reveal_editor = true;
                        if self.register_sort == i {
                            self.register_ascending = !self.register_ascending;
                        } else {
                            self.register_sort = i;
                            self.register_ascending = true;
                        }
                    }
                    if i == 0 {
                        let all =
                            !rows.is_empty() && rows.iter().all(|t| self.selected.contains(&t.id));
                        let some = rows.iter().any(|t| self.selected.contains(&t.id));
                        classic::selection_checkbox(ui.painter(), cell.center(), all, some && !all);
                        if header_response.clicked() {
                            for t in &rows {
                                if all {
                                    self.selected.remove(&t.id);
                                } else {
                                    self.selected.insert(t.id.clone());
                                }
                            }
                        }
                    } else if i == 8 {
                        classic::cleared_badge(ui.painter(), cell.center(), false, false);
                    } else if i == 9 {
                        crate::register_view::flag(ui.painter(), cell, None);
                    } else {
                        ui.painter().l_text(
                            cell.center(),
                            egui::Align2::CENTER_CENTER,
                            if self.register_sort == i {
                                format!(
                                    "{} {}",
                                    i18n::tr(ui.ctx(), label),
                                    if self.register_ascending {
                                        "▲"
                                    } else {
                                        "▼"
                                    }
                                )
                            } else {
                                i18n::tr(ui.ctx(), label)
                            },
                            FontId::proportional(12.),
                            Color32::WHITE,
                        );
                    }
                    ui.painter().line_segment(
                        [cell.right_top(), cell.right_bottom()],
                        Stroke::new(0.5_f32, Color32::from_white_alpha(55)),
                    );
                    x += widths[i];
                }
                // Sort once from document order after handling header clicks. Sorting an
                // already sorted list would change equal-key row positions next frame.
                crate::register_view::sort(
                    &self.b,
                    &mut rows,
                    self.register_sort,
                    self.register_ascending,
                );
                egui::ScrollArea::vertical()
                    .id_salt("register_rows")
                    .max_height((table_height - 59.).max(60.))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(total);
                        let row_count =
                            (rows.len() + 1).max(((table_height - 59.) / 24.).ceil() as usize);
                        for index in 0..row_count {
                            let edit_here = draft.as_ref().is_some_and(|d| match &d.id {
                                Some(id) => rows.get(index).is_some_and(|t| t.id == *id),
                                None => index == rows.len(),
                            });
                            if edit_here {
                                let editing = draft.as_mut().unwrap();
                                let editor = ui.push_id("register_editor", |ui| {
                                    editing.show(ui, &widths, &self.b)
                                });
                                row_action = editor.inner;
                                if reveal_editor {
                                    ui.scroll_to_rect(editor.response.rect, Some(Align::Center));
                                }
                                editing.autocomplete(&self.b, &self.preferences);
                                continue;
                            }
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(total, 24.), egui::Sense::hover());
                            let row = rows.get(index);
                            let selected = row.is_some_and(|t| self.selected.contains(&t.id));
                            ui.painter().rect_filled(
                                rect,
                                0.,
                                if selected {
                                    Color32::from_rgb(255, 246, 168)
                                } else if index % 2 == 0 {
                                    PALE
                                } else {
                                    Color32::WHITE
                                },
                            );
                            let Some(t) = row else {
                                if index == 2 && rows.is_empty() {
                                    ui.painter().l_text(
                                        rect.left_center() + Vec2::new(18., 0.),
                                        egui::Align2::LEFT_CENTER,
                                        "Добавьте операцию или импортируйте банковский CSV",
                                        FontId::proportional(13.),
                                        INK,
                                    );
                                }
                                continue;
                            };
                            let a = self.b.account(&t.account).unwrap().clone();
                            let category = if t.transfer.is_some()
                                && t.category.is_none()
                                && t.income_month.is_none()
                            {
                                "Перевод".into()
                            } else if !t.splits.is_empty() {
                                "Несколько категорий".into()
                            } else {
                                self.b.transaction_category_name(t)
                            };
                            let values = [
                                String::new(),
                                a.name.clone(),
                                crate::register_view::date(&t.date),
                                t.payee.clone(),
                                category,
                                t.memo.clone(),
                                if t.amount < 0 {
                                    i18n::number(-t.amount, &a.currency)
                                } else {
                                    String::new()
                                },
                                if t.amount >= 0 {
                                    i18n::number(t.amount, &a.currency)
                                } else {
                                    String::new()
                                },
                                if t.reconciled {
                                    "Reconciled"
                                } else if t.cleared {
                                    "C"
                                } else {
                                    "○"
                                }
                                .into(),
                                String::new(),
                            ];
                            let mut x = rect.left();
                            for i in crate::register_view::ORDER {
                                let value = &values[i];
                                if widths[i] <= 0. {
                                    continue;
                                }
                                let cell = egui::Rect::from_min_size(
                                    egui::pos2(x, rect.top()),
                                    Vec2::new(widths[i], rect.height()),
                                );
                                x += widths[i];
                                let response = ui
                                    .interact(cell, ui.id().with((&t.id, i)), egui::Sense::click())
                                    .on_hover_text(if self.b.pending_import(&t.id) {
                                        "Imported — awaiting approval"
                                    } else {
                                        ""
                                    });
                                if draft.is_none() {
                                    if response.secondary_clicked()
                                        && !self.selected.contains(&t.id)
                                    {
                                        self.selected.clear();
                                        self.selected.insert(t.id.clone());
                                    }
                                    egui::Popup::context_menu(&response)
                                        .frame(classic::register_menu_frame())
                                        .show(|ui| self.register_edit_menu(ui, true));
                                }
                                if i == 0 {
                                    classic::selection_checkbox(
                                        ui.painter(),
                                        cell.center(),
                                        selected,
                                        false,
                                    );
                                    if response.clicked() && !self.selected.remove(&t.id) {
                                        self.selected.insert(t.id.clone());
                                    }
                                } else if i == 9 {
                                    crate::register_view::flag(ui.painter(), cell, t.flag);
                                    if draft.is_none() {
                                        if response.clicked() && !self.selected.contains(&t.id) {
                                            self.selected.clear();
                                            self.selected.insert(t.id.clone());
                                        }
                                        egui::Popup::menu(&response)
                                            .frame(
                                                egui::Frame::new()
                                                    .fill(Color32::WHITE)
                                                    .stroke(Stroke::new(
                                                        1_f32,
                                                        Color32::from_rgb(42, 92, 241),
                                                    ))
                                                    .inner_margin(5),
                                            )
                                            .show(|ui| {
                                                if let Some(flag) = crate::register_view::flag_menu(
                                                    ui,
                                                    self.selected.len(),
                                                ) {
                                                    let ids = self.selected.clone();
                                                    self.run_register_command(
                                                        &ids,
                                                        RegisterCommand::Flag(flag),
                                                    );
                                                }
                                            });
                                    }
                                } else if i == 8 {
                                    classic::cleared_badge(
                                        ui.painter(),
                                        cell.center(),
                                        t.cleared,
                                        t.reconciled,
                                    );
                                    response.clone().on_hover_text(if t.reconciled {
                                        "Reconciled"
                                    } else if t.cleared {
                                        "Cleared"
                                    } else {
                                        "Uncleared"
                                    });
                                    if response.clicked() && !t.reconciled {
                                        self.run_register_command(
                                            &HashSet::from([t.id.clone()]),
                                            RegisterCommand::Cleared(!t.cleared),
                                        );
                                    }
                                } else {
                                    let right = (6..=7).contains(&i);
                                    let color = if i == 8 && t.cleared {
                                        GREEN
                                    } else if i == 9 {
                                        TEAL
                                    } else {
                                        INK
                                    };
                                    ui.painter()
                                        .with_clip_rect(cell.shrink2(Vec2::new(6., 0.)))
                                        .l_text(
                                            if right {
                                                cell.right_center() - Vec2::new(7., 0.)
                                            } else {
                                                cell.left_center() + Vec2::new(7., 0.)
                                            },
                                            if right {
                                                egui::Align2::RIGHT_CENTER
                                            } else {
                                                egui::Align2::LEFT_CENTER
                                            },
                                            value,
                                            FontId::proportional(12.),
                                            color,
                                        );
                                    if draft.is_none()
                                        && i > 1
                                        && response.double_clicked()
                                        && t.transfer.is_none()
                                        && !t.reconciled
                                    {
                                        self.start_register_edit(Some(&t.id));
                                    }
                                    if !value.is_empty() {
                                        response.l_on_hover_text(value);
                                    }
                                }
                            }
                        }
                    });
                let (rect, _) = ui.allocate_exact_size(Vec2::new(total, 30.), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0., PALE);
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink2(Vec2::new(4., 3.)))
                        .layout(Layout::left_to_right(Align::Center)),
                    |ui| {
                        let idle = draft.is_none()
                            && self.transaction_editor.is_none()
                            && self.scheduled_editor.is_none();
                        if ui
                            .add_enabled(
                                idle,
                                classic::Button::new("Добавить транзакцию")
                                    .small()
                                    .icon(classic::Glyph::Add),
                            )
                            .clicked()
                        {
                            self.start_register_edit(None);
                        }
                        if ui
                            .add_enabled(
                                idle,
                                classic::Button::new("Сделать перевод")
                                    .small()
                                    .icon(classic::Glyph::Transfer),
                            )
                            .clicked()
                        {
                            self.open(DialogKind::Transfer);
                        }
                    },
                );
            });
        if let Some(mut editing) = draft {
            self.finish_category_creation(&mut editing);
            match row_action {
                RegisterAction::Cancel => {}
                RegisterAction::Save | RegisterAction::SaveAnother => {
                    let result = editing.build(&self.b).and_then(|t| {
                        let mut next = self.b.clone();
                        next.put_transaction(t)?;
                        self.commit(next)
                    });
                    match result {
                        Ok(()) => {
                            if row_action == RegisterAction::SaveAnother {
                                let mut next =
                                    RegisterDraft::new(&self.b, Some(&editing.account), None)
                                        .expect("existing account");
                                next.date = editing.date;
                                self.transaction_editor = Some(next);
                            }
                            ui.ctx().request_repaint();
                        }
                        Err(e) => {
                            editing.error = e;
                            self.transaction_editor = Some(editing);
                        }
                    }
                }
                RegisterAction::None => {
                    self.transaction_editor = Some(editing);
                }
            }
        }
        let footer = egui::Rect::from_min_size(
            egui::pos2(
                ui.min_rect().left(),
                ui.max_rect().bottom() - schedule_space - 64.,
            ),
            Vec2::new(ui.available_width(), 64.),
        );
        let account = self.account.as_ref().and_then(|id| self.b.account(id).ok());
        let statement_date = self
            .reconciliation
            .as_ref()
            .filter(|s| self.account.as_ref() == Some(&s.account))
            .map(|s| s.date.as_str());
        let totals = |cleared: bool| -> Result<i64> {
            self.b
                .accounts
                .iter()
                .filter(|a| !a.closed)
                .try_fold(0_i64, |sum, a| {
                    let value = self.b.balance(&a.id, None, cleared);
                    Ok(sum + self.b.to_base(value, &a.currency, &current_month())?.0)
                })
        };
        let cleared = account
            .map(|a| Ok(self.b.balance(&a.id, statement_date, true)))
            .unwrap_or_else(|| totals(true));
        let working = account
            .map(|a| Ok(self.b.balance(&a.id, statement_date, false)))
            .unwrap_or_else(|| totals(false));
        let currency = account.map(|a| a.currency.as_str()).unwrap_or(&self.b.base);
        let action = register_footer::show(
            ui,
            footer,
            register_footer::Summary {
                currency,
                cleared: cleared.ok(),
                working: working.ok(),
                search: search_total,
                selected: selected_total,
            },
            account.is_some_and(|a| !a.closed) && self.reconciliation.is_none(),
        );
        match action {
            register_footer::Action::Scheduled => {
                self.register_scheduled_open = !self.register_scheduled_open
            }
            register_footer::Action::Reconcile => {
                if let Some(a) = self.account.clone() {
                    self.open(DialogKind::Reconcile(a));
                }
            }
            register_footer::Action::Cleared => {
                self.search = crate::register_search::CLEARED.into();
                self.selected.clear();
            }
            register_footer::Action::Uncleared => {
                self.search = crate::register_search::UNCLEARED.into();
                self.selected.clear();
            }
            register_footer::Action::None => {}
        }
        if schedule_space > 0. {
            let rect = egui::Rect::from_min_max(
                egui::pos2(footer.left(), footer.bottom() + 14.),
                ui.max_rect().right_bottom(),
            );
            self.scheduled_register_panel(ui, rect);
        }
    }
}
