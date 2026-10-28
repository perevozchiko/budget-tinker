use super::*;

impl App {
    pub(in crate::ui) fn scheduled_register_panel(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let divider =
            egui::Rect::from_min_max(rect.left_top() - Vec2::new(0., 14.), rect.right_top());
        let drag = ui
            .interact(
                divider,
                ui.id().with("scheduled_divider"),
                egui::Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeVertical)
            .on_hover_text("Drag to resize · Click to hide Scheduled Transactions");
        if drag.dragged() {
            self.preferences.scheduled_panel_height =
                (rect.height() - drag.drag_delta().y).max(140.);
            ui.ctx().request_repaint();
        }
        if drag.drag_stopped() {
            self.save_preferences();
        }
        if drag.clicked() {
            self.register_scheduled_open = false;
        }
        ui.painter().rect_filled(divider, 0., TEAL);
        for (offset, down) in [(-5., true), (5., false)] {
            let c = divider.center() + Vec2::new(offset, 0.);
            let points = if down {
                vec![
                    c + Vec2::new(-4., -2.),
                    c + Vec2::new(4., -2.),
                    c + Vec2::new(0., 2.),
                ]
            } else {
                vec![
                    c + Vec2::new(-4., 2.),
                    c + Vec2::new(4., 2.),
                    c + Vec2::new(0., -2.),
                ]
            };
            ui.painter().add(egui::Shape::convex_polygon(
                points,
                Color32::from_rgb(151, 220, 236),
                Stroke::NONE,
            ));
        }
        ui.painter().rect_filled(rect, 5., classic::CHROME);
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            5.,
            Stroke::new(1_f32, Color32::from_gray(155)),
            egui::StrokeKind::Inside,
        );

        let panel_rect = rect;
        // Paint the unused background without allocating fake scroll content.
        let background =
            egui::Rect::from_min_max(rect.min + Vec2::new(5., 60.), rect.max - Vec2::splat(5.));
        let painter = ui.painter().with_clip_rect(background);
        for index in 0..(background.height().max(0.) / 24.).ceil() as usize {
            let stripe = egui::Rect::from_min_size(
                background.min + Vec2::new(0., index as f32 * 24.),
                Vec2::new(background.width(), 24.),
            );
            painter.rect_filled(
                stripe,
                0.,
                if index % 2 == 0 { PALE } else { Color32::WHITE },
            );
        }

