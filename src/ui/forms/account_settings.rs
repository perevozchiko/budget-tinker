use super::*;

impl App {
    pub(super) fn account_settings_form(&mut self, ctx: &egui::Context, mut d: Dialog) {
        let DialogKind::AccountSettings(account_id) = &d.kind else {
            unreachable!();
        };
        let account_id = account_id.clone();
        let account = self.b.account(&account_id).expect("existing account");
        let closed = account.closed;
        let on_budget = account.on_budget;
        let width = 374.;
        let mut save = false;
        let mut cancel = false;
        let mut adjust = false;
        let mut modal = chrome::modal(ctx, "dialog").backdrop_color(Color32::TRANSPARENT);
        let mut pointer = None;
        if let Some(anchor) = d.anchor {
            let screen = ctx.screen_rect();
            let x = anchor.x.clamp(
                screen.left() + 8.,
                (screen.right() - width - 8.).max(screen.left() + 8.),
            );
            // Reserve room for validation text as well as the compact form.
            let y = anchor.y.min(screen.bottom() - 390.).max(screen.top() + 20.);
            if y == anchor.y {
                pointer = Some(egui::pos2(x, y));
            }
            modal = modal.area(egui::Modal::default_area(egui::Id::new("dialog")).anchor(
                egui::Align2::LEFT_TOP,
                Vec2::new(x - screen.left(), y - screen.top()),
            ));
        }
        let response = modal.show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            chrome::header(ui, "Edit Account", width);
            Frame::new()
                .fill(Color32::WHITE)
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(width - 24.);
                    ui.spacing_mut().item_spacing = Vec2::new(8., 12.);
                    ui.spacing_mut().interact_size.y = 27.;
                    ui.visuals_mut().widgets.inactive.bg_stroke =
                        Stroke::new(1_f32, classic::BORDER);
                    egui::Grid::new("edit_account_fields")
                        .num_columns(2)
                        .spacing([8., 12.])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Name:").font(crate::palette::bold(12.)));
                            account_input(ui, &mut d.fields[0], false, &mut d.focus);
                            ui.end_row();
                            ui.label(RichText::new("Note:").font(crate::palette::bold(12.)));
                            account_input(ui, &mut d.fields[2], true, &mut false);
                            ui.end_row();
                            ui.label(RichText::new("Type:").font(crate::palette::bold(12.)));
                            account_type(ui, &mut d.fields[1], 300.);
                            ui.end_row();
                        });
                    ui.add_space(2.);
                    ui.separator();
                    ui.horizontal(|ui| {
                        let r = ui.add_enabled(!closed, classic::Button::new("Adjust Balance"));
                        remember(ui, "adjust", r.rect);
                        adjust = r.clicked();
                        let r = classic::button(
                            ui,
                            if d.second {
                                "Make this Account Off-Budget"
                            } else {
                                "Make this Account On-Budget"
                            },
                        );
                        remember(ui, "budget", r.rect);
                        if r.clicked() {
                            d.second = !d.second;
                        }
                    });
                    if d.second != on_budget {
                        ui.label(
                            RichText::new(
                                "This changes the budget across the account's entire history.",
                            )
                            .size(11.)
                            .color(RED),
                        );
                    }
                    if !d.error.is_empty() {
                        ui.label(RichText::new(i18n::tr(ctx, &d.error)).size(12.).color(RED));
                    }
                });
            let footer = Frame::new()
                .fill(Color32::from_gray(232))
                .inner_margin(Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    ui.set_width(width - 24.);
                    ui.horizontal(|ui| {
                        let link = ui.allocate_ui_with_layout(
                            Vec2::new(145., 26.),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.spacing_mut().item_spacing.x = 4.;
                                action_link(
                                    ui,
                                    if closed {
                                        "Reopen Account"
                                    } else {
                                        "Close Account"
                                    },
                                    !closed,
                                )
                            },
                        );
                        remember(ui, "close", link.response.rect);
                        link.response.on_hover_text(if closed {
                            "Reopen this account"
                        } else {
                            "The account balance must be zero before closing it"
                        });
                        if link.inner {
                            d.check = !closed;
                            save = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 6.;
                            let r =
                                ui.add_enabled(!d.fields[0].trim().is_empty(), account_done_button);
                            remember(ui, "done", r.rect);
                            save |= r.clicked();
                            let r = classic::button(ui, "Cancel");
                            remember(ui, "cancel", r.rect);
                            cancel = r.clicked();
                        });
                    });
                });
            ui.painter().hline(
                footer.response.rect.x_range(),
                footer.response.rect.top(),
                Stroke::new(1_f32, Color32::from_gray(178)),
            );
        });
        remember_rect(ctx, "popup", response.response.rect);
        if pointer.is_some() {
            popup_pointer(
                &response.response,
                Color32::from_gray(232),
                Color32::from_gray(178),
            );
        }
        if cancel || response.should_close() {
            return;
        }
        if adjust {
            self.open(DialogKind::AdjustBalance(account_id));
            self.dialog.as_mut().expect("adjustment dialog").parent = Some(Box::new(d));
            return;
        }
        if save {
            match self.save_dialog(&d) {
                Ok(()) => return,
                Err(error) => {
                    d.error = error;
                    d.check = closed;
                }
            }
        }
        self.dialog = Some(d);
    }
}

