use super::*;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(job) = &self.chrome.report_print_job {
            match job.try_recv() {
                Ok(result) => {
                    self.chrome.report_print_job = None;
                    if let Err(error) = result {
                        self.notify(error, true);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.chrome.report_print_job = None;
                    self.notify("The PDF print helper stopped unexpectedly.", true);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                }
            }
        }
        self.preferences.language.install(ctx);
        if self.chrome.monthly.is_some() {
            self.native_menu.set_enabled(false, false, false);
            egui::CentralPanel::default()
                .frame(Frame::new().fill(TEAL))
                .show(ctx, |_| {});
            self.monthly_fx_dialog(ctx);
            return;
        }
        if self.chrome.upgrade.is_some() {
            self.native_menu.set_enabled(false, false, false);
            egui::CentralPanel::default()
                .frame(Frame::new().fill(TEAL))
                .show(ctx, |_| {});
            self.upgrade_dialog(ctx);
            return;
        }
        self.menu_commands(ctx);
        let title = format!("BudgetTinker — {}", i18n::tr(ctx, &self.b.name));
        if self.window_title != title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }

        if self.chrome.print_preview {
            self.print_preview(ctx);
            self.chrome_dialog(ctx);
            return;
        }
        if self.chrome.screen.is_some() {
            self.chrome_screen(ctx);
            self.chrome_dialog(ctx);
            self.dialog(ctx);
            return;
        }
        if self.page != Page::Budget {
            let mut editor = std::mem::take(&mut self.budget_editor);
            let base = self.b.base.clone();
            editor.begin(ctx);
            editor.finish(ctx, &base, &[], |cell, value| {
                let mut next = self.b.clone();
                next.allocate(&cell.month, &cell.category, value)?;
                self.commit(next)
            });
            self.budget_editor = editor;
        }
        if self.is_error {
            egui::TopBottomPanel::bottom("status")
                .frame(
                    Frame::new()
                        .fill(Color32::from_rgb(255, 237, 232))
                        .inner_margin(6),
                )
                .show(ctx, |ui| {
                    ui.l_label(RichText::new(&self.notice).size(12.).color(RED));
                });
        }
        egui::CentralPanel::default()
            .frame(Frame::new().fill(TEAL))
            .show(ctx, |ui| {
                let root = ui.max_rect();
                crate::palette::paint_fabric(ui.painter(), &self.fabric, root);
                let sidebar_width = if self.preferences.sidebar_compact {
                    148.
                } else {
                    248.
                };
                let sidebar = egui::Rect::from_min_max(
                    root.min + Vec2::splat(8.),
                    egui::pos2(root.left() + sidebar_width + 8., root.bottom() - 8.),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(sidebar), |ui| {
                    self.sidebar(ui)
                });
                let work = egui::Rect::from_min_max(
                    egui::pos2(root.left() + sidebar_width + 16., root.top()) + Vec2::splat(8.),
                    root.max - Vec2::splat(8.),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(work), |ui| {
                    let size = ui.available_size() - Vec2::splat(14.);
                    Frame::new()
                        .fill(classic::CHROME)
                        .stroke(Stroke::new(1_f32, Color32::from_gray(155)))
                        .corner_radius(7)
                        .inner_margin(6)
                        .show(ui, |ui| {
                            ui.set_min_size(size);
                            ui.scope(|ui| match self.page {
                                Page::Budget => self.budget(ui),
                                Page::Accounts => self.accounts(ui),
                                Page::Reports => self.reports(ui),
                            });
                        });
                });
            });
        self.dialog(ctx);
        self.chrome_dialog(ctx);
    }
}
