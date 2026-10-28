use super::*;

impl App {
    pub(super) fn bank_form(&mut self, ctx: &egui::Context, mut d: Dialog) {
        let mut apply = false;
        let mut close = false;
        let response = chrome::modal(ctx, "bank_import").show(ctx, |ui| {
            chrome::header(ui, "Import Bank Statement", 690.);
            chrome::body(ui, 690., |ui| {
                ui.spacing_mut().item_spacing.y = 8.;
                ui.label("Choose an OFX, QFX, QIF or CSV statement and review it before importing.");
                ui.label("Destination account");
                let old_account = d.fields[0].clone();
                account_combo(ui, "bank_import_account", &mut d.fields[0], &self.b);
                let mut changed = old_account != d.fields[0];
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::TextEdit::singleline(&mut d.fields[1]).desired_width(445.)).changed();
                    if classic::button(ui, "Choose File…").clicked()
                        && let Some(path) = self.choose_file(false, "ofx", ctx) {
                        d.fields[1] = path;
                        changed = true;
                    }
                });
                if changed {
                    d.bank_preview = None;
                    d.error.clear();
                }
                if ui.add_enabled(!d.fields[0].is_empty() && !d.fields[1].is_empty(), classic::Button::new("Check and Preview")).clicked() {
                    d.bank_preview = None;
                    match budgettinker::bank::preview(&self.b, Path::new(&d.fields[1]), &d.fields[0]) {
                        Ok(preview) => { d.bank_preview = Some(Box::new(preview)); d.error.clear(); }
                        Err(error) => d.error = error,
                    }
                }
                if let Some(preview) = &d.bank_preview {
                    let b = preview.budget();
                    let currency = &b.account(&d.fields[0]).unwrap().currency;
                    ui.strong(format!("{} new transactions, {} matched", preview.count() - preview.matched_count(), preview.matched_count()));
                    ui.label(format!("Account balance after import: {}", money(b.balance(&d.fields[0], None, false), currency)));
                    ui.label("Repeated rows are skipped. Matches keep your original dates and details. Imports await approval unless already reconciled.");
                    egui::ScrollArea::both().id_salt("bank_preview_rows")
                        .max_height((ctx.screen_rect().height() - 410.).clamp(100., 370.))
                        .show(ui, |ui| {
                            egui::Grid::new("bank_preview_table").striped(true).show(ui, |ui| {
                                for label in ["Date", "Payee", "Category", "Memo", "Amount", "Match"] { ui.strong(label); }
                                ui.end_row();
                                for t in preview.transactions() {
                                    ui.label(&t.date);
                                    ui.label(&t.payee);
                                    ui.label(b.transaction_category_name(t));
                                    ui.label(&t.memo);
                                    ui.label(money(t.amount, currency));
                                    ui.label(if b.register_meta.get(&t.id).is_some_and(|m| m.matched.is_some()) { "Matched" } else { "New" });
                                    ui.end_row();
                                    for part in &t.splits {
                                        ui.label("");
                                        ui.label(b.split_payee_name(t, part));
                                        ui.label(b.split_category_name(part));
                                        ui.label(&part.memo);
                                        ui.label(money(part.amount, currency));
                                        ui.label("");
                                        ui.end_row();
                                    }
                                }
                            });
                        });
                }
                if !d.error.is_empty() { ui.colored_label(RED, crate::i18n::tr(ui.ctx(), &d.error)); }
            });
            chrome::footer(ui, 690., |ui| {
                if ui.add_enabled(d.bank_preview.is_some(), classic::Button::new("Import Transactions").primary()).clicked() { apply = true; }
                if classic::button(ui, "Cancel").clicked() { close = true; }
            });
        });
        if apply {
            match self.save_dialog(&d) {
                Ok(()) => close = true,
                Err(error) => {
                    d.error = error;
                    d.bank_preview = None;
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            close = true;
        }
        if !close && !response.should_close() {
            self.dialog = Some(d);
        }
    }
}
