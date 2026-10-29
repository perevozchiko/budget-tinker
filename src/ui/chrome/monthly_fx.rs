use super::*;

pub(in crate::ui) struct MonthlyEditor {
    month: String,
    currency: String,
    rate: String,
    date: String,
    source: String,
    reason: String,
    error: String,
    preview: Option<MonthlyCorrectionPreview>,
    finalize: bool,
}

fn rate_text(rate: i64) -> String {
    format!("{}.{:08}", rate / RATE_SCALE, rate % RATE_SCALE)
}

impl MonthlyEditor {
    pub(super) fn new(b: &Budget) -> Self {
        let mut editor = Self {
            month: current_month(),
            currency: CURRENCIES
                .iter()
                .find(|c| **c != b.base)
                .unwrap()
                .to_string(),
            rate: String::new(),
            date: String::new(),
            source: String::new(),
            reason: String::new(),
            error: String::new(),
            preview: None,
            finalize: false,
        };
        editor.load(b);
        editor
    }

    fn load(&mut self, b: &Budget) {
        let quote = b.snapshot(&self.month, &self.currency).ok();
        self.rate = quote.map(|s| rate_text(s.rate)).unwrap_or_default();
        self.date = quote
            .map(|s| s.rate_date.clone())
            .unwrap_or_else(|| Budget::month_end(&self.month).min(today()));
        self.source = quote.map(|s| s.source.clone()).unwrap_or_default();
        self.reason.clear();
        self.error.clear();
        self.preview = None;
        self.finalize = false;
    }

    fn quote(&self) -> Result<MonthlyQuote> {
        Ok(MonthlyQuote {
            rate: parse_fixed(&self.rate, 8)?,
            rate_date: self.date.trim().into(),
            source: self.source.trim().into(),
        })
    }
}

