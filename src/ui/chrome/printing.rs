use super::*;

impl App {
    #[cfg(not(target_os = "macos"))]
    pub(in crate::ui) fn print_current(&mut self, _ctx: &egui::Context) -> Result<()> {
        Err("Native printing is currently available on macOS only. The on-screen preview remains available.".into())
    }

    #[cfg(target_os = "macos")]
    pub(in crate::ui) fn print_current(&mut self, ctx: &egui::Context) -> Result<()> {
        let mut lines = format!("{} — {}\n\n", self.b.name, month_name(ctx, &self.month));
        if self.page == Page::Reports {
            if self.chrome.report_print_job.is_some() {
                return Err("A report print dialog is already open.".into());
            }
            self.chrome.report_print_job = Some(self.report_print_document()?.print()?);
            return Ok(());
        } else if self.page == Page::Accounts {
            for t in self
                .b
                .transactions
                .iter()
                .filter(|t| self.account.as_ref().is_none_or(|a| a == &t.account))
            {
                lines.push_str(&format!(
                    "{}  {:24}  {:>14}  {}\n",
                    t.date,
                    t.payee,
                    i18n::money(t.amount, &self.b.account(&t.account)?.currency),
                    t.memo
                ));
            }
        } else {
            lines.push_str(&format!(
                "{:<36} {:>14} {:>14} {:>14}\n",
                i18n::tr(ctx, "Категории"),
                i18n::tr(ctx, "Назначено"),
                i18n::tr(ctx, "Расходы"),
                i18n::tr(ctx, "Остаток")
            ));
            for cat in &self.b.categories {
                lines.push_str(&format!(
                    "{:<36} {:>14} {:>14} {:>14}\n",
                    i18n::tr(ctx, &cat.name),
                    i18n::number(self.b.allocated(&self.month, &cat.id), &self.b.base),
                    i18n::number(self.b.activity(&self.month, &cat.id), &self.b.base),
                    i18n::number(self.b.category_balance(&self.month, &cat.id), &self.b.base)
                ));
            }
        }
        let path = std::env::temp_dir().join(format!("budgettinker-print-{}.txt", id()));
        std::fs::write(&path, lines).map_err(|e| e.to_string())?;
        let script = r#"ObjC.import('AppKit'); ObjC.import('Foundation');
function run(args) {
 const app = $.NSApplication.sharedApplication; app.setActivationPolicy($.NSApplicationActivationPolicyRegular);
 const content = $.NSString.stringWithContentsOfFileEncodingError(args[0], $.NSUTF8StringEncoding, null);
 const view = $.NSTextView.alloc.initWithFrame($.NSMakeRect(0, 0, 760, 1000));
 view.string = content; view.font = $.NSFont.userFixedPitchFontOfSize(10);
 const info = $.NSPrintInfo.sharedPrintInfo; info.orientation = $.NSPaperOrientationLandscape;
 info.horizontalPagination = $.NSFitPagination; info.verticallyCentered = false;
 const operation = $.NSPrintOperation.printOperationWithViewPrintInfo(view, info);
 app.activateIgnoringOtherApps(true); operation.runOperation;
 $.NSFileManager.defaultManager.removeItemAtPathError(args[0], null);
}"#;
        std::process::Command::new("/usr/bin/osascript")
            .args(["-l", "JavaScript", "-e", script])
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(in crate::ui) fn print_preview(&mut self, ctx: &egui::Context) {
        let mut zoom = if self.chrome.print_zoom == 0. {
            1.
        } else {
            self.chrome.print_zoom
        };
        egui::CentralPanel::default()
            .frame(Frame::new().fill(classic::CHROME).inner_margin(12))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.l_label(RichText::new("Предварительный просмотр печати").size(16.));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if classic::button(ui, "×").clicked() {
                            self.chrome.print_preview = false;
                        }
                    });
                });
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    if classic::button(ui, "Печать…").clicked()
                        && let Err(e) = self.print_current(ctx)
                    {
                        self.notify(e, true);
                    }
                    if self.page == Page::Reports && classic::button(ui, "Save PDF…").clicked() {
                        let result = self.report_print_document().and_then(|document| {
                            if let Some(path) = self.choose_file(true, "pdf", ctx) {
                                document.export_pdf(std::path::Path::new(&path))?;
                                self.notify("PDF exported.", false);
                            }
                            Ok(())
                        });
                        if let Err(error) = result {
                            self.notify(error, true);
                        }
                    }
                    ui.l_label("Масштаб");
                    ui.add(egui::Slider::new(&mut zoom, 0.5..=2.).show_value(false));
                    ui.label(format!("{:.0}%", zoom * 100.));
                });
                if self.is_error {
                    ui.colored_label(RED, crate::i18n::tr(ui.ctx(), &self.notice));
                }
                if self.chrome.report_print_job.is_some() {
                    ui.label("Print dialog open…");
                }
                ui.add_space(12.);
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.page == Page::Reports {
                            match self.report_print_document() {
                                Ok(document) => {
                                    ui.label(format!(
                                        "{} {} · A4 landscape",
                                        document.pages.len(),
                                        if document.pages.len() == 1 {
                                            "page"
                                        } else {
                                            "pages"
                                        }
                                    ));
                                    document.preview(ui, zoom);
                                }
                                Err(error) => {
                                    ui.colored_label(RED, crate::i18n::tr(ui.ctx(), error));
                                }
                            }
                            return;
                        }
                        Frame::new()
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1_f32, Color32::from_gray(163)))
                            .inner_margin(40)
                            .show(ui, |ui| {
                                ui.set_min_width(760. * zoom);
                                ui.set_min_height(900. * zoom);
                                ui.l_label(
                                    RichText::new(if self.page == Page::Reports {
                                        format!(
                                            "{} — {}",
                                            self.b.name,
                                            budgettinker::reports::TITLES[self.report]
                                        )
                                    } else {
                                        format!(
                                            "{} — {}",
                                            self.b.name,
                                            month_name(ctx, &self.month)
                                        )
                                    })
                                    .font(crate::palette::bold(25. * zoom)),
                                );
                                ui.add_space(24. * zoom);
                                if self.page == Page::Accounts {
                                    egui::Grid::new("print_transactions")
                                        .striped(true)
                                        .spacing([16. * zoom, 8. * zoom])
                                        .show(ui, |ui| {
                                            for title in
                                                ["Дата", "Получатель", "Категория", "Сумма"]
                                            {
                                                ui.l_strong(title);
                                            }
                                            ui.end_row();
                                            for t in self.b.transactions.iter().filter(|t| {
                                                self.account
                                                    .as_ref()
                                                    .is_none_or(|a| a == &t.account)
                                            }) {
                                                ui.label(&t.date);
                                                ui.label(&t.payee);
                                                ui.l_label(self.b.transaction_category_name(t));
                                                ui.label(i18n::money(
                                                    t.amount,
                                                    &self.b.account(&t.account).unwrap().currency,
                                                ));
                                                ui.end_row();
                                            }
                                        });
                                } else {
                                    ui.l_label(
                                        RichText::new(format!(
                                            "{}: {}",
                                            i18n::tr(ctx, "Доступно для распределения"),
                                            self.b
                                                .available(&self.month)
                                                .map(|v| i18n::money(v, &self.b.base))
                                                .unwrap_or_else(|_| "—".into())
                                        ))
                                        .font(crate::palette::bold(20. * zoom)),
                                    );
                                    ui.add_space(20. * zoom);
                                    egui::Grid::new("print_budget")
                                        .striped(true)
                                        .spacing([22. * zoom, 8. * zoom])
                                        .show(ui, |ui| {
                                            for title in
                                                ["Категории", "Назначено", "Расходы", "Остаток"]
                                            {
                                                ui.l_strong(title);
                                            }
                                            ui.end_row();
                                            let mut group = String::new();
                                            for c in &self.b.categories {
                                                if group != c.group {
                                                    group = c.group.clone();
                                                    ui.l_strong(&group);
                                                    for _ in 0..3 {
                                                        ui.label("");
                                                    }
                                                    ui.end_row();
                                                }
                                                ui.l_label(RichText::new(&c.name).size(13. * zoom));
                                                for value in [
                                                    self.b.allocated(&self.month, &c.id),
                                                    self.b.activity(&self.month, &c.id),
                                                    self.b.category_balance(&self.month, &c.id),
                                                ] {
                                                    ui.label(
                                                        RichText::new(i18n::number(
                                                            value,
                                                            &self.b.base,
                                                        ))
                                                        .size(13. * zoom),
                                                    );
                                                }
                                                ui.end_row();
                                            }
                                        });
                                }
                            });
                    });
            });
        self.chrome.print_zoom = zoom;
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.chrome.print_preview = false;
        }
    }
}
