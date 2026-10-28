use super::*;

impl App {
    pub(in crate::ui) fn chrome_dialog(&mut self, ctx: &egui::Context) {
        let mut c = std::mem::take(&mut self.chrome);
        let Some(overlay) = c.overlay.clone() else {
            self.chrome = c;
            return;
        };
        let width = if matches!(overlay, Overlay::Payees) {
            720.
        } else if matches!(overlay, Overlay::Wifi) {
            464.
        } else {
            575.
        };
        let title = match &overlay {
            Overlay::Export => "Экспорт",
            Overlay::Fresh => "Начать бюджет заново",
            Overlay::Wifi => "Синхронизация с устройством по Wi-Fi",
            Overlay::Payees => "Настройки получателей",
            Overlay::About => "О программе BudgetTinker",
            Overlay::Diagnostics => "Режим диагностики",
            Overlay::Technical => "Техническая информация",
            Overlay::Help(_) => "Справка BudgetTinker",
            Overlay::SavedVersion => "Версия сохранена",
            Overlay::LoadVersion(_) => "Загрузить другую версию",
        };
        let mut close = false;
        let response = modal(ctx, "chrome_dialog").show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            header(ui, title, width);
            body(ui, width, |ui| {
                ui.spacing_mut().item_spacing.y = 8.;
                match &overlay {
                    Overlay::Export => {
                        ui.l_label("Выберите способ экспорта данных:");
                        ui.add_space(14.);
                        for (kind, label, description) in [
                            ("csv", "Экспорт CSV", "Операции всех счетов. Можно открыть в Excel и других табличных редакторах."),
                            ("qif", "Экспорт QIF", "Операции всех счетов для переноса в другие финансовые программы."),
                            ("budget", "Экспорт бюджета CSV", "Назначения, расходы и остатки категорий по месяцам."),
                        ] {
                            let (r, hit) = ui.allocate_exact_size(Vec2::new(width - 56., 78.), egui::Sense::click());
                            classic::gradient(ui.painter(), r, 4., Color32::WHITE, Color32::from_gray(233));
                            ui.painter().rect_stroke(r.shrink(0.5), 4., Stroke::new(if hit.hovered() { 1.5_f32 } else { 1_f32 }, if hit.hovered() { Color32::from_rgb(0, 174, 238) } else { classic::BORDER }), egui::StrokeKind::Inside);
                            ui.painter().text(r.min + Vec2::new(15., 13.), egui::Align2::LEFT_TOP, i18n::tr(ctx, label), crate::palette::bold(16.), INK);
                            let description = ui.painter().layout(i18n::tr(ctx, description), FontId::proportional(12.), INK, r.width() - 30.);
                            ui.painter().galley(r.min + Vec2::new(15., 36.), description, INK);
                            if hit.clicked() && let Some(path) = self.choose_file(true, if kind == "qif" { "qif" } else { "csv" }, ctx) {
                                let result = match kind { "qif" => store::export_qif(&self.b, Path::new(&path)), "budget" => store::export_budget_csv(&self.b, Path::new(&path)), _ => store::export_csv(&self.b, Path::new(&path)) };
                                match result { Ok(()) => close = true, Err(e) => c.error = e }
                            }
                            ui.add_space(10.);
                        }
                    }
                    Overlay::Fresh => {
                        ui.l_label(RichText::new("Начать бюджет с чистого листа").font(crate::palette::bold(17.)).color(GREEN));
                        ui.l_label("Счета, текущие остатки, категории и плановые операции будут сохранены в новом бюджете. История операций и назначения начнутся заново.");
                        ui.add_space(12.);
                        ui.l_label(RichText::new("Исходный бюджет останется в отдельном файле.").strong());
                    }
                    Overlay::Wifi => {
                        ui.l_label("Шаг 1: подключите мобильное устройство к той же сети Wi-Fi.");
                        ui.l_label("Шаг 2: откройте экран синхронизации на устройстве.");
                        ui.add_space(15.);
                        ui.l_label("Синхронизация с мобильными приложениями пока не реализована.");
                        ui.add_space(20.);
                        ui.l_label("IP-адрес мобильного устройства:");
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut c.payee).desired_width(400.).margin(8));
                        ui.add_space(8.); ui.l_label("Код доступа устройства:");
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut c.renamed_payee).password(true).desired_width(400.).margin(8));
                    }
                    Overlay::Payees => self.payee_settings(ui, &mut c),
                    Overlay::About => {
                        ui.horizontal_top(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(64.), egui::Sense::hover());
                            crate::icons::draw(ui.painter(), r.shrink(10.), 0, crate::palette::LIME);
                            ui.vertical(|ui| {
                                ui.l_label(RichText::new("BudgetTinker").font(crate::palette::bold(18.)).color(GREEN));
                                ui.l_label(format!("Версия: {}", env!("CARGO_PKG_VERSION")));
                                ui.l_label("Локальное приложение для ведения бюджета.");
                                ui.add_space(12.);
                                ui.l_label("Локальное планирование бюджета на Rust и egui.");
                            });
                        });
                    }
                    Overlay::Diagnostics => {
                        ui.l_label("Диагностика показывает путь бюджета, версию приложения и состояние локального хранилища. Данные никуда не отправляются.");
                        if ui.l_checkbox(&mut self.preferences.diagnostics, "Включить режим диагностики").changed() { self.save_preferences(); }
                    }
                    Overlay::Technical => {
                        ui.l_label(format!("BudgetTinker {} · {} / {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH));
                        ui.l_label(format!("Счетов: {} · Категорий: {} · Операций: {}", self.b.accounts.len(), self.b.categories.len(), self.b.transactions.len()));
                        ui.l_label("Хранилище: SQLite · автоматическое сохранение · 100 версий");
                        ui.label(self.store.path.display().to_string());
                        if classic::button(ui, "Скопировать информацию").clicked() { ctx.copy_text(format!("BudgetTinker {}\n{} / {}\n{}", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH, self.store.path.display())); }
                    }
                    Overlay::Help(topic) => {
                        let (heading, content) = match topic.as_str() {
                            "privacy" => ("Конфиденциальность", "Бюджет и резервные копии хранятся локально. Приложение не отправляет операции, названия счетов или категории на сервер."),
                            "release_notes" => ("Изменения версии 0.1.0", "Системное меню macOS; свойства и создание бюджета; сохранение и загрузка версий; экспорт операций и бюджета; настройки получателей; классическое оформление."),
                            "credit_basics" | "credit_payments" => ("Credit Cards in BudgetTinker", "Add the card as a budget account with a negative starting balance. Starting Debt holds that old debt and carries its negative balance forward. Budget money to that category to reduce the old debt.\n\nBudget new purchases and interest in their spending categories. Record a payment as a transfer from Checking to the card; it does not create another expense or reduce the old-debt category a second time. A categorized refund restores money to its original category, even after a payment.\n\nThe card balance includes new purchases and payments, so it can differ from the Starting Debt category balance. Existing accounts are preserved; their starting balances are not recategorized automatically."),
                            _ => ("Первые шаги", "1. Добавьте счета и начальные остатки.\n\n2. Распределите деньги по категориям в колонке Budgeted. Нажмите на сумму, введите назначение; Enter или Tab перейдёт к следующей категории.\n\n3. В реестре добавляйте доходы, расходы и переводы. Доходы без категории увеличивают доступную сумму бюджета.\n\n4. Сверяйте подтверждённые операции с выпиской.\n\n5. Используйте отчёты и сохраняйте резервные копии через меню File."),
                        };
                        ui.l_label(RichText::new(heading).font(crate::palette::bold(18.)).color(GREEN));
                        ui.add_space(10.); ui.l_label(content);
                    }
                    Overlay::SavedVersion => { ui.l_label("Версия бюджета сохранена. Её можно открыть через Файл → Загрузить другую версию."); }
                    Overlay::LoadVersion(_) => { ui.l_label("Загрузить выбранную версию бюджета? Текущее состояние будет сохранено в истории. Загрузку можно отменить через Правка → Отменить."); }
                }
                let error = if !c.error.is_empty() { &c.error } else if self.is_error { &self.notice } else { "" };
                if !error.is_empty() { ui.l_label(RichText::new(error).color(RED)); }
            });
            footer(ui, width, |ui| {
                match overlay {
                    Overlay::Fresh => if classic::button(ui, "Начать заново").clicked()
                        && let Some(path) = self.choose_file(true, "budgettinker", ctx) {
                        let result = self.b.fresh_start().and_then(|budget| self.switch_budget(Path::new(&path), Some(budget)));
                        match result { Ok(()) => { close = true; c.screen = None; }, Err(e) => c.error = e }
                    },
                    Overlay::LoadVersion(id) => if classic::button(ui, "Загрузить версию").clicked() {
                        match self.store.version(id).and_then(|b| self.commit(b)) { Ok(()) => close = true, Err(e) => c.error = e }
                    },
                    Overlay::Wifi => { ui.add_enabled(false, classic::Button::new("Синхронизировать")); }
                    _ => {}
                }
                if classic::button(ui, if matches!(overlay, Overlay::Fresh | Overlay::Export | Overlay::LoadVersion(_) | Overlay::Wifi) { "Отмена" } else { "Закрыть" }).clicked() { close = true; }
            });
        });
        if close || response.should_close() {
            c.overlay = None;
            c.error.clear();
        }
        c.versions_dirty |= self.chrome.versions_dirty;
        self.chrome = c;
    }
}
