use super::*;

impl App {
    pub(super) fn legacy_import_form(&mut self, ctx: &egui::Context, mut d: Dialog) {
        let mut keep = true;
        let mut apply = false;
        chrome::modal(ctx, "legacy_import").show(ctx, |ui| {
            chrome::header(ui, "Import Legacy Budget", 690.);
            Frame::new().inner_margin(20).show(ui, |ui| {
                ui.set_width(650.);
                ui.label("Close the source application after synchronization, then choose its budget folder.");
                ui.label("The source is read only. Check the mapping before replacing this budget.");
                ui.horizontal(|ui| {
                    if ui.add(egui::TextEdit::singleline(&mut d.fields[0]).desired_width(460.)).changed() {
                        d.legacy_preview = None;
                        d.check = false;
                        d.error.clear();
                    }
                    if classic::button(ui, "Choose Folder…").clicked() {
                        match crate::desktop::choose_budget_folder() {
                            Ok(Some(path)) => {
                                d.fields[0] = path;
                                d.legacy_preview = None;
                                d.check = false;
                                d.error.clear();
                            }
                            Ok(None) => {}
                            Err(error) => d.error = error,
                        }
                    }
                });
                if classic::button(ui, "Check and Preview").clicked() {
                    d.legacy_preview = None;
                    d.check = false;
                    match budgettinker::legacy_budget::preview(Path::new(&d.fields[0])) {
                        Ok(preview) => { d.legacy_preview = Some(Box::new(preview)); d.error.clear(); }
                        Err(e) => d.error = e,
                    }
                }
                if let Some(preview) = &d.legacy_preview {
                    let b = &preview.budget;
                    let height = (ctx.screen_rect().height() - 330.).clamp(120., 420.);
                    egui::ScrollArea::vertical().id_salt("legacy_preview_rows").max_height(height).show(ui, |ui| {
                        ui.strong(format!("{} • {}", b.name, b.base));
                        ui.label(format!("{} accounts • {} categories • {} transactions • {} scheduled rows", b.accounts.len(), b.categories.len(), b.transactions.len(), b.scheduled.len()));
                        ui.label(format!("{} transfer pairs • {} splits • {} pending imports • {} monthly assignments • {} notes", b.transactions.iter().flat_map(|t| t.transfer_links()).count()/2, b.transactions.iter().filter(|t| !t.splits.is_empty()).count(), b.register_meta.len(), b.allocations.len(), b.notes.len()));
                        ui.label(format!("{} zero-amount transactions • {} zero-amount scheduled rows", b.transactions.iter().filter(|t| t.amount == 0).count(), b.scheduled.iter().filter(|s| s.transaction.amount == 0).count()));
                        ui.label(format!("Snapshot: {}", preview.snapshot.display()));
                        ui.label("Original category names and groups are preserved. This imports a complete budget; categories are not merged with the current budget.");
                        ui.collapsing("Account mapping and balances", |ui| {
                            for a in &b.accounts {
                                ui.label(format!("{} → {} / {}{} / {}", a.name, account_kind(&a.account_type), if a.on_budget { "Budget" } else { "Off Budget" }, if a.closed { " / Closed" } else { "" }, money(b.balance(&a.id,None,false), &a.currency)));
                            }
                        });
                        ui.collapsing("Category mapping and hidden categories", |ui| {
                            for c in &b.categories { ui.label(format!("{}: {}{}", c.group, c.name, if c.hidden { " (hidden)" } else { "" })); }
                        });
                        ui.collapsing("Categorized transfers across the budget boundary", |ui| {
                            for t in b.transactions.iter().chain(b.scheduled.iter().map(|s| &s.transaction))
                                .filter(|t| t.transfer.is_some() && b.has_category_flow(t)) {
                                ui.label(format!("{} / {} / {} / {}", t.date, t.payee, money(t.amount, &b.base), b.transaction_category_name(t)));
                            }
                        });
                        ui.collapsing("Income timing, splits and schedules", |ui| {
                            for t in b.transactions.iter().filter(|t| t.income_month.is_some() || !t.splits.is_empty()) {
                                ui.label(format!("{} / {} / {} / {}",t.date,t.payee,money(t.amount,&b.base),b.transaction_category_name(t)));
                                for part in &t.splits { ui.label(format!("  {} / {} / {} / {}", b.split_payee_name(t,part), b.split_category_name(part), money(part.amount,&b.base), part.memo)); }
                            }
                            for s in &b.scheduled {
                                ui.label(format!("{} / {} / {} / {}",s.next_date,s.transaction.payee,money(s.transaction.amount,&b.base),s.frequency().label()));
                                for part in &s.transaction.splits { ui.label(format!("  {} / {} / {} / {}",b.split_payee_name(&s.transaction,part),b.split_category_name(part),money(part.amount,&b.base),part.memo)); }
                            }
                        });
                        let matched = b.register_meta.values().filter(|m| m.matched.is_some()).count();
                        ui.collapsing(format!("Matched bank records ({matched})"), |ui| {
                            for t in &b.transactions {
                                if let Some(state) = b.register_meta.get(&t.id)
                                    && let Some(pair) = &state.matched {
                                    ui.label(format!("{} / {} / {} / {}", t.date, t.payee, money(t.amount, &b.base), if state.pending { "Awaiting approval" } else { "Approved" }));
                                    ui.label(format!("Bank: {} / {} • Your entry: {} / {}", pair.imported.date, pair.imported.payee, pair.manual.date, pair.manual.payee));
                                }
                            }
                        });
                        for warning in &preview.warnings { ui.colored_label(DARK, format!("Review: {warning}")); }
                    });
                    ui.checkbox(&mut d.check, "Replace current budget (can be undone)");
                }
                if !d.error.is_empty() { ui.colored_label(RED, crate::i18n::tr(ui.ctx(), &d.error)); }
            });
            chrome::footer(ui, 690., |ui| {
                if ui.add_enabled(d.legacy_preview.is_some() && d.check, classic::Button::new("Import Budget").primary()).clicked() { apply = true; }
                if classic::button(ui,"Cancel").clicked() { keep = false; }
            });
        });
        if apply {
            match self.save_dialog(&d) {
                Ok(()) => keep = false,
                Err(e) => {
                    d.error = e;
                    d.legacy_preview = None;
                    d.check = false;
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            keep = false;
        }
        if keep {
            self.dialog = Some(d);
        }
    }
}

fn account_kind(kind: &str) -> &str {
    match kind {
        "checking" => "Checking",
        "savings" => "Savings",
        "credit" => "Credit Card",
        "cash" => "Cash",
        "line_of_credit" => "Line of Credit",
        "paypal" => "PayPal",
        "merchant" => "Merchant Account",
        "investment" => "Investment Account",
        "mortgage" => "Mortgage",
        "asset" => "Other Asset",
        "liability" => "Other Liability",
        _ => kind,
    }
}
