use super::*;

impl App {
    pub(in crate::ui) fn account_total(&self, accounts: &[Account]) -> Result<i64> {
        accounts.iter().try_fold(0_i64, |total, a| {
            let balance = self.b.balance(&a.id, None, false);
            Ok(total + self.b.to_base(balance, &a.currency, &current_month())?.0)
        })
    }
    pub(in crate::ui) fn sidebar(&mut self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        let ctx = &context;
        let category_popup = self.dialog.as_ref().is_some_and(|d| {
            matches!(
                d.kind,
                DialogKind::Category
                    | DialogKind::NewGroup
                    | DialogKind::EditCategory(_)
                    | DialogKind::EditGroup(_)
                    | DialogKind::Note(_)
            )
        });
        if (self.dialog.is_some() && !category_popup) || self.chrome.overlay.is_some() {
            ui.disable();
        }
        let compact = self.preferences.sidebar_compact;
        let sidebar_width = if compact { 148. } else { 248. };
        ui.spacing_mut().item_spacing.y = 0.;
        let panel_fill = Color32::from_rgba_unmultiplied(0, 32, 75, 85);
        let panel_stroke = Stroke::new(0.7_f32, Color32::from_black_alpha(40));
        Frame::new()
            .fill(panel_fill)
            .stroke(panel_stroke)
            .corner_radius(5)
            .inner_margin(Margin::symmetric(0, 7))
            .show(ui, |ui| {
                ui.set_width(sidebar_width);
                for (kind, page, label) in [
                    (0, Page::Budget, "Бюджет"),
                    (1, Page::Reports, "Отчёты"),
                    (2, Page::Accounts, "Все счета"),
                ] {
                    let active =
                        self.page == page && (page != Page::Accounts || self.account.is_none());
                    let (r, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 36.),
                        egui::Sense::click(),
                    );
                    if active || response.hovered() {
                        ui.painter().rect_filled(
                            r,
                            0.,
                            if active {
                                crate::palette::SELECTED
                            } else {
                                Color32::from_white_alpha(14)
                            },
                        );
                    }
                    let icon = egui::Rect::from_center_size(
                        r.left_center() + Vec2::new(17., 0.),
                        Vec2::splat(25.),
                    );
                    crate::icons::draw(
                        ui.painter(),
                        icon.translate(Vec2::new(0., 1.5)),
                        kind,
                        Color32::from_black_alpha(80),
                    );
                    crate::icons::draw(
                        ui.painter(),
                        icon,
                        kind,
                        [
                            Color32::from_rgb(213, 240, 21),
                            Color32::from_rgb(255, 205, 40),
                            Color32::from_rgb(18, 221, 245),
                        ][kind],
                    );
                    let label = if compact && kind == 2 {
                        "Accounts".into()
                    } else {
                        i18n::tr(ctx, label)
                    };
                    let pos = r.left_center() + Vec2::new(39., 0.);
                    ui.painter().text(
                        pos + Vec2::new(0., 1.),
                        egui::Align2::LEFT_CENTER,
                        &label,
                        crate::palette::bold(14.),
                        Color32::from_black_alpha(100),
                    );
                    ui.painter().text(
                        pos,
                        egui::Align2::LEFT_CENTER,
                        &label,
                        crate::palette::bold(14.),
                        Color32::WHITE,
                    );
                    if kind == 2 && !compact {
                        let total = self
                            .account_total(&self.b.accounts)
                            .map(|v| i18n::money(v, &self.b.base))
                            .unwrap_or_else(|_| "—".into());
                        ui.painter().text(
                            r.right_center() - Vec2::new(8., 0.),
                            egui::Align2::RIGHT_CENTER,
                            total,
                            crate::palette::bold(12.),
                            Color32::WHITE,
                        );
                    }
                    if response.clicked() {
                        self.page = page;
                        self.account = None;
                        self.selected.clear();
                    }
                }
            });
        ui.add_space(8.);
        egui::ScrollArea::vertical()
            .max_height((ui.available_height() - 144.).max(130.))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for group in 0..3 {
                    let accounts: Vec<_> = self
                        .b
                        .accounts
                        .iter()
                        .filter(|a| match group {
                            0 => a.on_budget && !a.closed,
                            1 => !a.on_budget && !a.closed,
                            _ => a.closed,
                        })
                        .cloned()
                        .collect();
                    if group > 0 && accounts.is_empty() {
                        continue;
                    }
                    let title = if compact {
                        ["Budget Accts", "Off-Budget Accts", "Closed Accts"][group]
                    } else {
                        ["Бюджетные счета", "Вне бюджета", "Закрытые счета"][group]
                    };
                    let collapsed = match group {
                        0 => self.preferences.budget_collapsed,
                        1 => self.preferences.off_budget_collapsed,
                        _ => self.preferences.closed_collapsed,
                    };
                    Frame::new()
                        .fill(panel_fill)
                        .stroke(panel_stroke)
                        .corner_radius(5)
                        .inner_margin(8)
                        .show(ui, |ui| {
                            ui.set_width(sidebar_width - 16.);
                            let (header, response) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 25.),
                                egui::Sense::click(),
                            );
                            let center = header.left_center() + Vec2::new(3., 0.);
                            let points = if collapsed {
                                vec![
                                    center + Vec2::new(-2., -4.),
                                    center + Vec2::new(-2., 4.),
                                    center + Vec2::new(3., 0.),
                                ]
                            } else {
                                vec![
                                    center + Vec2::new(-4., -2.),
                                    center + Vec2::new(4., -2.),
                                    center + Vec2::new(0., 3.),
                                ]
                            };
                            ui.painter().add(egui::Shape::convex_polygon(
                                points,
                                Color32::WHITE,
                                Stroke::NONE,
                            ));
                            let total = self
                                .account_total(&accounts)
                                .map(|v| i18n::money(v, &self.b.base))
                                .unwrap_or_else(|_| "—".into());
                            let font = FontId::new(
                                13.,
                                egui::FontFamily::Name("arial-bold-italic".into()),
                            );
                            let amount_width = if compact {
                                0.
                            } else {
                                ui.painter()
                                    .layout_no_wrap(total.clone(), font.clone(), Color32::WHITE)
                                    .size()
                                    .x
                            };
                            let title_rect = egui::Rect::from_min_max(
                                header.left_top() + Vec2::new(16., 0.),
                                header.right_bottom() - Vec2::new(amount_width + 7., 0.),
                            );
                            ui.painter().with_clip_rect(title_rect).l_text(
                                title_rect.left_center(),
                                egui::Align2::LEFT_CENTER,
                                title,
                                font.clone(),
                                Color32::WHITE,
                            );
                            if !compact {
                                ui.painter().text(
                                    header.right_center(),
                                    egui::Align2::RIGHT_CENTER,
                                    total,
                                    font,
                                    Color32::WHITE,
                                );
                            }
                            if response.clicked() {
                                match group {
                                    0 => self.preferences.budget_collapsed = !collapsed,
                                    1 => self.preferences.off_budget_collapsed = !collapsed,
                                    _ => self.preferences.closed_collapsed = !collapsed,
                                }
                                self.save_preferences();
                            }
                            if !collapsed {
                                for account in accounts {
                                    let (row, response) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), 25.),
                                        egui::Sense::click(),
                                    );
                                    if self.page == Page::Accounts
                                        && self.account.as_ref() == Some(&account.id)
                                    {
                                        ui.painter().rect_filled(
                                            row.expand2(Vec2::new(8., 0.)),
                                            0.,
                                            crate::palette::SELECTED,
                                        );
                                    } else if response.hovered() {
                                        ui.painter().rect_filled(
                                            row,
                                            2.,
                                            Color32::from_white_alpha(13),
                                        );
                                    }
                                    let balance = self.b.balance(&account.id, None, false);
                                    let amount = if account.currency == self.b.base {
                                        i18n::number(balance, &account.currency)
                                    } else {
                                        format!(
                                            "{} {}",
                                            i18n::number(balance, &account.currency),
                                            account.currency
                                        )
                                    };
                                    let font = crate::palette::bold(13.);
                                    let width = if compact {
                                        0.
                                    } else {
                                        ui.painter()
                                            .layout_no_wrap(
                                                amount.clone(),
                                                font.clone(),
                                                Color32::WHITE,
                                            )
                                            .size()
                                            .x
                                    };
                                    let name_rect = egui::Rect::from_min_max(
                                        row.min + Vec2::new(16., 0.),
                                        row.max - Vec2::new(width + 8., 0.),
                                    );
                                    ui.painter().with_clip_rect(name_rect).text(
                                        name_rect.left_center(),
                                        egui::Align2::LEFT_CENTER,
                                        &account.name,
                                        font.clone(),
                                        Color32::WHITE,
                                    );
                                    if !compact {
                                        ui.painter().text(
                                            row.right_center(),
                                            egui::Align2::RIGHT_CENTER,
                                            amount,
                                            font,
                                            Color32::WHITE,
                                        );
                                    }
                                    if response.clicked() {
                                        self.page = Page::Accounts;
                                        self.account = Some(account.id.clone());
                                        self.selected.clear();
                                    }
                                    if response.secondary_clicked() {
                                        self.open(DialogKind::AccountSettings(account.id.clone()));
                                    }
                                }
                            }
                        });
                    ui.add_space(8.);
                }
            });
        let (add, response) = ui.allocate_exact_size(
            Vec2::new(if compact { 64. } else { 119. }, 25.),
            egui::Sense::click(),
        );
        ui.painter().rect_filled(
            add,
            14.,
            Color32::from_white_alpha(if response.hovered() { 35 } else { 18 }),
        );
        ui.painter()
            .circle_filled(add.left_center() + Vec2::new(12., 0.), 7., Color32::WHITE);
        crate::icons::draw(
            ui.painter(),
            egui::Rect::from_center_size(add.left_center() + Vec2::new(12., 0.), Vec2::splat(11.)),
            5,
            TEAL,
        );
        ui.painter().l_text(
            add.left_center() + Vec2::new(24., 0.),
            egui::Align2::LEFT_CENTER,
            if compact {
                "Add"
            } else {
                "Добавить счёт"
            },
            FontId::proportional(14.),
            Color32::WHITE,
        );
        if response.clicked() {
            self.open(DialogKind::Account);
        }
        let toggle = egui::Rect::from_center_size(
            egui::pos2(ui.max_rect().right() - 15., add.center().y),
            Vec2::new(26., 25.),
        );
        if classic::sidebar_toggle(ui, toggle, compact).clicked() {
            self.preferences.sidebar_compact = !compact;
            self.save_preferences();
        }
    }
}
