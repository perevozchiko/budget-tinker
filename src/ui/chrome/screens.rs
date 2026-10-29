use super::*;

impl App {
    pub(in crate::ui) fn chrome_screen(&mut self, ctx: &egui::Context) {
        let mut c = std::mem::take(&mut self.chrome);
        let Some(screen) = c.screen else {
            self.chrome = c;
            return;
        };
        egui::CentralPanel::default().frame(Frame::new().fill(TEAL)).show(ctx, |ui| {
            crate::palette::paint_fabric(ui.painter(), &self.fabric, ui.max_rect());
            ui.visuals_mut().override_text_color = Some(Color32::WHITE);
            let height: f32 = if screen == Screen::Preferences { 490. } else if screen == Screen::Open { 295. } else if screen == Screen::New { 510. } else { 430. };
            let available = ui.max_rect();
            let form = egui::Rect::from_center_size(available.center(), Vec2::new(650., height.min(available.height() - 30.)));
            ui.scope_builder(egui::UiBuilder::new().max_rect(form), |ui| {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(Vec2::new(200., height), Layout::top_down(Align::RIGHT), |ui| {
                        let title = match screen { Screen::Preferences => "Настройки", Screen::Properties => "Свойства\nбюджета", Screen::New => "Создание\nбюджета", Screen::Open => "Открыть\nбюджет" };
                        ui.l_label(RichText::new(title).font(crate::palette::bold(30.)).color(Color32::WHITE));
                    });
                    ui.add_space(18.);
                    ui.allocate_ui_with_layout(Vec2::new(420., height), Layout::top_down(Align::Min), |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(7., 6.);
                        match screen {
                            Screen::Preferences => {
                                section(ui, "Запуск", |ui| { if ui.l_checkbox(&mut self.preferences.open_recent, "При запуске открывать последний бюджет").changed() { self.save_preferences(); } });
                                section(ui, "Облачная синхронизация: отключена", |ui| {
                                    ui.l_label("Бюджет сохраняется на этом Mac. Синхронизация с Dropbox и мобильными приложениями не подключена.");
                                    ui.add_enabled_ui(false, |ui| { classic::button(ui, "Включить облачную синхронизацию"); });
                                });
                                section(ui, "Папка бюджета", |ui| {
                                    let mut path = self.store.path.parent().unwrap_or(Path::new(".")).display().to_string();
                                    ui.add(egui::TextEdit::singleline(&mut path).desired_width(400.).interactive(false).text_color(INK));
                                    if classic::button(ui, if cfg!(target_os = "macos") { "Показать в Finder" } else { "Show Budget Folder" }).clicked() && let Err(error) = crate::desktop::reveal_budget(&self.store.path) { c.error = error; }
                                });
                                if classic::large_button(ui, "Готово", Vec2::new(160., 46.), true).clicked() { c.screen = None; }
                            }
                            Screen::Open => {
                                ui.l_label(RichText::new("Выберите бюджет BudgetTinker").size(17.));
                                ui.l_label("Откройте файл бюджета SQLite или восстановите резервную копию JSON.");
                                ui.add_space(14.);
                                if classic::large_button(ui, "Открыть бюджет", Vec2::new(400., 46.), true).clicked()
                                    && let Some(path) = self.choose_file(false, "budgettinker", ctx) {
                                        match self.switch_budget(Path::new(&path), None) { Ok(()) => c.screen = None, Err(e) => c.error = e }
                                }
                                if classic::large_button(ui, "Восстановить копию JSON", Vec2::new(400., 46.), false).clicked() { c.screen = None; self.open(DialogKind::Restore); }
                                if classic::large_button(ui, "Import Legacy Budget…", Vec2::new(400., 46.), false).clicked() { c.screen = None; self.open(DialogKind::ImportLegacyBudget); }
                                if classic::button(ui, "Отмена").clicked() { c.screen = None; }
                            }
                            Screen::Properties | Screen::New => {
                                section(ui, "Название бюджета:", |ui| { ui.add(egui::TextEdit::singleline(&mut c.name).font(FontId::proportional(18.)).margin(9).text_color(INK).desired_width(400.)); });
                                section(ui, "Хранение бюджета", |ui| {
                                    ui.l_label("Бюджет сохраняется автоматически на этом Mac.");
                                    ui.l_label("Нет, оставить локальным");
                                });
                                section(ui, "Форматы валюты и даты:", |ui| {
                                    egui::Grid::new("budget_formats").spacing([12., 8.]).show(ui, |ui| {
                                        ui.l_label("Валюта:");
                                        ui.add_enabled_ui(screen == Screen::New, |ui| { ui.visuals_mut().override_text_color = Some(INK); currency_combo(ui, "property_currency", &mut c.currency); });
                                        ui.end_row();
                                        ui.l_label("Формат суммы:"); ui.label(format!("9 876.54 {}", c.currency)); ui.end_row();
                                        ui.l_label("Формат даты:"); ui.label(today()); ui.end_row();
                                    });
                                });
                                if screen == Screen::Properties && self.b.version == 1
                                    && classic::button(ui, "Upgrade currency data…").clicked() {
                                    match self.store.preflight_upgrade() {
                                        Ok(preview) => c.upgrade = Some(preview),
                                        Err(error) => c.error = error,
                                    }
                                }
                                if screen == Screen::Properties && self.b.version == 2 && classic::button(ui, "Monthly exchange rates…").clicked() { c.monthly = Some(super::monthly_fx::MonthlyEditor::new(&self.b)); }
                                ui.horizontal(|ui| {
                                    if classic::large_button(ui, "Отмена", Vec2::new(130., 46.), false).clicked() { c.screen = None; }
                                    if classic::large_button(ui, if screen == Screen::New { "Создать бюджет" } else { "Сохранить изменения" }, Vec2::new(192., 46.), true).clicked() {
                                        let mut canceled = false;
                                        let result = if c.name.trim().is_empty() { Err("Введите название бюджета".into()) }
                                        else if screen == Screen::New {
                                            if let Some(path) = self.choose_file(true, "budgettinker", ctx) {
                                                let mut b = Budget::new(&c.currency); b.name = c.name.trim().into();
                                                self.switch_budget(Path::new(&path), Some(b))
                                            } else { canceled = true; Ok(()) }
                                        } else {
                                            let mut b = self.b.clone(); b.name = c.name.trim().into(); self.commit(b)
                                        };
                                        match result { Ok(()) if !canceled => c.screen = None, Ok(()) => {}, Err(e) => c.error = e }
                                    }
                                });
                            }
                        }
                        let error = if !c.error.is_empty() { &c.error } else if self.is_error { &self.notice } else { "" };
                        if !error.is_empty() { ui.l_label(RichText::new(error).color(Color32::from_rgb(255, 233, 131))); }
                    });
                });
            });
        });
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            c.screen = None;
        }
        c.versions_dirty |= self.chrome.versions_dirty;
        self.chrome = c;
    }
}
