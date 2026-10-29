use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Reserved report selection key; real categories use UUIDs.
pub const UNCATEGORIZED_CATEGORY: &str = "__uncategorized__";

static EMPTY_IDS: Vec<String> = Vec::new();

/// Multi-select state for categories, payees, and accounts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Selection {
    /// All items are selected (equivalent to the old "None" single-choice).
    #[default]
    All,
    /// Only on-budget accounts — used as the default for spending reports.
    BudgetAccounts,
    /// No items are selected — produces empty results.
    NoneSelected,
    /// Explicitly selected items by ID.
    Selected(Vec<String>),
}

impl Selection {
    /// Returns true when *every* item in the universe is selected.
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Returns true when no items are selected.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::NoneSelected) || matches!(self, Self::Selected(v) if v.is_empty())
    }

    /// Returns the list of selected IDs. For `All` and `BudgetAccounts` this
    /// is `None` (meaning "everything"), for `NoneSelected` this is `Some([])`,
    /// and for `Selected` this is `Some(vec)`.
    pub fn ids(&self) -> Option<&Vec<String>> {
        match self {
            Self::All | Self::BudgetAccounts => None,
            Self::NoneSelected => Some(&EMPTY_IDS),
            Self::Selected(v) => Some(v),
        }
    }

    /// Returns true when the given string value is included by this selection.
    pub fn matches(&self, value: &str) -> bool {
        match self.ids() {
            None => true, // All
            Some(ids) => ids.contains(&value.to_owned()),
        }
    }

    /// Returns true when the optional category ID is included by this selection.
    pub fn matches_category(&self, id: &Option<String>) -> bool {
        match self.ids() {
            None => true, // All
            Some(ids) => ids
                .iter()
                .any(|selected| selected == id.as_deref().unwrap_or(UNCATEGORIZED_CATEGORY)),
        }
    }

    /// Returns a display label for the current selection.
    pub fn label(&self) -> &str {
        match self {
            Self::All => "All",
            Self::BudgetAccounts => "Budget Accounts",
            Self::NoneSelected => "None",
            Self::Selected(v) => {
                if v.is_empty() {
                    "None"
                } else {
                    &v[0]
                }
            }
        }
    }

    /// Returns display names for the selected items (up to 3).
    pub fn display_names(&self, names: &[&str]) -> String {
        match self {
            Self::All => "All".to_string(),
            Self::BudgetAccounts => "Budget Accounts".to_string(),
            Self::NoneSelected => "None".to_string(),
            Self::Selected(ids) => {
                let shown: Vec<&str> = ids
                    .iter()
                    .filter_map(|id| names.iter().find(|n| **n == *id).copied())
                    .take(3)
                    .collect();
                if shown.len() == ids.len() {
                    shown.join(", ").to_string()
                } else if shown.is_empty() {
                    "None".to_string()
                } else {
                    format!("{} ({} more)", shown.join(", "), ids.len() - shown.len())
                }
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, String>;
pub const RATE_SCALE: i64 = 100_000_000;
pub const CURRENCIES: &[&str] = &[
    "EUR", "RSD", "USD", "RUB", "GBP", "CHF", "CAD", "AUD", "JPY", "KWD",
];
pub fn id() -> String {
    Uuid::new_v4().to_string()
}
pub fn today() -> String {
    Local::now().date_naive().to_string()
}
pub fn current_month() -> String {
    today()[..7].into()
}
pub fn valid_date(s: &str) -> bool {
    s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok_and(|d| d.to_string() == s)
}
pub fn month_shift(month: &str, delta: i32) -> String {
    let d = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").expect("validated month");
    let n = d.year() * 12 + d.month0() as i32 + delta;
    format!("{:04}-{:02}", n.div_euclid(12), n.rem_euclid(12) + 1)
}
pub fn valid_month(s: &str) -> bool {
    s.len() == 7 && NaiveDate::parse_from_str(&format!("{s}-01"), "%Y-%m-%d").is_ok()
}
pub fn decimals(currency: &str) -> u32 {
    match currency {
        "JPY" => 0,
        "KWD" => 3,
        _ => 2,
    }
}
pub fn parse_fixed(s: &str, digits: u32) -> Result<i64> {
    let s = s.trim().replace(',', ".");
    let negative = s.starts_with('-');
    let raw = s.strip_prefix(['-', '+']).unwrap_or(&s);
    let parts: Vec<_> = raw.split('.').collect();
    if parts.len() > 2 || parts[0].is_empty() || !parts[0].bytes().all(|c| c.is_ascii_digit()) {
        return Err("Введите число, например 1234,56".into());
    }
    let fraction = parts.get(1).copied().unwrap_or("");
    if fraction.len() > digits as usize || !fraction.bytes().all(|c| c.is_ascii_digit()) {
        return Err(format!("Допустимо знаков после запятой: {digits}"));
    }
    let whole: i64 = parts[0].parse().map_err(|_| "Слишком большая сумма")?;
    let frac: i64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().map_err(|_| "Некорректная дробь")?
    };
    let amount = whole
        .checked_mul(10_i64.pow(digits))
        .and_then(|v| v.checked_add(frac * 10_i64.pow(digits - fraction.len() as u32)))
        .ok_or("Слишком большая сумма")?;
    if amount > 100_000_000_000_000 {
        return Err("Слишком большая сумма".into());
    }
    Ok(if negative { -amount } else { amount })
}
pub fn amount(s: &str, currency: &str) -> Result<i64> {
    parse_fixed(s, decimals(currency))
}
pub fn plain(value: i64, currency: &str) -> String {
    let digits = decimals(currency);
    let scale = 10_i64.pow(digits);
    let abs = value.unsigned_abs();
    let sign = if value < 0 { "-" } else { "" };
    if digits == 0 {
        format!("{sign}{abs}")
    } else {
        format!(
            "{sign}{}.{:0width$}",
            abs / scale as u64,
            abs % scale as u64,
            width = digits as usize
        )
    }
}
pub fn money(value: i64, currency: &str) -> String {
    format!("{} {currency}", plain(value, currency))
}
pub fn convert(value: i64, from: &str, to: &str, rate: i64) -> Result<i64> {
    if rate <= 0 {
        return Err("Курс должен быть положительным".into());
    }
    let numerator = value as i128 * rate as i128 * 10_i128.pow(decimals(to));
    let denominator = RATE_SCALE as i128 * 10_i128.pow(decimals(from));
    let rounded = if numerator < 0 {
        -((-numerator + denominator / 2) / denominator)
    } else {
        (numerator + denominator / 2) / denominator
    };
    i64::try_from(rounded).map_err(|_| "Переполнение при конвертации".into())
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub on_budget: bool,
    pub closed: bool,
    #[serde(default = "default_account_type")]
    pub account_type: String,
}
fn default_account_type() -> String {
    "checking".into()
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Category {
    pub id: String,
    pub group: String,
    pub name: String,
    #[serde(default)]
    pub hidden: bool,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Split {
    pub category: Option<String>,
    pub amount: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub income_month: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payee: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub memo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer: Option<SplitTransfer>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SplitTransfer {
    pub link: String,
    pub account: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CategoryGroup {
    pub name: String,
    pub hidden: bool,
}
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransactionFlag {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
}
impl TransactionFlag {
    pub const ALL: [Self; 6] = [
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub id: String,
    pub account: String,
    pub date: String,
    pub payee: String,
    pub memo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_number: Option<String>,
    pub amount: i64,
    pub base_amount: i64,
    pub snapshot: Option<String>,
    pub category: Option<String>,
    pub cleared: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag: Option<TransactionFlag>,
    /// Budget month for uncategorized income; absent in legacy documents (date month).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub income_month: Option<String>,
    pub reconciled: bool,
    pub transfer: Option<String>,
    pub splits: Vec<Split>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Snapshot {
    pub id: String,
    pub month: String,
    pub currency: String,
    pub rate: i64,
    pub rate_date: String,
    pub captured_at: String,
    pub source: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Scheduled {
    pub id: String,
    pub transaction: Transaction,
    pub next_date: String,
    pub monthly: bool,
    #[serde(default)]
    pub repeat: Option<crate::register::Repeat>,
    #[serde(default)]
    pub anchor_day: Option<u32>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PayeeRule {
    pub listed: bool,
    pub category: Option<String>,
    pub rename_contains: String,
}
impl Default for PayeeRule {
    fn default() -> Self {
        Self {
            listed: true,
            category: None,
            rename_contains: String::new(),
        }
    }
}
#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum OverspendingRule {
    AvailableToBudget,
    // Documents written before these rules always carried negative balances.
    #[default]
    CategoryBalance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthSummary {
    pub not_budgeted: i64,
    /// Negative balances charged to this month's Available to Budget.
    pub overspent: i64,
    /// Includes uncategorized cash movements and currency revaluation.
    pub income: i64,
    pub budgeted: i64,
    pub available: i64,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Budget {
    pub version: u32,
    pub name: String,
    pub base: String,
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    #[serde(default)]
    pub category_groups: Vec<CategoryGroup>,
    pub transactions: Vec<Transaction>,
    pub allocations: BTreeMap<String, i64>,
    #[serde(default)]
    pub default_overspending_rule: OverspendingRule,
    /// Category rules take effect in this month, until the next explicit change.
    #[serde(default)]
    pub overspending_rules: BTreeMap<String, OverspendingRule>,
    pub snapshots: Vec<Snapshot>,
    pub scheduled: Vec<Scheduled>,
    #[serde(default)]
    pub payee_rules: BTreeMap<String, PayeeRule>,
    #[serde(default)]
    pub notes: BTreeMap<String, String>,
    #[serde(default)]
    pub register_meta: BTreeMap<String, crate::register::ImportState>,
}

mod accounts;
mod allocations;
mod categories;
mod initialization;
mod quick_budget;
mod reconciliation;
mod schedules;
mod totals;
pub use quick_budget::QuickBudget;
mod transactions;
mod valuation;
