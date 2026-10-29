use super::*;

impl Draft {
    pub fn show(&mut self, ui: &mut egui::Ui, widths: &[f32; 10], b: &Budget) -> Action {
        self.show_inner(ui, widths, b, None)
    }
    pub fn show_scheduled(
        &mut self,
        ui: &mut egui::Ui,
        widths: &[f32; 10],
        b: &Budget,
        repeat: &mut Repeat,
    ) -> Action {
        self.show_inner(ui, widths, b, Some(repeat))
    }
    pub(super) fn show_inner(
        &mut self,
        ui: &mut egui::Ui,
        widths: &[f32; 10],
        b: &Budget,
        repeat: Option<&mut Repeat>,
    ) -> Action {
        let scheduled = repeat.is_some();
        let total: f32 = widths.iter().sum::<f32>() + if scheduled { 118. } else { 0. };
        let mut action = Action::None;
        let (row, _) = ui.allocate_exact_size(Vec2::new(total, 27.), egui::Sense::hover());
        let orange = Color32::from_rgb(255, 162, 47);
        ui.painter().rect_filled(row, 0., Color32::WHITE);
        ui.painter().rect_stroke(
            row.shrink(0.5),
            0.,
            Stroke::new(1_f32, orange),
            egui::StrokeKind::Inside,
        );
        let mut x = row.left();
        let mut cells = [Rect::NOTHING; 10];
        let mut frequency_cell = Rect::NOTHING;
        for index in crate::register_view::ORDER {
            let width = widths[index];
            cells[index] =
                Rect::from_min_size(egui::pos2(x, row.top()), Vec2::new(width, row.height()));
            x += width;
            if scheduled && index == 2 {
                frequency_cell =
                    Rect::from_min_size(egui::pos2(x, row.top()), Vec2::new(118., row.height()));
                x += 118.;
            }
        }
        if let Some(repeat) = repeat {
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(frequency_cell.shrink(2.)),
                |ui| {
                    egui::ComboBox::from_id_salt("schedule_frequency")
                        .width(frequency_cell.width() - 4.)
                        .selected_text(repeat.label())
                        .show_ui(ui, |ui| {
                            for value in Repeat::ALL {
                                ui.selectable_value(repeat, value, value.label());
                            }
                        });
                },
            );
        }
        let prefix = self.id.as_deref().unwrap_or("new_transaction");
        crate::register_view::flag(
            ui.painter(),
            cells[9],
            self.id
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
        );
        let input = |ui: &mut egui::Ui,
                     rect: Rect,
                     value: &mut String,
                     hint: &str,
                     right: bool,
                     id: egui::Id| {
            ui.put(
                rect.shrink2(Vec2::new(2., 2.)),
                egui::TextEdit::singleline(value)
                    .id(id)
                    .font(FontId::proportional(12.))
                    .hint_text(i18n::tr(ui.ctx(), hint))
                    .margin(egui::Margin::symmetric(3, 1))
                    .horizontal_align(if right {
                        egui::Align::RIGHT
                    } else {
                        egui::Align::LEFT
                    })
                    .desired_width(rect.width() - 4.),
            )
        };
        if cells[1].width() > 0. {
            ui.scope_builder(egui::UiBuilder::new().max_rect(cells[1].shrink(2.)), |ui| {
                egui::ComboBox::from_id_salt((prefix, "account"))
                    .width(cells[1].width() - 4.)
                    .selected_text(i18n::tr(
                        ui.ctx(),
                        b.account(&self.account)
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|_| "Счёт".into()),
                    ))
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .show_ui(ui, |ui| {
                        for a in &b.accounts {
                            if !a.closed {
                                ui.l_selectable_value(
                                    &mut self.account,
                                    a.id.clone(),
                                    a.name.clone(),
                                );
                            }
                        }
                    });
            });
        }
        let date_id = ui.id().with((prefix, "date"));
        let calendar_was_open =
            self.date_input.calendar_open() || egui::Popup::is_any_open(ui.ctx());
        let date_response = self.date_input.show(ui, cells[2], date_id, &mut self.date);
        if self.transfer_to.is_none()
            && date_response.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Tab) && !i.modifiers.shift)
        {
            ui.memory_mut(|m| m.request_focus(ui.id().with((prefix, "payee"))));
        }
        let calendar_is_open = egui::Popup::is_any_open(ui.ctx());
        let mut reveal_editor = self.focus;
        if self.focus {
            date_response.request_focus();
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), date_id) {
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.date_input.text().chars().count()),
                    )));
                state.store(ui.ctx(), date_id);
            }
            self.focus = false;
        }
        if let Some(target) = &mut self.transfer_to {
            let source = b.account(&self.account).ok();
            // An account change must not leave an invalid destination selected.
            if !b.accounts.iter().any(|a| {
                a.id == *target
                    && !a.closed
                    && a.id != self.account
                    && source.is_some_and(|source| source.currency == a.currency)
            }) {
                target.clear();
            }
            ui.scope_builder(egui::UiBuilder::new().max_rect(cells[3].shrink(2.)), |ui| {
                egui::ComboBox::from_id_salt((prefix, "transfer_account"))
                    .width(cells[3].width() - 4.)
                    .selected_text(
                        b.account(target)
                            .map(|a| format!("Transfer : {}", a.name))
                            .unwrap_or_else(|_| "Select an account".into()),
                    )
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .show_ui(ui, |ui| {
                        for account in &b.accounts {
                            if !account.closed
                                && account.id != self.account
                                && source.is_some_and(|source| source.currency == account.currency)
                            {
                                ui.selectable_value(
                                    target,
                                    account.id.clone(),
                                    format!("Transfer : {}", account.name),
                                );
                            }
                        }
                    });
            });
            let label = if !self.category.is_empty() {
                crate::category_picker::category_label(ui.ctx(), b, &self.category)
            } else if let Some(next) = self.income_next.filter(|_| valid_date(&self.date)) {
                budgettinker::income::income_label(&month_shift(&self.date[..7], i32::from(next)))
            } else {
                "Transfer".into()
            };
            ui.scope_builder(egui::UiBuilder::new().max_rect(cells[4].shrink(2.)), |ui| {
                ui.add(egui::Label::new(label).truncate());
            });
        } else {
            let (accepted_payee, advance_payee) = crate::payee_picker::show(
                ui,
                cells[3],
                ui.id().with((prefix, "payee")),
                &mut self.payee,
                b,
            );
            self.complete_payee |= accepted_payee;
            let memo_id = ui.id().with((prefix, "memo"));
            ui.scope_builder(egui::UiBuilder::new().max_rect(cells[4].shrink(2.)), |ui| {
                if advance_payee {
                    let id = ui
                        .make_persistent_id((prefix, "category"))
                        .with("request_focus");
                    ui.ctx().data_mut(|d| d.insert_temp(id, true));
                }
                let selection = category_combo(
                    ui,
                    (prefix, "category"),
                    &mut self.category,
                    &mut self.income_next,
                    &self.date,
                    !self.splits.is_empty(),
                    b,
                    cells[4].width() - 4.,
                );
                if let Some(selection) = selection {
                    if selection.advance {
                        ui.memory_mut(|memory| memory.request_focus(memo_id));
                    }
                    match selection.choice {
                        crate::category_picker::Choice::Split if self.splits.is_empty() => {
                            self.splits.push((self.category.clone(), String::new()));
                            reveal_editor = true;
                        }
                        crate::category_picker::Choice::Create(request) => {
                            self.pending_category = Some((request, None));
                        }
                        _ => {}
                    }
                }
            });
        }
        input(
            ui,
            cells[5],
            &mut self.memo,
            "заметка",
            false,
            ui.id().with((prefix, "memo")),
        );
        input(
            ui,
            cells[6],
            &mut self.outflow,
            "расход",
            true,
            ui.id().with((prefix, "outflow")),
        );
        input(
            ui,
            cells[7],
            &mut self.inflow,
            "приход",
            true,
            ui.id().with((prefix, "inflow")),
        );
        if !scheduled {
            crate::classic::cleared_badge(ui.painter(), cells[8].center(), self.cleared, false);
            if ui
                .interact(
                    cells[8],
                    ui.id().with((prefix, "cleared")),
                    egui::Sense::click(),
                )
                .clicked()
            {
                self.cleared = !self.cleared;
            }
        }
        let (footer, _) = ui.allocate_exact_size(Vec2::new(total, 32.), egui::Sense::hover());
        ui.painter()
            .rect_filled(footer, 0., Color32::from_rgb(255, 248, 237));
        let p = ui.painter().with_clip_rect(footer);
        for i in 0..(total / 18.).ceil() as i32 {
            let x = footer.left() + i as f32 * 18.;
            p.line_segment(
                [
                    egui::pos2(x, footer.bottom()),
                    egui::pos2(x + 32., footer.top()),
                ],
                Stroke::new(6_f32, Color32::from_rgb(255, 234, 205)),
            );
        }
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(footer.shrink2(Vec2::new(4., 3.)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                if classic::small_button(ui, "Отмена").clicked() {
                    action = Action::Cancel;
                }
                if ui
                    .add(classic::Button::new("Сохранить").small().icon(Glyph::Check))
                    .clicked()
                {
                    action = Action::Save;
                }
                if ui
                    .add(
                        classic::Button::new("Сохранить и добавить ещё")
                            .small()
                            .icon(Glyph::Check),
                    )
                    .clicked()
                {
                    action = Action::SaveAnother;
                }
                if self.transfer_to.is_none()
                    && classic::small_button(ui, "Разбить по категориям").clicked()
                {
                    self.splits.push((self.category.clone(), String::new()));
                    reveal_editor = true;
                }
            },
        );
        let mut remove = None;
        self.split_details
            .resize_with(self.splits.len(), Default::default);
        for (i, (category, value)) in self.splits.iter_mut().enumerate() {
            let details = &mut self.split_details[i];
            ui.horizontal(|ui| {
                ui.add_space(
                    widths[..3].iter().sum::<f32>() + widths[9] + if scheduled { 118. } else { 0. },
                );
                let payee_width = (widths[3] - 40.).max(75.);
                let mut advance_payee = false;
                if let Some(target) = &details.transfer_to {
                    ui.add_sized(
                        [payee_width, 22.],
                        egui::Label::new(
                            b.account(target)
                                .map(|a| format!("Transfer : {}", a.name))
                                .unwrap_or_else(|_| "Select an account".into()),
                        )
                        .truncate(),
                    );
                } else {
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(payee_width, 24.), egui::Sense::hover());
                    let (accepted, advance) = crate::payee_picker::show(
                        ui,
                        rect,
                        ui.id().with((prefix, "split_payee", i)),
                        &mut details.payee,
                        b,
                    );
                    if accepted {
                        details.payee = b.imported_payee(details.payee.trim());
                    }
                    advance_payee = advance;
                }
                ui.push_id((prefix, "split_transfer", i), |ui| {
                    ui.menu_button("↔", |ui| {
                        if ui
                            .selectable_label(details.transfer_to.is_none(), "Payee")
                            .clicked()
                        {
                            details.transfer_to = None;
                            details.transfer = None;
                            ui.close();
                        }
                        for a in &b.accounts {
                            if !a.closed
                                && a.id != self.account
                                && b.account(&self.account)
                                    .is_ok_and(|source| source.currency == a.currency)
                                && ui
                                    .selectable_label(
                                        details.transfer_to.as_ref() == Some(&a.id),
                                        format!("Transfer : {}", a.name),
                                    )
                                    .clicked()
                            {
                                details.transfer_to = Some(a.id.clone());
                                if b.account(&self.account)
                                    .is_ok_and(|source| source.on_budget == a.on_budget)
                                {
                                    category.clear();
                                    details.income_next = None;
                                }
                                ui.close();
                            }
                        }
                    });
                });
                let is_internal = details.transfer_to.as_ref().is_some_and(|target| {
                    b.account(&self.account).is_ok_and(|a| {
                        b.account(target)
                            .is_ok_and(|other| a.on_budget == other.on_budget)
                    })
                });
                let selected = if let Some(next) = details.income_next {
                    crate::category_picker::Choice::Income(next)
                } else if category.is_empty() {
                    crate::category_picker::Choice::Uncategorized
                } else {
                    crate::category_picker::Choice::Category(category.clone())
                };
                let label = if is_internal && category.is_empty() && details.income_next.is_none() {
                    "Transfer".into()
                } else if let Some(next) = details.income_next.filter(|_| valid_date(&self.date)) {
                    budgettinker::income::income_label(&month_shift(
                        &self.date[..7],
                        i32::from(next),
                    ))
                } else {
                    crate::category_picker::category_label(ui.ctx(), b, category)
                };
                let selection = ui
                    .add_enabled_ui(
                        !is_internal || !category.is_empty() || details.income_next.is_some(),
                        |ui| {
                            if advance_payee {
                                let request = ui
                                    .make_persistent_id((prefix, "split", i))
                                    .with("request_focus");
                                ui.ctx().data_mut(|d| d.insert_temp(request, true));
                            }
                            crate::category_picker::show_part(
                                ui,
                                (prefix, "split", i),
                                b,
                                crate::category_picker::Options {
                                    selected,
                                    label,
                                    date: &self.date,
                                    width: widths[4] - 10.,
                                    special: true,
                                },
                            )
                        },
                    )
                    .inner;
                let amount_id = ui.id().with((prefix, "split_amount", i));
                let memo_id = ui.id().with((prefix, "split_memo", i));
                if let Some(selection) = selection {
                    if selection.advance {
                        ui.memory_mut(|m| m.request_focus(memo_id));
                    }
                    match selection.choice {
                        crate::category_picker::Choice::Category(id) => {
                            *category = id;
                            details.income_next = None;
                        }
                        crate::category_picker::Choice::Income(next) => {
                            category.clear();
                            details.income_next = Some(next);
                        }
                        crate::category_picker::Choice::Uncategorized => {
                            category.clear();
                            details.income_next = None;
                        }
                        crate::category_picker::Choice::Create(request) => {
                            self.pending_category = Some((request, Some(i)))
                        }
                        _ => {}
                    }
                }
                ui.add(
                    egui::TextEdit::singleline(&mut details.memo)
                        .id(memo_id)
                        .desired_width((widths[5] - 10.).max(65.))
                        .hint_text("Memo"),
                );
                ui.add(
                    egui::TextEdit::singleline(value)
                        .id(amount_id)
                        .desired_width(100.)
                        .hint_text(i18n::tr(ui.ctx(), "− расход / + доход")),
                );
                if classic::small_button(ui, "×").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            self.splits.remove(i);
            self.split_details.remove(i);
            if i < self.fx.parts.len() {
                self.fx.parts.remove(i);
            }
        }
        if !scheduled && self.transfer_to.is_none() {
            self.show_fx(ui, b);
        }
        if !self.error.is_empty() {
            ui.add(
                egui::Label::new(i18n::widget(
                    ui,
                    egui::RichText::new(&self.error).color(Color32::from_rgb(190, 68, 61)),
                ))
                .wrap(),
            );
            ui.add_space(5.);
        }
        if reveal_editor {
            ui.scroll_to_rect(
                Rect::from_min_max(row.min, egui::pos2(row.right(), ui.min_rect().bottom())),
                Some(egui::Align::Center),
            );
        }
        if ui.is_enabled()
            && !calendar_was_open
            && !calendar_is_open
            && !egui::Popup::is_any_open(ui.ctx())
            && !crate::category_picker::is_open(ui.ctx())
        {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                action = Action::Cancel;
            } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)) {
                action = Action::SaveAnother;
            } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                action = Action::Save;
            }
        }
        action
    }
}
