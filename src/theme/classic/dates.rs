use super::*;

pub fn account_date_field(ui: &mut Ui, value: &mut String, display: &mut String) {
    date_field_with_display(ui, value, Some(display));
}

fn date_field_with_display(ui: &mut Ui, value: &mut String, mut display: Option<&mut String>) {
    ui.horizontal(|ui| {
        let date_response = if let Some(shown) = display.as_mut() {
            let edited = ui.add(
                egui::TextEdit::singleline(*shown)
                    .desired_width(159.)
                    .font(crate::palette::bold(14.))
                    .text_color(TEXT)
                    .margin(6),
            );
            if edited.changed()
                && let Ok(date) = chrono::NaiveDate::parse_from_str(shown, "%m/%d/%Y")
            {
                *value = date.format("%Y-%m-%d").to_string();
            }
            edited
        } else {
            ui.add(egui::TextEdit::singleline(value).desired_width(145.))
        };
        let response = if display.is_some() {
            account_calendar_button(ui)
        } else {
            small_button(ui, "▦")
        };
        let before_calendar = value.clone();
        date_calendar(
            ui,
            value,
            &response,
            display.is_some().then_some(date_response.rect),
        );
        if *value != before_calendar
            && let Some(shown) = display
            && let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        {
            *shown = date.format("%m/%d/%Y").to_string();
        }
    });
}

pub fn date_calendar(ui: &mut Ui, value: &mut String, response: &Response, anchor: Option<Rect>) {
    use chrono::Datelike;
    let id = response.id.with("calendar_month");
    let selected = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());
    let mut month = ui
        .ctx()
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| selected.format("%Y-%m").to_string());
    if response.clicked() {
        month = selected.format("%Y-%m").to_string();
    }
    let popup = egui::Popup::menu(response)
        .width(183.)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    let popup = if let Some(anchor) = anchor {
        popup
            .anchor(anchor)
            .align(egui::RectAlign::BOTTOM_START)
            .gap(2.)
    } else {
        popup.align(egui::RectAlign::BOTTOM_END)
    };
    popup
        .frame(
            egui::Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1_f32, Color32::from_gray(180)))
                .corner_radius(4.)
                .inner_margin(4.),
        )
        .show(|ui| {
            ui.set_width(175.);
            ui.set_min_height(201.);
            egui::Frame::new()
                .fill(Color32::from_gray(232))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        if small_button(ui, "◀").clicked() {
                            month = budgettinker::model::month_shift(&month, -1);
                        }
                        let month_date =
                            chrono::NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d")
                                .unwrap();
                        let names = [
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
                        ];
                        let label = format!(
                            "{}  {}",
                            crate::i18n::tr(ui.ctx(), names[month_date.month0() as usize]),
                            month_date.year(),
                        );
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(123., 26.), egui::Sense::hover());
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            crate::palette::bold(13.),
                            TEXT,
                        );
                        if small_button(ui, "▶").clicked() {
                            month = budgettinker::model::month_shift(&month, 1);
                        }
                    });
                });
            ui.add_space(2.);
            egui::Frame::new()
                .fill(Color32::from_rgb(222, 240, 249))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        for day in ["S", "M", "T", "W", "T", "F", "S"] {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(25., 28.), egui::Sense::hover());
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                day,
                                crate::palette::bold(12.),
                                TEXT,
                            );
                        }
                    });
                });
            let first =
                chrono::NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").unwrap();
            let last = chrono::NaiveDate::parse_from_str(
                &budgettinker::model::Budget::month_end(&month),
                "%Y-%m-%d",
            )
            .unwrap()
            .day();
            egui::Grid::new(id.with("days"))
                .num_columns(7)
                .min_col_width(25.)
                .max_col_width(25.)
                .spacing([0., 0.])
                .show(ui, |ui| {
                    let offset = first.weekday().num_days_from_sunday();
                    for cell in 0..(offset + last).div_ceil(7) * 7 {
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(25., 24.), egui::Sense::click());
                        if cell >= offset && cell < offset + last {
                            let day = cell - offset + 1;
                            let date = format!("{month}-{day:02}");
                            let selected = date == *value;
                            if selected || response.hovered() {
                                ui.painter().rect_filled(
                                    rect.shrink(1.),
                                    0.,
                                    if selected {
                                        Color32::from_gray(133)
                                    } else {
                                        Color32::from_gray(232)
                                    },
                                );
                            }
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                day.to_string(),
                                FontId::proportional(12.),
                                if selected { Color32::WHITE } else { TEXT },
                            );
                            if response.clicked() {
                                *value = date;
                                ui.close();
                            }
                        }
                        if cell % 7 == 6 {
                            ui.end_row();
                        }
                    }
                });
        });
    ui.ctx().data_mut(|d| d.insert_temp(id, month));
}

fn account_calendar_button(ui: &mut Ui) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.), egui::Sense::click());
    gradient(
        ui.painter(),
        rect,
        3.,
        Color32::WHITE,
        Color32::from_gray(213),
    );
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        3.,
        Stroke::new(1_f32, BORDER),
        egui::StrokeKind::Inside,
    );
    let icon = rect.shrink(5.);
    ui.painter().rect_filled(icon, 0., Color32::WHITE);
    ui.painter().rect_stroke(
        icon,
        0.,
        Stroke::new(1_f32, Color32::from_gray(84)),
        egui::StrokeKind::Inside,
    );
    ui.painter().rect_filled(
        Rect::from_min_size(icon.min, Vec2::new(icon.width(), 4.)),
        0.,
        Color32::from_gray(84),
    );
    for n in 1..3 {
        let x = icon.left() + icon.width() * n as f32 / 3.;
        let y = icon.top() + 4. + (icon.height() - 4.) * n as f32 / 3.;
        ui.painter()
            .vline(x, icon.y_range(), Stroke::new(0.7_f32, BORDER));
        ui.painter()
            .hline(icon.x_range(), y, Stroke::new(0.7_f32, BORDER));
    }
    response.on_hover_text(crate::i18n::tr(ui.ctx(), "Выбрать дату"))
}