impl App {
    pub(in crate::ui) fn monthly_fx_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut editor) = self.chrome.monthly.take() else {
            return;
        };
        let mut close = false;
        let mut result = Ok(());
        let mut acted = false;
        let response = modal(ctx, "monthly_fx").show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            header(ui, "Monthly exchange rates", 760.);
            body(ui, 760., |ui| {
                ui.spacing_mut().item_spacing.y = 8.;
                if let Some(preview) = &editor.preview {
                    ui.label(format!("Confirm correction: {} / {}", editor.month, editor.currency));
                    if let Ok(old) = self.b.snapshot(&editor.month, &editor.currency) {
                        ui.label(format!("Previous: {} {} per {} · {} · {}", rate_text(old.rate), self.b.base, editor.currency, old.rate_date, old.source));
                    } else { ui.label("Previous: no monthly quote"); }
                    ui.label(format!("New: {} {} per {} · {} · {}", editor.rate, self.b.base, editor.currency, editor.date, editor.source));
                    ui.label(format!("Reason: {}", editor.reason));
                    ui.label("Frozen operation FX and category flows stay unchanged. Values below are in the budget currency; Unavailable means a required comparison quote is missing.");
                    for (before, after) in preview.before.iter().zip(&preview.after) {
                        ui.separator();
                        ui.strong(&before.month);
                        egui::Grid::new(("impact", &before.month)).show(ui, |ui| {
                            ui.label("Valuation"); ui.label("Before"); ui.label("After"); ui.end_row();
                            for (label, old, new) in [
                                ("Net Worth", before.net_worth, after.net_worth),
                                ("On-budget stock", before.budget_value, after.budget_value),
                                ("Available", before.available, after.available),
                                ("Revaluation (all accounts)", before.revaluation_all, after.revaluation_all),
                                ("Revaluation (on budget)", before.revaluation_budget, after.revaluation_budget),
                            ] {
                                ui.label(label);
                                for value in [old, new] { ui.label(value.map(|v| money(v, &self.b.base)).unwrap_or_else(|| "Unavailable".into())); }
                                ui.end_row();
                            }
                        });
                    }
                } else if editor.finalize {
                    ui.label(format!("Finalize {}?", editor.month));
                    ui.label("This records explicit month-end approval. Later quote changes require a historical correction with a reason and valuation preview. Frozen operation FX stays unchanged.");
                } else {
                    ui.horizontal(|ui| {
                        let previous = (editor.month.clone(), editor.currency.clone());
                        if classic::button(ui, "Previous month").clicked() { editor.month = month_shift(&editor.month, -1); }
                        ui.strong(&editor.month);
                        if ui.add_enabled(editor.month < current_month(), classic::Button::new("Next month")).clicked() { editor.month = month_shift(&editor.month, 1); }
                        egui::ComboBox::from_id_salt("monthly_currency").selected_text(&editor.currency).show_ui(ui, |ui| {
                            for currency in CURRENCIES { if *currency != self.b.base { ui.selectable_value(&mut editor.currency, currency.to_string(), *currency); } }
                        });
                        if previous != (editor.month.clone(), editor.currency.clone()) { editor.load(&self.b); }
                    });
                    let status = self.b.monthly_valuation_status(&editor.month);
                    ui.label(format!("Valuation status: {}", match status { Ok(ValuationStatus::Incomplete) => "Incomplete", Ok(ValuationStatus::Finalized) => "Finalized", _ => "Provisional" }));
                    ui.label("Monthly rates value month-end balances. Operation FX remains frozen independently.");
                    egui::Grid::new("monthly_quote_fields").spacing([12., 8.]).show(ui, |ui| {
                        ui.label(format!("1 {} = (in {})", editor.currency, self.b.base));
                        ui.add(egui::TextEdit::singleline(&mut editor.rate).hint_text("Rate").desired_width(280.)); ui.end_row();
                        ui.label("Quote date (YYYY-MM-DD)"); ui.text_edit_singleline(&mut editor.date); ui.end_row();
                        ui.label("Source"); ui.text_edit_singleline(&mut editor.source); ui.end_row();
                        if editor.month < current_month() { ui.label("Correction reason"); ui.text_edit_singleline(&mut editor.reason); ui.end_row(); }
                    });
                    ui.separator();
                    ui.strong("Account valuation preview");
                    for account in self.b.accounts.iter().filter(|a| a.currency == editor.currency) {
                        if let Ok(value) = self.b.account_valuation_preview(&account.id, &editor.month) {
                            let amount = value.base_amount.map(|v| money(v, &self.b.base)).unwrap_or_else(|| "Unavailable".into());
                            let provenance = match (value.quote_month, value.quote_date) { (Some(month), Some(date)) => format!(" · {month} / {date}"), _ => String::new() };
                            ui.label(format!("{}: {} → {}{}{}", account.name, money(value.native_amount, &account.currency), amount, if value.estimated { " · Estimated (earlier quote)" } else { "" }, provenance));
                        }
                    }
                    for audit in self.b.monthly_fx.corrections.iter().rev().filter(|a| a.month == editor.month && a.currency == editor.currency) {
                        ui.label(format!("Correction · {} · {} · {}", audit.recorded_at, audit.reason, audit.after.source));
                    }
                }
                if !editor.error.is_empty() { ui.colored_label(RED, &editor.error); }
            });
            footer(ui, 760., |ui| {
                if editor.preview.is_some() {
                    if classic::button(ui, "Confirm correction").clicked() {
                        let mut next = self.b.clone();
                        acted = true;
                        result = next.apply_monthly_correction(editor.preview.as_ref().unwrap().clone()).and_then(|()| self.commit(next));
                        if result.is_ok() { editor.preview = None; editor.load(&self.b); }
                    }
                    if classic::button(ui, "Back").clicked() { editor.preview = None; editor.error.clear(); }
                } else if editor.finalize {
                    if classic::button(ui, "Confirm finalization").clicked() {
                        let mut next = self.b.clone();
                        acted = true;
                        result = next.finalize_valuation_month(&editor.month).and_then(|()| self.commit(next));
                        if result.is_ok() { editor.finalize = false; }
                    }
                    if classic::button(ui, "Back").clicked() { editor.finalize = false; }
                } else {
                    if classic::button(ui, if editor.month == current_month() { "Save rate" } else { "Preview correction" }).clicked() {
                        acted = true;
                        result = editor.quote().and_then(|quote| {
                            if editor.month == current_month() {
                                let mut next = self.b.clone();
                                next.edit_current_monthly_quote(&editor.currency, quote)?;
                                self.commit(next)
                            } else {
                                editor.preview = Some(self.b.preview_monthly_correction(&editor.month, &editor.currency, quote, &editor.reason)?);
                                Ok(())
                            }
                        });
                    }
                    let can_finalize = editor.month < current_month() && matches!(self.b.monthly_valuation_status(&editor.month), Ok(ValuationStatus::Provisional));
                    if ui.add_enabled(can_finalize, classic::Button::new("Finalize month…")).clicked() { editor.finalize = true; }
                }
                close = classic::button(ui, "Close").clicked();
            });
        });
        if let Err(error) = result {
            editor.error = error;
        } else if acted {
            editor.error.clear();
        }
        close |= response.should_close()
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if !close {
            self.chrome.monthly = Some(editor);
        }
    }
}

#[cfg(test)]
mod tests;
