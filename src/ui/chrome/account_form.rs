use super::*;

impl App {
    pub(in crate::ui) fn classic_form(&mut self, ctx: &egui::Context, mut d: Dialog) {
        let width = 474.;
        let mut save = false;
        let mut close = false;
        let response = modal(ctx, "dialog").show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            account_header(ui, width);
            body(ui, width, |ui| {
                ui.spacing_mut().item_spacing.y = 8.;
                    ui.style_mut().text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.));
                    ui.spacing_mut().interact_size.y = 27.;
                    ui.set_min_height(310.);
                    ui.add_space(4.);
                    ui.l_label(RichText::new("Название:").font(crate::palette::bold(13.)));
                    if d.fields[9].is_empty() {
                        d.fields[0] = i18n::tr(ctx, "Новый счёт");
                    }
                    let response = ui.add(egui::TextEdit::singleline(&mut d.fields[0]).id_salt("account_name").desired_width(width - 56.).font(crate::palette::bold(16.)).text_color(INK).margin(10));
                    if d.fields[9].is_empty() {
                        response.request_focus();
                        if let Some(mut state) = egui::TextEdit::load_state(ctx, response.id) {
                            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(d.fields[0].chars().count()))));
                            state.store(ctx, response.id);
                        }
                        d.fields[9] = "focused".into();
                    }
                    ui.add_space(5.); ui.separator(); ui.add_space(5.);
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.l_label(RichText::new("Текущий остаток:").font(crate::palette::bold(13.)));
                            ui.add(egui::TextEdit::singleline(&mut d.fields[2]).desired_width(185.).font(crate::palette::bold(14.)).text_color(INK).margin(6));
                        });
                        ui.vertical(|ui| {
                            ui.l_label(RichText::new("Дата текущего остатка:").font(crate::palette::bold(13.)));
                            let (earlier, later) = d.fields.split_at_mut(8);
                            classic::account_date_field(ui, &mut earlier[3], &mut later[0]);
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.l_label(RichText::new("Вы всегда сможете изменить это позже").size(12.).italics());
                    });
                    ui.add_space(6.); ui.separator(); ui.add_space(6.);
                    ui.l_label(RichText::new("Тип:").font(crate::palette::bold(13.)));
                    if forms::account_type(ui, &mut d.fields[4], 195.) { d.check = !["investment", "mortgage", "asset", "liability"].contains(&d.fields[4].as_str()); d.second = true; }
                    ui.add_space(8.);
                    account_radio(ui, &mut d.check, &mut d.second, true, "Бюджетный счёт — влияет на бюджет");
                    account_radio(ui, &mut d.check, &mut d.second, false, "Вне бюджета — не влияет на бюджет");
                    if d.check && amount(&d.fields[2], &self.b.base).is_ok_and(|value| value < 0) {
                        ui.add_space(6.);
                        ui.label("The starting debt will appear in Starting Debt. Budget money there to pay it down; record payments as transfers.");
                    }
                if !d.error.is_empty() { ui.l_label(RichText::new(&d.error).color(RED)); }
            });
            footer(ui, width, |ui| {
                let enabled = !d.fields[0].trim().is_empty() && !d.fields[4].is_empty() && chrono::NaiveDate::parse_from_str(&d.fields[8], "%m/%d/%Y").is_ok_and(|date| date.format("%Y-%m-%d").to_string() == d.fields[3]);
                if ui.add_enabled(enabled, classic::Button::new("Создать счёт")).clicked() { save = true; }
                if classic::button(ui, "Отмена").clicked() { close = true; }
            });
        });
        if save {
            match self.save_dialog(&d) {
                Ok(()) => close = true,
                Err(e) => d.error = e,
            }
        }
        if !close && !response.should_close() {
            self.dialog = Some(d);
        }
    }
}