fn remember(ui: &egui::Ui, control: &str, rect: egui::Rect) {
    remember_rect(ui.ctx(), control, rect);
}

fn account_input(ui: &mut egui::Ui, value: &mut String, multiline: bool, focus: &mut bool) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(300., if multiline { 68. } else { 27. }),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 3., Color32::WHITE);
    classic::gradient(
        ui.painter(),
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + 12.)),
        3.,
        Color32::from_gray(212),
        Color32::WHITE,
    );
    ui.painter().rect_stroke(
        rect,
        3.,
        Stroke::new(1_f32, classic::BORDER),
        egui::StrokeKind::Inside,
    );
    let edit = if multiline {
        egui::TextEdit::multiline(value)
    } else {
        egui::TextEdit::singleline(value)
    };
    let response = ui.put(
        rect,
        edit.frame(false).font(FontId::proportional(13.)).margin(6),
    );
    if *focus {
        response.request_focus();
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(value.chars().count()),
                )));
            state.store(ui.ctx(), response.id);
        }
        *focus = false;
    }
}

fn account_done_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(68., 27.), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Done")
    });
    let (top, bottom) = if response.is_pointer_button_down_on() {
        (
            Color32::from_rgb(167, 194, 141),
            Color32::from_rgb(236, 246, 223),
        )
    } else {
        (
            Color32::from_rgb(246, 251, 239),
            Color32::from_rgb(184, 211, 158),
        )
    };
    classic::gradient(ui.painter(), rect, 3., top, bottom);
    ui.painter().rect_stroke(
        rect,
        3.,
        Stroke::new(1_f32, classic::BORDER),
        egui::StrokeKind::Inside,
    );
    let center = rect.left_center() + Vec2::new(13., 0.);
    ui.painter()
        .circle_filled(center, 7., Color32::from_rgb(71, 158, 0));
    ui.painter().add(egui::Shape::line(
        vec![
            center + Vec2::new(-3., 0.),
            center + Vec2::new(-1., 3.),
            center + Vec2::new(4., -3.),
        ],
        Stroke::new(1.5_f32, Color32::WHITE),
    ));
    ui.painter().text(
        rect.center() + Vec2::new(8., 0.),
        egui::Align2::CENTER_CENTER,
        "Done",
        crate::palette::bold(13.),
        if ui.is_enabled() { INK } else { Color32::GRAY },
    );
    response
}

#[cfg(test)]
mod tests;

fn remember_rect(ctx: &egui::Context, control: &str, rect: egui::Rect) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new(("account_settings", control)), rect));
}
