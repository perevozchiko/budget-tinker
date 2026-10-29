//! Independent operation postings for schema v2 and legacy upgrade preparation.
use super::*;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RateOrigin {
    MonthlyDefault { snapshot_id: String },
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationRate {
    /// Base major units per account major unit, scaled by RATE_SCALE.
    pub rate: i64,
    pub rate_date: String,
    pub source: String,
    /// Provenance only; conversion never dereferences this ID.
    pub origin: RateOrigin,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrozenSplit {
    pub id: String,
    pub native_amount: i64,
    pub base_amount: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrozenOperation {
    pub account_currency: String,
    pub base_currency: String,
    pub native_amount: i64,
    pub base_amount: i64,
    pub rate: Option<OperationRate>,
    /// Explicit exact-base entry; native/base amounts define the exact ratio.
    #[serde(default)]
    pub exact_base: bool,
    /// Explicit split base entries, constrained by the parent total.
    #[serde(default)]
    pub split_override: bool,
    pub splits: Vec<FrozenSplit>,
}

impl FrozenOperation {
    /// The caller must validate the whole legacy document first, including
    /// hidden match originals. No invented quote for an all-zero operation.
    pub(crate) fn from_current(b: &Budget, t: &Transaction) -> Result<Self> {
        let account = b.account(&t.account)?;
        let rate = if let Some(snapshot_id) = &t.snapshot {
            let s = b
                .snapshots
                .iter()
                .find(|s| &s.id == snapshot_id)
                .ok_or("Missing operation exchange snapshot")?;
            Some(OperationRate {
                rate: s.rate,
                rate_date: s.rate_date.clone(),
                source: s.source.clone(),
                origin: RateOrigin::MonthlyDefault {
                    snapshot_id: s.id.clone(),
                },
            })
        } else {
            None
        };
        let mut legacy = t.clone();
        legacy.operation_fx = None;
        let bases = b.split_base_amounts(&legacy)?;
        let frozen = Self {
            account_currency: account.currency.clone(),
            base_currency: b.base.clone(),
            native_amount: t.amount,
            base_amount: t.base_amount,
            rate,
            exact_base: false,
            split_override: false,
            splits: t
                .splits
                .iter()
                .zip(bases)
                .enumerate()
                .map(|(i, (part, base))| FrozenSplit {
                    // Stable within this transaction, including repeated preflight.
                    id: if part.id.is_empty() {
                        format!("{}/split/{i}", t.id)
                    } else {
                        part.id.clone()
                    },
                    native_amount: part.amount,
                    base_amount: base,
                })
                .collect(),
        };
        frozen.validate()?;
        Ok(frozen)
    }

    /// Validate only copied operation data. Monthly valuations are deliberately
    /// absent from this interface, even for MonthlyDefault provenance.
    pub fn validate(&self) -> Result<()> {
        if !CURRENCIES.contains(&self.account_currency.as_str())
            || !CURRENCIES.contains(&self.base_currency.as_str())
            || self.native_amount.unsigned_abs() > 100_000_000_000_000
        {
            return Err("Invalid frozen operation currency or amount".into());
        }
        let foreign = self.account_currency != self.base_currency;
        if !foreign && self.split_override {
            return Err("Base-currency splits use their native amounts".into());
        }
        let movement = self.native_amount != 0 || self.splits.iter().any(|s| s.native_amount != 0);
        if self.rate.is_some() != (foreign && movement) {
            return Err("Frozen FX required exactly for foreign movements".into());
        }
        if let Some(rate) = &self.rate
            && (rate.rate <= 0
                || rate.rate > 100_000_000_000_000
                || !valid_date(&rate.rate_date)
                || matches!(&rate.origin, RateOrigin::MonthlyDefault { snapshot_id } if snapshot_id.is_empty()))
        {
            return Err("Invalid frozen operation rate".into());
        }
        let convert_part = |value| match &self.rate {
            Some(rate) => convert(
                value,
                &self.account_currency,
                &self.base_currency,
                rate.rate,
            ),
            None => Ok(value),
        };
        if self.exact_base
            && (!foreign
                || self.native_amount == 0
                || self.native_amount.signum() != self.base_amount.signum())
        {
            return Err("Exact base requires a nonzero foreign movement of the same sign".into());
        }
        if !self.exact_base && convert_part(self.native_amount)? != self.base_amount {
            return Err("Frozen operation base amount disagrees with its quote".into());
        }
        if self.splits.is_empty() {
            return Ok(());
        }
        if self.exact_base && !self.split_override {
            return Err("Exact-base splits require explicit base parts".into());
        }
        let mut ids = HashSet::new();
        let mut native_total = 0_i128;
        let mut base_total = 0_i128;
        let mut remainder = i128::from(self.base_amount);
        let last = self.splits.iter().rposition(|s| s.native_amount != 0);
        for (index, split) in self.splits.iter().enumerate() {
            if split.id.is_empty()
                || !ids.insert(&split.id)
                || split.native_amount.unsigned_abs() > 100_000_000_000_000
            {
                return Err("Invalid frozen split identity or amount".into());
            }
            let expected = if self.split_override {
                i128::from(split.base_amount)
            } else if split.native_amount == 0 {
                0
            } else if Some(index) == last {
                remainder
            } else {
                i128::from(convert_part(split.native_amount)?)
            };
            if (split.native_amount == 0 && split.base_amount != 0)
                || (!self.split_override && i128::from(split.base_amount) != expected)
            {
                return Err("Frozen split disagrees with deterministic rounding".into());
            }
            native_total += i128::from(split.native_amount);
            base_total += i128::from(split.base_amount);
            remainder -= i128::from(split.base_amount);
        }
        if native_total != i128::from(self.native_amount)
            || base_total != i128::from(self.base_amount)
        {
            return Err("Frozen split totals disagree with the parent".into());
        }
        Ok(())
    }
}

impl FrozenOperation {
    pub fn validate_transaction(&self, b: &Budget, t: &Transaction) -> Result<()> {
        self.validate()?;
        if self.account_currency != b.account(&t.account)?.currency
            || self.base_currency != b.base
            || self.native_amount != t.amount
            || self.base_amount != t.base_amount
            || self.splits.len() != t.splits.len()
            || self
                .splits
                .iter()
                .zip(&t.splits)
                .any(|(a, b)| a.native_amount != b.amount || a.id != b.id)
        {
            return Err("Operation FX does not match its transaction".into());
        }
        Ok(())
    }
}

impl Budget {
    /// Prepare explicitly confirmed ordinary operation FX without requiring or
    /// changing a monthly valuation quote. All work stays in the returned draft.
    pub fn prepare_operation_fx(
        &self,
        mut t: Transaction,
        rate: OperationRate,
        exact: Option<(i64, Vec<i64>)>,
    ) -> Result<Transaction> {
        if self.version != 2 || t.has_transfers() {
            return Err("Manual FX requires an upgraded budget and an ordinary transaction".into());
        }
        if rate.rate <= 0
            || rate.rate > 100_000_000_000_000
            || !valid_date(&rate.rate_date)
            || rate.rate_date > today()
        {
            return Err(
                "Enter a positive operation rate and a quote date no later than today".into(),
            );
        }
        let currency = self.account(&t.account)?.currency.clone();
        if currency == self.base {
            return Err("Base-currency operations use their native amounts".into());
        }
        for part in &mut t.splits {
            if part.id.is_empty() {
                part.id = id();
            }
        }
        let base = convert(t.amount, &currency, &self.base, rate.rate)?;
        let mut remainder = i128::from(base);
        let last = t.splits.iter().rposition(|s| s.amount != 0);
        let mut splits = Vec::new();
        for (i, part) in t.splits.iter().enumerate() {
            let value = if part.amount == 0 {
                0
            } else if Some(i) == last {
                i64::try_from(remainder).map_err(|_| "Split base amount overflow")?
            } else {
                convert(part.amount, &currency, &self.base, rate.rate)?
            };
            remainder -= i128::from(value);
            splits.push(FrozenSplit {
                id: part.id.clone(),
                native_amount: part.amount,
                base_amount: value,
            });
        }
        let mut fx = FrozenOperation {
            account_currency: currency,
            base_currency: self.base.clone(),
            native_amount: t.amount,
            base_amount: base,
            rate: Some(rate),
            exact_base: false,
            split_override: false,
            splits,
        };
        if let Some((base, parts)) = exact {
            if parts.len() != fx.splits.len() {
                return Err("Enter every split base amount".into());
            }
            fx.base_amount = base;
            fx.exact_base = t.amount != 0;
            fx.split_override = !parts.is_empty();
            for (part, value) in fx.splits.iter_mut().zip(parts) {
                part.base_amount = value;
            }
        }
        if t.amount == 0 && t.splits.iter().all(|s| s.amount == 0) {
            fx.rate = None;
        }
        fx.validate()?;
        t.snapshot = None;
        t.base_amount = fx.base_amount;
        t.operation_fx = Some(fx);
        self.prepare_transaction(t)
    }

    /// Freeze a newly priced posting; never used by persistence to repair input.
    pub(crate) fn freeze_posting(&self, t: &mut Transaction) -> Result<()> {
        if self.version == 2 {
            for part in &mut t.splits {
                if part.id.is_empty() {
                    part.id = id();
                }
            }
            t.operation_fx = Some(FrozenOperation::from_current(self, t)?);
        }
        Ok(())
    }

    pub(crate) fn price_posting(&self, t: &mut Transaction) -> Result<()> {
        if self.version == 2 {
            for part in &mut t.splits {
                if part.id.is_empty() {
                    part.id = id();
                }
            }
        }
        if self.version == 2
            && let Some(mut fx) = t.operation_fx.clone()
        {
            if fx.account_currency != self.account(&t.account)?.currency
                || fx.base_currency != self.base
            {
                return Err("Operation currencies cannot change implicitly".into());
            }
            if fx.rate.is_none()
                && fx.account_currency != fx.base_currency
                && (t.amount != 0 || t.splits.iter().any(|s| s.amount != 0))
            {
                t.operation_fx = None;
                (t.base_amount, t.snapshot) = self.transaction_value(t)?;
                self.freeze_posting(t)?;
                return Ok(());
            }
            let unchanged = fx.native_amount == t.amount
                && fx.splits.len() == t.splits.len()
                && fx
                    .splits
                    .iter()
                    .zip(&t.splits)
                    .all(|(a, b)| a.native_amount == b.amount && a.id == b.id);
            if unchanged {
                t.base_amount = fx.base_amount;
                return Ok(());
            }
            if fx.exact_base || fx.split_override {
                return Err(
                    "Confirm new base amounts before changing an exact-base operation".into(),
                );
            }
            let convert_part = |v| match &fx.rate {
                Some(rate) => convert(v, &fx.account_currency, &fx.base_currency, rate.rate),
                None if fx.account_currency == fx.base_currency || v == 0 => Ok(v),
                None => Err("Enter an operation exchange rate".into()),
            };
            fx.native_amount = t.amount;
            fx.base_amount = convert_part(t.amount)?;
            let mut remainder = i128::from(fx.base_amount);
            let last = t.splits.iter().rposition(|s| s.amount != 0);
            let mut splits = Vec::new();
            for (i, part) in t.splits.iter().enumerate() {
                let value = if part.amount == 0 {
                    0
                } else if Some(i) == last {
                    i64::try_from(remainder).map_err(|_| "Split base amount overflow")?
                } else {
                    convert_part(part.amount)?
                };
                remainder -= i128::from(value);
                splits.push(FrozenSplit {
                    id: part.id.clone(),
                    native_amount: part.amount,
                    base_amount: value,
                });
            }
            fx.splits = splits;
            if fx.native_amount == 0 && fx.splits.iter().all(|s| s.native_amount == 0) {
                fx.rate = None;
            }
            fx.validate()?;
            t.base_amount = fx.base_amount;
            t.operation_fx = Some(fx);
        } else {
            (t.base_amount, t.snapshot) = self.transaction_value(t)?;
            self.freeze_posting(t)?;
        }
        Ok(())
    }

    /// Explicit exact-base and split-base command. Applied to a disposable copy,
    /// including validation, so a bad override cannot change the original.
    pub fn set_operation_base(&mut self, tid: &str, base: i64, parts: &[i64]) -> Result<()> {
        if self.version != 2 {
            return Err("Upgrade the budget before entering exact base amounts".into());
        }
        let mut next = self.clone();
        let t = next
            .transactions
            .iter_mut()
            .find(|t| t.id == tid)
            .ok_or("Transaction not found")?;
        if t.reconciled || t.has_transfers() {
            return Err("Exact base editing requires an unreconciled ordinary transaction".into());
        }
        let fx = t.operation_fx.as_mut().ok_or("Missing operation FX")?;
        if parts.len() != fx.splits.len() {
            return Err("Enter every split base amount".into());
        }
        fx.base_amount = base;
        if fx.account_currency == fx.base_currency {
            return Err("Base-currency operations use their native amounts".into());
        }
        fx.exact_base = fx.native_amount != 0;
        fx.split_override = !parts.is_empty();
        for (s, amount) in fx.splits.iter_mut().zip(parts) {
            s.base_amount = *amount;
        }
        t.base_amount = base;
        crate::store::validate(&next)?;
        *self = next;
        Ok(())
    }
}
