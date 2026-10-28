use super::*;

impl App {
    pub(super) fn budget_navigation(&mut self, ui: &mut egui::Ui, label_w: f32, count: i32) {
        let (bar, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 27.), egui::Sense::hover());
        let start = bar.left() + label_w + 8.;
        if classic::month_arrow(
            ui,
            egui::Rect::from_min_size(egui::pos2(start, bar.top()), Vec2::new(29., 24.)),
            false,
        )
        .clicked()
        {
            self.month = month_shift(&self.month, -1);
        }
        let year = self.month[..4].to_owned();
        let year_rect =
            egui::Rect::from_min_size(egui::pos2(start + 31., bar.top()), Vec2::new(43., 24.));
        ui.painter().rect_filled(year_rect, 0., DARK);
        ui.painter().l_text(
            year_rect.center(),
            egui::Align2::CENTER_CENTER,
            &year,
            crate::palette::bold(12.),
            Color32::LIGHT_GRAY,
        );
        let tab_width = (bar.right() - start - 105.) / 12.;
        for (index, name) in [
            "Январь",
            "Февраль",
            "Март",
            "Апрель",
            "Май",
            "Июнь",
            "Июль",
            "Август",
            "Сентябрь",
            "Октябрь",
            "Ноябрь",
            "Декабрь",
        ]
        .iter()
        .enumerate()
        {
            let tab = egui::Rect::from_min_size(
                egui::pos2(start + 74. + index as f32 * tab_width, bar.top()),
                Vec2::new(tab_width, 24.),
            );
            let active = (0..count).any(|offset| {
                month_shift(&self.month, offset) == format!("{year}-{:02}", index + 1)
            });
            classic::gradient(
                ui.painter(),
                tab,
                0.,
                if active {
                    Color32::from_rgb(0, 144, 202)
                } else {
                    Color32::from_rgb(86, 107, 116)
                },
                if active {
                    Color32::from_rgb(0, 126, 184)
                } else {
                    Color32::from_rgb(65, 84, 93)
                },
            );
            ui.painter().rect_stroke(
                tab,
                0.,
                Stroke::new(0.5_f32, Color32::from_rgb(45, 69, 79)),
                egui::StrokeKind::Inside,
            );
            let mut label = i18n::tr(ui.ctx(), name);
            if ui
                .painter()
                .layout_no_wrap(label.clone(), FontId::proportional(12.), Color32::WHITE)
                .size()
                .x
                > tab.width() - 5.
            {
                label = label.chars().take(3).collect();
            }
            ui.painter().text(
                tab.center(),
                egui::Align2::CENTER_CENTER,
                label,
                FontId::proportional(12.),
                if active {
                    Color32::WHITE
                } else {
                    Color32::LIGHT_GRAY
                },
            );
            if ui
                .interact(
                    tab,
                    ui.id().with(("month_tab", index)),
                    egui::Sense::click(),
                )
                .clicked()
            {
                self.month = format!("{year}-{:02}", index + 1);
            }
        }
        if classic::month_arrow(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(bar.right() - 29., bar.top()),
                Vec2::new(29., 24.),
            ),
            true,
        )
        .clicked()
        {
            self.month = month_shift(&self.month, 1);
        }
        ui.add_space(4.);
    }
}
