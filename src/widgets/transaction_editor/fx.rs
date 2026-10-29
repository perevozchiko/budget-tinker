use super::*;
use budgettinker::model::operation_fx::{OperationRate, RateOrigin};

fn rate_text(rate: i64) -> String {
    format!("{}.{:08}", rate / RATE_SCALE, rate % RATE_SCALE)
}

impl Draft {
    fn load_fx(&mut self, b: &Budget) {
        self.fx = FxDraft {
            context: (self.account.clone(), self.date.clone()),
            date: self.date.clone(),
            ..Default::default()
        };
        let Ok(account) = b.account(&self.account) else {
            return;
        };
        if let Some(t) = self.id.as_ref().and_then(|id| {
            b.transactions
                .iter()
                .find(|t| &t.id == id && t.account == self.account)
        }) && let Some(fx) = &t.operation_fx
        {
            self.fx.suggestion = fx.rate.clone();
            self.fx.exact = fx.exact_base || fx.split_override;
            self.fx.base = plain(fx.base_amount, &b.base);
            self.fx.parts = fx
                .splits
                .iter()
                .map(|s| plain(s.base_amount, &b.base))
                .collect();
        } else if valid_date(&self.date)
            && let Some(s) = b
                .snapshots
                .iter()
                .filter(|s| s.currency == account.currency && s.month.as_str() <= &self.date[..7])
                .max_by_key(|s| &s.month)
        {
            self.fx.stale = s.month != self.date[..7];
            self.fx.suggestion = Some(OperationRate {
                rate: s.rate,
                rate_date: s.rate_date.clone(),
                source: s.source.clone(),
                origin: RateOrigin::MonthlyDefault {
                    snapshot_id: s.id.clone(),
                },
            });
        }
        if let Some(rate) = &self.fx.suggestion {
            self.fx.rate = rate_text(rate.rate);
            self.fx.date = rate.rate_date.clone();
        }
    }

    pub(super) fn show_fx(&mut self, ui: &mut egui::Ui, b: &Budget) {
        if b.version != 2 || !b.account(&self.account).is_ok_and(|a| a.currency != b.base) {
            self.fx.enabled = false;
            return;
        }
        if self.fx.context != (self.account.clone(), self.date.clone()) {
            self.load_fx(b);
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.fx.enabled, "Enter operation FX");
            if let Some(t) = self
                .id
                .as_ref()
                .and_then(|id| b.transactions.iter().find(|t| &t.id == id))
            {
                ui.label(format!(
                    "Frozen budget amount: {}",
                    money(t.base_amount, &b.base)
                ));
            } else if self.fx.suggestion.is_none() {
                ui.label("No monthly quote. Enter a rate or exact budget amount.");
            }
        });
        if !self.fx.enabled {
            return;
        }
        let currency = &b.account(&self.account).unwrap().currency;
        ui.horizontal(|ui| {
            ui.label(format!("1 {currency} ="));
            ui.add(
                egui::TextEdit::singleline(&mut self.fx.rate)
                    .desired_width(120.)
                    .hint_text("Rate"),
            );
            ui.label(&b.base);
            ui.label("Quote date");
            ui.add(egui::TextEdit::singleline(&mut self.fx.date).desired_width(95.));
            ui.checkbox(&mut self.fx.exact, "Exact budget amount (signed)");
            if self.fx.exact {
                ui.add(
                    egui::TextEdit::singleline(&mut self.fx.base)
                        .desired_width(100.)
                        .hint_text("Signed base amount"),
                );
                ui.label(&b.base);
            }
        });
        if let Some(rate) = &self.fx.suggestion {
            ui.label(format!(
                "{}: {} · {}. Save confirms this operation only.",
                if self.fx.stale {
                    "Stale suggestion"
                } else {
                    "Copied quote"
                },
                rate.rate_date,
                rate.source
            ));
        } else {
            ui.label("Save freezes this operation only; monthly valuation rates are unchanged.");
        }
        if self.fx.exact {
            self.fx.parts.resize(self.splits.len(), String::new());
            for (i, part) in self.fx.parts.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Split {} budget amount (signed)", i + 1));
                    ui.add(egui::TextEdit::singleline(part).desired_width(100.));
                    ui.label(&b.base);
                });
            }
        }
    }

    pub(super) fn prepare_fx(&self, b: &Budget, t: Transaction) -> Result<Transaction> {
        if self.fx.context != (self.account.clone(), self.date.clone()) {
            return Err("Account or date changed. Review operation FX again.".into());
        }
        let exact = if self.fx.exact {
            Some((
                amount(&self.fx.base, &b.base)?,
                self.fx
                    .parts
                    .iter()
                    .map(|p| amount(p, &b.base))
                    .collect::<Result<Vec<_>>>()?,
            ))
        } else {
            None
        };
        let value = if self.fx.rate.trim().is_empty() {
            let (base, _) = exact
                .as_ref()
                .ok_or("Enter an operation exchange rate or exact budget amount")?;
            if t.amount == 0 || base.signum() != t.amount.signum() {
                return Err("Exact base requires a nonzero movement of the same sign".into());
            }
            let currency = &b.account(&t.account)?.currency;
            let numerator = i128::from(base.unsigned_abs())
                * i128::from(RATE_SCALE)
                * 10_i128.pow(decimals(currency));
            let denominator = i128::from(t.amount.unsigned_abs()) * 10_i128.pow(decimals(&b.base));
            i64::try_from((numerator + denominator / 2) / denominator)
                .map_err(|_| "Exchange rate overflow")?
        } else {
            parse_fixed(&self.fx.rate, 8)?
        };
        let rate = self
            .fx
            .suggestion
            .as_ref()
            .filter(|r| r.rate == value && r.rate_date == self.fx.date)
            .cloned()
            .unwrap_or(OperationRate {
                rate: value,
                rate_date: self.fx.date.clone(),
                source: "Manual operation entry".into(),
                origin: RateOrigin::Manual,
            });
        b.prepare_operation_fx(t, rate, exact)
    }
}
