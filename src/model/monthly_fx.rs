//! Monthly stock valuation is independent of frozen operation postings.
use super::*;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MonthlyFxState {
    #[serde(default)]
    pub finalized: BTreeMap<String, String>,
    #[serde(default)]
    pub corrections: Vec<RateCorrection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonthlyQuote {
    pub rate: i64,
    pub rate_date: String,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateCorrection {
    pub month: String,
    pub currency: String,
    pub before: Option<MonthlyQuote>,
    pub after: MonthlyQuote,
    pub reason: String,
    pub recorded_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValuationStatus {
    Incomplete,
    Provisional,
    Finalized,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValuationImpact {
    pub month: String,
    pub net_worth: Option<i64>,
    pub budget_value: Option<i64>,
    pub available: Option<i64>,
    pub revaluation_all: Option<i64>,
    pub revaluation_budget: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountValuationPreview {
    pub native_amount: i64,
    pub base_amount: Option<i64>,
    pub estimated: bool,
    pub quote_month: Option<String>,
    pub quote_date: Option<String>,
}

/// Opaque proposal binds approval to the entire source document, including
/// accounts and postings that determine the displayed valuation impact.
#[derive(Clone, Debug)]
pub struct MonthlyCorrectionPreview {
    source: String,
    candidate: Budget,
    pub before: Vec<ValuationImpact>,
    pub after: Vec<ValuationImpact>,
}

fn quote(snapshot: &Snapshot) -> MonthlyQuote {
    MonthlyQuote {
        rate: snapshot.rate,
        rate_date: snapshot.rate_date.clone(),
        source: snapshot.source.clone(),
    }
}

fn validate_quote(month: &str, currency: &str, base: &str, value: &MonthlyQuote) -> Result<()> {
    if !valid_month(month)
        || !CURRENCIES.contains(&currency)
        || currency == base
        || value.rate <= 0
        || value.rate > 100_000_000_000_000
        || !valid_date(&value.rate_date)
        || value.rate_date > Budget::month_end(month)
        || value.rate_date > today()
    {
        return Err("Invalid monthly quote, date or source".into());
    }
    Ok(())
}

impl Budget {
    pub(crate) fn validate_monthly_fx(&self) -> Result<()> {
        if self.version != 2
            && (!self.monthly_fx.finalized.is_empty() || !self.monthly_fx.corrections.is_empty())
        {
            return Err("Monthly FX lifecycle requires currency data v2".into());
        }
        for (month, timestamp) in &self.monthly_fx.finalized {
            let recorded = chrono::DateTime::parse_from_rfc3339(timestamp)
                .map_err(|_| "Invalid month finalization timestamp")?;
            if !valid_month(month) || Self::month_end(month) >= recorded.date_naive().to_string() {
                return Err("A month can only be finalized after month end".into());
            }
            if self.monthly_valuation_status(month)? == ValuationStatus::Incomplete {
                return Err("A finalized month requires every exposure quote".into());
            }
        }
        for correction in &self.monthly_fx.corrections {
            validate_quote(
                &correction.month,
                &correction.currency,
                &self.base,
                &correction.after,
            )?;
            if let Some(before) = &correction.before {
                validate_quote(&correction.month, &correction.currency, &self.base, before)?;
            }
            if correction.after.source.trim().is_empty()
                || correction.reason.trim().is_empty()
                || chrono::DateTime::parse_from_rfc3339(&correction.recorded_at).is_err()
            {
                return Err("Invalid historical correction audit".into());
            }
        }
        Ok(())
    }

    pub fn monthly_valuation_status(&self, month: &str) -> Result<ValuationStatus> {
        if !valid_month(month) {
            return Err("Invalid valuation month".into());
        }
        let end = Self::month_end(month);
        for account in &self.accounts {
            if account.currency != self.base
                && self.balance(&account.id, Some(&end), false) != 0
                && self.snapshot(month, &account.currency).is_err()
            {
                return Ok(ValuationStatus::Incomplete);
            }
        }
        Ok(if self.monthly_fx.finalized.contains_key(month) {
            ValuationStatus::Finalized
        } else {
            ValuationStatus::Provisional
        })
    }

    /// Preview only: an earlier quote never becomes an authoritative monthly rate.
    pub fn account_valuation_preview(
        &self,
        account: &str,
        month: &str,
    ) -> Result<AccountValuationPreview> {
        if !valid_month(month) {
            return Err("Invalid valuation month".into());
        }
        let account = self.account(account)?;
        let native_amount = self.balance(&account.id, Some(&Self::month_end(month)), false);
        let mut result = AccountValuationPreview {
            native_amount,
            base_amount: Some(native_amount),
            estimated: false,
            quote_month: None,
            quote_date: None,
        };
        if account.currency == self.base || native_amount == 0 {
            return Ok(result);
        }
        let snapshot = self
            .snapshots
            .iter()
            .filter(|s| s.currency == account.currency && s.month.as_str() <= month)
            .max_by_key(|s| &s.month);
        result.base_amount = snapshot
            .map(|s| convert(native_amount, &account.currency, &self.base, s.rate))
            .transpose()?;
        if let Some(snapshot) = snapshot {
            result.estimated = snapshot.month != month;
            result.quote_month = Some(snapshot.month.clone());
            result.quote_date = Some(snapshot.rate_date.clone());
        }
        Ok(result)
    }

    fn optional_stock_value(&self, month: &str, on_budget: bool) -> Result<Option<i64>> {
        let end = Self::month_end(month);
        if self.accounts.iter().any(|a| {
            (!on_budget || a.on_budget)
                && a.currency != self.base
                && self.balance(&a.id, Some(&end), false) != 0
                && self.snapshot(month, &a.currency).is_err()
        }) {
            return Ok(None);
        }
        self.net_worth(month, on_budget).map(Some)
    }

    fn replace_monthly_quote(
        &mut self,
        month: &str,
        currency: &str,
        value: &MonthlyQuote,
    ) -> Result<()> {
        if self.version != 2 {
            return Err("Upgrade currency data before editing monthly quotes".into());
        }
        validate_quote(month, currency, &self.base, value)?;
        if value.source.trim().is_empty() {
            return Err("Enter the monthly quote source".into());
        }
        if let Some(snapshot) = self
            .snapshots
            .iter_mut()
            .find(|s| s.month == month && s.currency == currency)
        {
            snapshot.rate = value.rate;
            snapshot.rate_date = value.rate_date.clone();
            snapshot.source = value.source.clone();
            snapshot.captured_at = Local::now().to_rfc3339();
        } else {
            self.add_snapshot(month, currency, value.rate, &value.rate_date, &value.source)?;
        }
        Ok(())
    }

    pub fn edit_current_monthly_quote(
        &mut self,
        currency: &str,
        value: MonthlyQuote,
    ) -> Result<()> {
        let month = current_month();
        if self.monthly_fx.finalized.contains_key(&month) {
            return Err("Finalized quotes require a historical correction".into());
        }
        let mut candidate = self.clone();
        candidate.replace_monthly_quote(&month, currency, &value)?;
        crate::store::validate(&candidate)?;
        *self = candidate;
        Ok(())
    }

    pub fn finalize_valuation_month(&mut self, month: &str) -> Result<()> {
        if self.version != 2 || !valid_month(month) || month >= current_month().as_str() {
            return Err("Finalize a v2 valuation month only after month end".into());
        }
        if self.monthly_fx.finalized.contains_key(month) {
            return Err("Valuation month is already finalized".into());
        }
        if self.monthly_valuation_status(month)? == ValuationStatus::Incomplete {
            return Err("Enter all required monthly quotes before finalizing".into());
        }
        let mut candidate = self.clone();
        candidate
            .monthly_fx
            .finalized
            .insert(month.into(), Local::now().to_rfc3339());
        crate::store::validate(&candidate)?;
        *self = candidate;
        Ok(())
    }

    /// Residual stock valuation, never an earned-income calculation. An absent
    /// comparison valuation remains unavailable, rather than becoming zero.
    fn valuation_impact(&self, month: &str) -> Result<ValuationImpact> {
        let previous = month_shift(month, -1);
        let net_worth = self.optional_stock_value(month, false)?;
        let budget_value = self.optional_stock_value(month, true)?;
        let revaluation = |on_budget| -> Result<Option<i64>> {
            let (Some(now), Some(before)) = (
                self.optional_stock_value(month, on_budget)?,
                self.optional_stock_value(&previous, on_budget)?,
            ) else {
                return Ok(None);
            };
            let flows = self
                .transactions
                .iter()
                .filter(|t| {
                    t.date.starts_with(month)
                        && (!on_budget || self.account(&t.account).is_ok_and(|a| a.on_budget))
                })
                .map(|t| i128::from(t.base_amount))
                .sum::<i128>();
            Ok(Some(
                i64::try_from(i128::from(now) - i128::from(before) - flows)
                    .map_err(|_| "Revaluation overflow")?,
            ))
        };
        Ok(ValuationImpact {
            month: month.into(),
            net_worth,
            budget_value,
            available: if budget_value.is_some() {
                Some(self.available(month)?)
            } else {
                None
            },
            revaluation_all: revaluation(false)?,
            revaluation_budget: revaluation(true)?,
        })
    }

    pub fn preview_monthly_correction(
        &self,
        month: &str,
        currency: &str,
        value: MonthlyQuote,
        reason: &str,
    ) -> Result<MonthlyCorrectionPreview> {
        if !valid_month(month)
            || month > current_month().as_str()
            || (month == current_month() && !self.monthly_fx.finalized.contains_key(month))
            || reason.trim().is_empty()
        {
            return Err("Historical correction requires a past month and a reason".into());
        }
        crate::store::validate(self)?;
        let mut candidate = self.clone();
        let before_quote = self.snapshot(month, currency).ok().map(quote);
        candidate.replace_monthly_quote(month, currency, &value)?;
        candidate.monthly_fx.corrections.push(RateCorrection {
            month: month.into(),
            currency: currency.into(),
            before: before_quote,
            after: value,
            reason: reason.trim().into(),
            recorded_at: Local::now().to_rfc3339(),
        });
        crate::store::validate(&candidate)?;
        let months = [month.to_string(), month_shift(month, 1)];
        Ok(MonthlyCorrectionPreview {
            source: serde_json::to_string(self).map_err(|e| e.to_string())?,
            before: months
                .iter()
                .map(|m| self.valuation_impact(m))
                .collect::<Result<_>>()?,
            after: months
                .iter()
                .map(|m| candidate.valuation_impact(m))
                .collect::<Result<_>>()?,
            candidate,
        })
    }

    pub fn apply_monthly_correction(&mut self, preview: MonthlyCorrectionPreview) -> Result<()> {
        if serde_json::to_string(self).map_err(|e| e.to_string())? != preview.source {
            return Err("Budget changed; preview the historical correction again".into());
        }
        crate::store::validate(&preview.candidate)?;
        *self = preview.candidate;
        Ok(())
    }
}