        let mut rows: Vec<_> = self
            .b
            .scheduled
            .iter()
            .filter(|s| {
                self.account
                    .as_ref()
                    .is_none_or(|a| *a == s.transaction.account)
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| a.next_date.cmp(&b.next_date));
        self.scheduled_selected
            .retain(|id| rows.iter().any(|s| &s.id == id));
        let mut draft = self.scheduled_editor.take();
        let mut row_action = RegisterAction::None;
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(panel_rect.shrink(5.))
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(panel_rect.shrink(1.)));
                // Toolbar row
                let (toolbar, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 29.),
                    egui::Sense::hover(),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(toolbar)
                        .layout(Layout::left_to_right(Align::Center)),
                    |ui| {
                        let edit = ui.add_enabled(
                            !self.scheduled_selected.is_empty(),
                            classic::Button::new("Edit Transactions ▾")
                                .small()
                                .icon(classic::Glyph::Check),
                        );
                        egui::Popup::menu(&edit)
                            .frame(classic::register_menu_frame())
                            .show(|ui| {
                                ui.add_enabled_ui(draft.is_none(), |ui| {
                                    self.schedule_menu(ui, &rows)
                                });
                            });
                    },
                );
                // Title with count
                let count_label = if rows.is_empty() {
                    "Scheduled Transactions (0)".into()
                } else {
                    format!("Scheduled Transactions ({})", rows.len())
                };
                let title = egui::Rect::from_center_size(toolbar.center(), Vec2::new(240., 23.));
                ui.painter()
                    .rect_filled(title, 12., Color32::from_gray(224));
                ui.painter().text(
                    title.center(),
                    egui::Align2::CENTER_CENTER,
                    &count_label,
                    FontId::proportional(13.),
                    INK,
                );
                ui.spacing_mut().item_spacing.y = 0.;
                // Scrollable content area
                egui::ScrollArea::horizontal()
                    .id_salt("scheduled_register_table")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let account_width = if self.account.is_none() { 110. } else { 0. };
                        let mut widths = [
                            28.,
                            28.,
                            account_width,
                            100.,
                            118.,
                            180.,
                            160.,
                            140.,
                            80.,
                            80.,
                        ];
                        let extra = (panel_rect.width() - 10. - widths.iter().sum::<f32>()).max(0.);
                        widths[5] += extra * 0.35;
                        widths[6] += extra * 0.5;
                        widths[7] += extra * 0.15;
                        let total = widths.iter().sum();
                        // Header
                        let (header, _) =
                            ui.allocate_exact_size(Vec2::new(total, 26.), egui::Sense::hover());
                        classic::gradient(
                            ui.painter(),
                            header,
                            0.,
                            Color32::from_rgb(106, 123, 130),
                            Color32::from_rgb(65, 83, 91),
                        );
                        let mut x = header.left();
                        for (i, label) in [
                            "",
                            "⚑",
                            "Account",
                            "Date",
                            "Frequency",
                            "Payee",
                            "Category",
                            "Memo",
                            "Outflow",
                            "Inflow",
                        ]
                        .iter()
                        .enumerate()
                        {
                            if widths[i] == 0. {
                                continue;
                            }
                            let cell = egui::Rect::from_min_size(
                                egui::pos2(x, header.top()),
                                Vec2::new(widths[i], header.height()),
                            );
                            if i == 0 {
                                let all = !rows.is_empty()
                                    && rows.iter().all(|s| self.scheduled_selected.contains(&s.id));
                                classic::selection_checkbox(
                                    ui.painter(),
                                    cell.center(),
                                    all,
                                    !all && rows
                                        .iter()
                                        .any(|s| self.scheduled_selected.contains(&s.id)),
                                );
                                if ui
                                    .interact(
                                        cell,
                                        ui.id().with("select_schedules"),
                                        egui::Sense::click(),
                                    )
                                    .clicked()
                                {
                                    for s in &rows {
                                        if all {
                                            self.scheduled_selected.remove(&s.id);
                                        } else {
                                            self.scheduled_selected.insert(s.id.clone());
                                        }
                                    }
                                }
                            } else {
                                ui.painter().text(
                                    cell.center(),
                                    egui::Align2::CENTER_CENTER,
                                    label,
                                    FontId::proportional(12.),
                                    Color32::WHITE,
                                );
                            }
                            x += widths[i];
                        }
                        let list_height = (panel_rect.height() - 29. - 26. - 30. - 24.).max(0.);
                        egui::ScrollArea::vertical()
                            .id_salt("scheduled_register_rows")
                            .max_height(list_height)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                // Editor row
                                if let Some((editing, repeat)) = &mut draft {
                                    let editor_widths = [
                                        widths[0], widths[2], widths[3], widths[5], widths[6],
                                        widths[7], widths[8], widths[9], 0., widths[1],
                                    ];
                                    ui.push_id("scheduled_editor", |ui| {
                                        row_action = editing.show_scheduled(
                                            ui,
                                            &editor_widths,
                                            &self.b,
                                            repeat,
                                        );
                                        editing.autocomplete(&self.b, &self.preferences);
                                    });
                                }
                                // Data rows
                                for (index, s) in rows.iter().enumerate() {
                                    if draft
                                        .as_ref()
                                        .is_some_and(|(d, _)| d.id.as_ref() == Some(&s.id))
                                    {
                                        continue;
                                    }
                                    let (row, _) = ui.allocate_exact_size(
                                        Vec2::new(total, 24.),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        row,
                                        0.,
                                        if self.scheduled_selected.contains(&s.id) {
                                            Color32::from_rgb(255, 246, 168)
                                        } else if index % 2 == 0 {
                                            PALE
                                        } else {
                                            Color32::WHITE
                                        },
                                    );
                                    let response = ui.interact(
                                        row,
                                        ui.id().with(("schedule_row", &s.id)),
                                        egui::Sense::click(),
                                    );
                                    if response.double_clicked() && draft.is_none() {
                                        open_schedule_editor(self, s);
                                    }
                                    if response.clicked() {
                                        let checkbox = response
                                            .interact_pointer_pos()
                                            .is_some_and(|p| p.x < row.left() + widths[0]);
                                        let was_selected = self.scheduled_selected.contains(&s.id);
                                        if !checkbox
                                            && !ui
                                                .input(|i| i.modifiers.command || i.modifiers.shift)
                                        {
                                            self.scheduled_selected.clear();
                                        }
                                        if checkbox && was_selected {
                                            self.scheduled_selected.remove(&s.id);
                                        } else {
                                            self.scheduled_selected.insert(s.id.clone());
                                        }
                                    }
                                    if response.secondary_clicked()
                                        && !self.scheduled_selected.contains(&s.id)
                                    {
                                        self.scheduled_selected.clear();
                                        self.scheduled_selected.insert(s.id.clone());
                                    }
                                    egui::Popup::context_menu(&response)
                                        .frame(classic::register_menu_frame())
                                        .show(|ui| {
                                            ui.add_enabled_ui(draft.is_none(), |ui| {
                                                self.schedule_menu(ui, &rows)
                                            });
                                        });
                                    let a = self
                                        .b
                                        .account(&s.transaction.account)
                                        .expect("validated schedule account");
                                    let date =
                                        chrono::NaiveDate::parse_from_str(&s.next_date, "%Y-%m-%d")
                                            .expect("validated schedule date")
                                            .format("%m/%d/%y")
                                            .to_string();
                                    let mut display = s.transaction.clone();
                                    display.set_date(s.next_date.clone());
                                    let values = [
                                        String::new(),
                                        String::new(),
                                        a.name.clone(),
                                        date,
                                        s.frequency().label().into(),
                                        s.transaction.payee.clone(),
                                        if display.transfer.is_some()
                                            && display.category.is_none()
                                            && display.income_month.is_none()
                                        {
                                            "Transfer".into()
                                        } else if let Some(category) = &display.category {
                                            crate::category_picker::category_label(
                                                ui.ctx(),
                                                &self.b,
                                                category,
                                            )
                                        } else {
                                            i18n::tr(
                                                ui.ctx(),
                                                self.b.transaction_category_name(&display),
                                            )
                                        },
                                        s.transaction.memo.clone(),
                                        if s.transaction.amount < 0 {
                                            i18n::number(-s.transaction.amount, &a.currency)
                                        } else {
                                            String::new()
                                        },
                                        if s.transaction.amount >= 0 {
                                            i18n::number(s.transaction.amount, &a.currency)
                                        } else {
                                            String::new()
                                        },
                                    ];
                                    let mut x = row.left();
                                    for (i, value) in values.iter().enumerate() {
                                        if widths[i] == 0. {
                                            continue;
                                        }
                                        let cell = egui::Rect::from_min_size(
                                            egui::pos2(x, row.top()),
                                            Vec2::new(widths[i], row.height()),
                                        );
                                        if i == 0 {
                                            classic::selection_checkbox(
                                                ui.painter(),
                                                cell.center(),
                                                self.scheduled_selected.contains(&s.id),
                                                false,
                                            );
                                        } else if i == 1 {
                                            crate::register_view::flag(
                                                ui.painter(),
                                                cell,
                                                s.transaction.flag,
                                            );
                                        } else {
                                            ui.painter()
                                                .with_clip_rect(cell.shrink2(Vec2::new(5., 0.)))
                                                .text(
                                                    if i >= 8 {
                                                        cell.right_center() - Vec2::new(6., 0.)
                                                    } else {
                                                        cell.left_center() + Vec2::new(6., 0.)
                                                    },
                                                    if i >= 8 {
                                                        egui::Align2::RIGHT_CENTER
                                                    } else {
                                                        egui::Align2::LEFT_CENTER
                                                    },
                                                    value,
                                                    FontId::proportional(12.),
                                                    INK,
                                                );
                                        }
                                        x += widths[i];
                                    }
                                }
                            });
                        // Actions stay outside the vertical list.

                        if draft.is_none() {
                            let (row, _) =
                                ui.allocate_exact_size(Vec2::new(total, 24.), egui::Sense::hover());
                            ui.painter().rect_filled(row, 0., PALE);
                            ui.scope_builder(
                                egui::UiBuilder::new()
                                    .max_rect(row.shrink2(Vec2::new(4., 2.)))
                                    .layout(Layout::left_to_right(Align::Center)),
                                |ui| {
                                    if ui
                                        .add(
                                            classic::Button::new("Schedule a new transaction")
                                                .small()
                                                .icon(classic::Glyph::Add),
                                        )
                                        .clicked()
                                    {
                                        if self.transaction_editor.is_some() {
                                            self.notify(
                                                "Сначала сохраните или отмените текущую строку",
                                                true,
                                            );
                                        } else if draft.is_none() {
                                            match RegisterDraft::new(
                                                &self.b,
                                                self.account.as_deref(),
                                                None,
                                            ) {
                                                Ok(editing) => {
                                                    draft = Some((editing, Repeat::Monthly))
                                                }
                                                Err(e) => self.notify(e, true),
                                            }
                                            ui.ctx().request_repaint();
                                        }
                                    }
                                    if ui
                                        .add(
                                            classic::Button::new("Schedule a transfer")
                                                .small()
                                                .icon(classic::Glyph::Transfer),
                                        )
                                        .clicked()
                                    {
                                        if self.transaction_editor.is_some() {
                                            self.notify(
                                                "Сначала сохраните или отмените текущую строку",
                                                true,
                                            );
                                        } else if draft.is_none() {
                                            match RegisterDraft::new(
                                                &self.b,
                                                self.account.as_deref(),
                                                None,
                                            ) {
                                                Ok(mut editing) => {
                                                    editing.transfer_to = Some(String::new());
                                                    draft = Some((editing, Repeat::Monthly));
                                                }
                                                Err(error) => self.notify(error, true),
                                            }
                                            ui.ctx().request_repaint();
                                        }
                                    }
                                },
                            );
                        }
                        // FIX: do NOT fill empty space with dummy stripes.
                        // The scroll area will naturally be as tall as its content.
                    });
            },
        );
        // Handle editor actions
        if let Some((mut editing, repeat)) = draft {
            self.finish_category_creation(&mut editing);
            match row_action {
                RegisterAction::Cancel => {}
                RegisterAction::Save | RegisterAction::SaveAnother => {
                    let mut next = self.b.clone();
                    let result = save_scheduled_draft(&mut next, &editing, repeat);
                    match result.and_then(|_| self.commit(next)) {
                        Ok(()) => {
                            if row_action == RegisterAction::SaveAnother {
                                let mut next_draft =
                                    RegisterDraft::new(&self.b, Some(&editing.account), None)
                                        .expect("existing account");
                                next_draft.date = editing.date;
                                next_draft.transfer_to = editing.transfer_to.map(|_| String::new());
                                self.scheduled_editor = Some((next_draft, repeat));
                            } else {
                                // Clear editor after saving
                                self.scheduled_editor = None;
                            }
                            ui.ctx().request_repaint();
                        }
                        Err(error) => {
                            editing.error = error;
                            self.scheduled_editor = Some((editing, repeat));
                        }
                    }
                }
                RegisterAction::None => self.scheduled_editor = Some((editing, repeat)),
            }
        }
    }
}
