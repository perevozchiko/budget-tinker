//! Report calculations shared by the screen, CSV export and print preview.
use std::collections::BTreeMap;

pub use crate::model::Selection;
use crate::model::{
    Account, Budget, Result, Transaction, current_month, month_shift, plain, valid_date,
};

pub const TITLES: [&str; 5] = [
    "Spending By Category",
    "Spending By Payee",
    "Spending Trends",
    "Income v. Expense",
    "Net Worth",
];

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Filter {
    pub from: String,
    pub through: String,
    pub category: Selection,
    pub payee: Selection,
    pub accounts: Selection,
}

impl Default for Filter {
    fn default() -> Self {
        let month = current_month();
        Self {
            from: format!("{month}-01"),
            through: Budget::month_end(&month),
            category: Selection::All,
            payee: Selection::All,
            accounts: Selection::BudgetAccounts,
        }
    }
}

impl Filter {
    /// Returns true when the given account should be included in report data.
    pub fn includes_account(&self, account: &Account) -> bool {
        match self.accounts {
            Selection::BudgetAccounts => account.on_budget,
            Selection::All | Selection::Selected(_) | Selection::NoneSelected => {
                match self.accounts.ids() {
                    None => true,                         // All → all accounts
                    Some(ids) if ids.is_empty() => false, // NoneSelected
                    Some(ids) => ids.contains(&account.id),
                }
            }
        }
    }

    pub fn months(&self) -> Result<Vec<String>> {
        if !valid_date(&self.from) || !valid_date(&self.through) || self.from > self.through {
            return Err("Enter a valid date range (YYYY-MM-DD).".into());
        }
        let mut month = self.from[..7].to_owned();
        let end = &self.through[..7];
        let mut months = vec![];
        while month.as_str() <= end {
            if months.len() == 1200 {
                return Err("Choose a report period of 100 years or less.".into());
            }
            months.push(month.clone());
            if month == end {
                break;
            }
            month = month_shift(&month, 1);
        }
        Ok(months)
    }
}

#[derive(Clone, Debug)]
pub struct Month {
    pub month: String,
    pub income: i64,
    pub expense: i64,
    /// Positive account balances, converted independently to the base currency.
    pub assets: i64,
    /// Negative account balances, converted independently to the base currency.
    pub debts: i64,
    pub net_worth: i64,
}

/// A row of the income/expense matrix. Expenses use their signed cash flow,
/// so a categorized refund offsets spending in the same category.
#[derive(Clone, Debug)]
pub struct MatrixRow {
    pub key: String,
    pub label: String,
    pub values: Vec<i64>,
    pub children: Vec<MatrixRow>,
    average_override: Option<i64>,
}

impl MatrixRow {
    pub fn total(&self) -> i64 {
        self.values.iter().sum()
    }

    /// legacy rounds ties toward positive infinity. Its Net Income average is
    /// the sum of rounded leaf averages rather than the average of its total.
    pub fn average(&self) -> i64 {
        if let Some(average) = self.average_override {
            return average;
        }
        let count = self.values.len() as i128;
        if count == 0 {
            return 0;
        }
        let total = i128::from(self.total());
        ((total + count / 2).div_euclid(count)) as i64
    }
}

fn matrix_row(key: &str, label: &str, values: Vec<i64>) -> MatrixRow {
    MatrixRow {
        key: key.into(),
        label: label.into(),
        values,
        children: vec![],
        average_override: None,
    }
}

/// A navigation target, separate from labels (including real payees named Other).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SpendingScope {
    Group(String),
    Category(Option<String>),
    Payee(String),
    Other(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct SpendingRow {
    pub label: String,
    pub value: i64,
    pub scope: SpendingScope,
}

/// One spending part in base minor units; split transactions retain their part amounts.
#[derive(Clone, Debug)]
pub struct SpendingEntry {
    pub transaction_id: String,
    pub date: String,
    pub account: String,
    pub payee: String,
    pub category: Option<String>,
    pub category_label: String,
    pub group: String,
    pub memo: String,
    pub value: i64,
}

impl SpendingScope {
    pub fn includes(&self, entry: &SpendingEntry) -> bool {
        match self {
            Self::Group(group) => entry.group == *group,
            Self::Category(category) => entry.category == *category,
            Self::Payee(payee) => entry.payee == *payee,
            Self::Other(payees) => payees.contains(&entry.payee),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Report {
    pub kind: usize,
    pub from: String,
    pub through: String,
    pub rows: Vec<(String, i64)>,
    pub months: Vec<Month>,
    pub total: i64,
    pub matrix: Vec<MatrixRow>,
    /// Spending by group in month order; refunds are negative spending.
    pub spending_series: Vec<MatrixRow>,
    pub spending_entries: Vec<SpendingEntry>,
}

struct Part<'a> {
    category: Option<String>,
    value: i64,
    income: bool,
    payee: String,
    memo: &'a str,
}
fn parts<'a>(budget: &Budget, t: &'a Transaction) -> Result<Vec<Part<'a>>> {
    if t.splits.is_empty() {
        return Ok(vec![Part {
            category: t.category.clone(),
            value: t.base_amount,
            income: t.category.is_none() && (t.amount > 0 || t.income_month.is_some()),
            payee: t.payee.clone(),
            memo: &t.memo,
        }]);
    }
    Ok(t.splits
        .iter()
        .zip(budget.split_base_amounts(t)?)
        .filter(|(s, _)| budget.split_has_category_flow(t, s))
        .map(|(s, value)| Part {
            category: s.category.clone(),
            value,
            income: s.category.is_none() && (s.amount > 0 || s.income_month.is_some()),
            payee: budget.split_payee_name(t, s),
            memo: if s.memo.is_empty() { &t.memo } else { &s.memo },
        })
        .collect())
}

pub fn calculate(budget: &Budget, kind: usize, filter: &Filter) -> Result<Report> {
    if kind >= TITLES.len() {
        return Err("Unknown report".into());
    }
    let mut months: Vec<_> = filter
        .months()?
        .into_iter()
        .map(|month| Month {
            month,
            income: 0,
            expense: 0,
            assets: 0,
            debts: 0,
            net_worth: 0,
        })
        .collect();
    let mut groups = BTreeMap::<String, i64>::new();
    let mut income_sources = BTreeMap::<String, Vec<i64>>::new();
    let mut category_values = BTreeMap::<Option<String>, Vec<i64>>::new();
    let mut spending_entries = vec![];
    let mut spending_values = BTreeMap::<String, Vec<i64>>::new();
    if kind != 4 {
        for t in &budget.transactions {
            if t.date < filter.from
                || t.date > filter.through
                || !budget.has_category_flow(t)
                || !filter.includes_account(budget.account(&t.account)?)
            {
                continue;
            }
            let month_index = months
                .iter()
                .position(|m| t.date.starts_with(&m.month))
                .ok_or("Report transaction outside period")?;
            let month_count = months.len();
            let month = &mut months[month_index];
            for part in parts(budget, t)? {
                if !filter.payee.matches(&part.payee) {
                    continue;
                }
                let Part {
                    category,
                    value,
                    income: is_income,
                    payee,
                    memo,
                } = part;
                if !filter.category.matches_category(&category)
                    || (is_income
                        && !matches!(filter.category, Selection::All | Selection::BudgetAccounts))
                {
                    continue;
                }
                let category = category
                    .as_ref()
                    .and_then(|id| budget.categories.iter().find(|c| c.id == *id));
                // Categorized inflows are refunds, not new income.
                if is_income {
                    month.income += value;
                    if kind == 3 {
                        let payee = if payee.is_empty() {
                            "No Payee"
                        } else {
                            payee.as_str()
                        };
                        income_sources
                            .entry(payee.into())
                            .or_insert_with(|| vec![0; month_count])[month_index] += value;
                    }
                    continue;
                }
                month.expense -= value;
                let group = category
                    .map(|c| {
                        if c.hidden
                            || budget
                                .category_groups
                                .iter()
                                .any(|g| g.name == c.group && g.hidden)
                        {
                            "Hidden Categories".to_owned()
                        } else {
                            c.group.clone()
                        }
                    })
                    .unwrap_or_else(|| "Uncategorized".into());
                spending_entries.push(SpendingEntry {
                    transaction_id: t.id.clone(),
                    date: t.date.clone(),
                    account: budget.account(&t.account)?.name.clone(),
                    payee: payee.to_owned(),
                    category: category.map(|c| c.id.clone()),
                    category_label: category
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| "Uncategorized".into()),
                    group,
                    memo: memo.to_owned(),
                    value: -value,
                });
                if kind == 3 {
                    category_values
                        .entry(category.map(|c| c.id.clone()))
                        .or_insert_with(|| vec![0; month_count])[month_index] += value;
                }
                let label = if kind == 1 {
                    if payee.is_empty() {
                        "No Payee".to_owned()
                    } else {
                        payee.to_owned()
                    }
                } else {
                    category
                        .map(|c| {
                            if c.hidden
                                || budget
                                    .category_groups
                                    .iter()
                                    .any(|g| g.name == c.group && g.hidden)
                            {
                                "Hidden Categories".into()
                            } else {
                                c.group.clone()
                            }
                        })
                        .unwrap_or_else(|| "Uncategorized".into())
                };
                if kind == 2 {
                    spending_values
                        .entry(label.clone())
                        .or_insert_with(|| vec![0; month_count])[month_index] -= value;
                }
                *groups.entry(label).or_default() -= value;
            }
        }
    } else {
        for month in &mut months {
            let end = if month.month == filter.through[..7] {
                filter.through.clone()
            } else {
                Budget::month_end(&month.month)
            };
            for account in budget
                .accounts
                .iter()
                .filter(|a| filter.includes_account(a))
            {
                let balance = budget.balance(&account.id, Some(&end), false);
                if balance != 0 {
                    let value = budget.to_base(balance, &account.currency, &month.month)?.0;
                    if value >= 0 {
                        month.assets += value;
                    } else {
                        month.debts += value;
                    }
                }
            }
            month.net_worth = month.assets + month.debts;
        }
    }
    let mut rows: Vec<_> = match kind {
        0 | 1 => groups
            .into_iter()
            .filter(|(_, value)| *value != 0)
            .collect(),
        2 => months
            .iter()
            .map(|m| (m.month.clone(), m.expense))
            .collect(),
        3 => vec![
            ("Income".into(), months.iter().map(|m| m.income).sum()),
            ("Expenses".into(), months.iter().map(|m| m.expense).sum()),
            (
                "Net Income".into(),
                months.iter().map(|m| m.income - m.expense).sum(),
            ),
        ],
        _ => months
            .iter()
            .map(|m| (m.month.clone(), m.net_worth))
            .collect(),
    };
    if kind < 2 {
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    }
    let total = match kind {
        3 => months.iter().map(|m| m.income - m.expense).sum(),
        4 => months.last().map_or(0, |m| m.net_worth),
        _ => rows.iter().map(|(_, value)| value).sum(),
    };
    let mut matrix = vec![];
    if kind == 3 {
        let mut income = matrix_row(
            "income",
            "All Income Sources",
            months.iter().map(|m| m.income).collect(),
        );
        income.children = income_sources
            .into_iter()
            .map(|(name, values)| matrix_row(&format!("payee:{name}"), &name, values))
            .collect();
        matrix.push(income);
        matrix.push(matrix_row(
            "total-income",
            "Total Income",
            months.iter().map(|m| m.income).collect(),
        ));
        // Preserve budget category order and identity, even for duplicate labels.
        let mut expense_groups: Vec<MatrixRow> = vec![];
        for category in &budget.categories {
            let Some(values) = category_values.remove(&Some(category.id.clone())) else {
                continue;
            };
            let hidden = category.hidden
                || budget
                    .category_groups
                    .iter()
                    .any(|g| g.name == category.group && g.hidden);
            let (key, label) = if hidden {
                ("hidden".to_owned(), "Hidden Categories")
            } else {
                (format!("group:{}", category.group), category.group.as_str())
            };
            let index = expense_groups
                .iter()
                .position(|g| g.key == key)
                .unwrap_or_else(|| {
                    expense_groups.push(matrix_row(&key, label, vec![0; months.len()]));
                    expense_groups.len() - 1
                });
            let group = &mut expense_groups[index];
            for (sum, value) in group.values.iter_mut().zip(&values) {
                *sum += value;
            }
            group.children.push(matrix_row(
                &format!("category:{}", category.id),
                &category.name,
                values,
            ));
        }
        if let Some(values) = category_values.remove(&None) {
            expense_groups.push(matrix_row("uncategorized", "Uncategorized", values));
        }
        matrix.extend(expense_groups);
        matrix.push(matrix_row(
            "total-expenses",
            "Total Expenses",
            months.iter().map(|m| -m.expense).collect(),
        ));
        let mut net_income = matrix_row(
            "net-income",
            "Net Income",
            months.iter().map(|m| m.income - m.expense).collect(),
        );
        net_income.average_override = Some(
            matrix
                .iter()
                .filter(|row| row.key != "total-income" && row.key != "total-expenses")
                .map(|row| {
                    if row.children.is_empty() || row.key == "hidden" {
                        row.average()
                    } else {
                        row.children.iter().map(MatrixRow::average).sum()
                    }
                })
                .sum(),
        );
        matrix.push(net_income);
    }
    Ok(Report {
        kind,
        from: filter.from.clone(),
        through: filter.through.clone(),
        rows,
        months,
        total,
        matrix,
        spending_entries,
        spending_series: spending_values
            .into_iter()
            .map(|(label, values)| matrix_row(&format!("spending:{label}"), &label, values))
            .collect(),
    })
}

impl Report {
    /// Overview or the next navigation level, leaving period and filters intact.
    /// The payee overview has at most nine named slices and one Other slice.
    pub fn spending_rows(
        &self,
        scope: Option<&SpendingScope>,
        month: Option<&str>,
    ) -> Vec<SpendingRow> {
        let mut rows = Vec::<SpendingRow>::new();
        for entry in self.spending_entries.iter().filter(|e| {
            scope.is_none_or(|s| s.includes(e)) && month.is_none_or(|m| e.date.starts_with(m))
        }) {
            let (target, label) = match scope {
                Some(SpendingScope::Group(_)) => (
                    SpendingScope::Category(entry.category.clone()),
                    entry.category_label.clone(),
                ),
                Some(SpendingScope::Other(_)) => (
                    SpendingScope::Payee(entry.payee.clone()),
                    if entry.payee.is_empty() {
                        "No Payee".into()
                    } else {
                        entry.payee.clone()
                    },
                ),
                Some(_) => continue,
                None if self.kind == 1 => (
                    SpendingScope::Payee(entry.payee.clone()),
                    if entry.payee.is_empty() {
                        "No Payee".into()
                    } else {
                        entry.payee.clone()
                    },
                ),
                None => (
                    SpendingScope::Group(entry.group.clone()),
                    entry.group.clone(),
                ),
            };
            if let Some(row) = rows.iter_mut().find(|r| r.scope == target) {
                row.value += entry.value;
            } else {
                rows.push(SpendingRow {
                    label,
                    value: entry.value,
                    scope: target,
                });
            }
        }
        rows.retain(|r| scope.is_some() || r.value != 0);
        rows.sort_by(|a, b| b.value.cmp(&a.value).then_with(|| a.label.cmp(&b.label)));
        if self.kind == 1 && scope.is_none() && rows.len() > 10 {
            let others = rows.split_off(9);
            rows.push(SpendingRow {
                label: "All Other Payees".into(),
                value: others.iter().map(|r| r.value).sum(),
                scope: SpendingScope::Other(
                    others
                        .into_iter()
                        .map(|r| match r.scope {
                            SpendingScope::Payee(payee) => payee,
                            _ => unreachable!(),
                        })
                        .collect(),
                ),
            });
        }
        rows
    }

    pub fn spending_details(
        &self,
        scope: &SpendingScope,
        month: Option<&str>,
    ) -> Vec<&SpendingEntry> {
        let mut entries: Vec<_> = self
            .spending_entries
            .iter()
            .filter(|e| scope.includes(e) && month.is_none_or(|m| e.date.starts_with(m)))
            .collect();
        entries.sort_by(|a, b| {
            a.date
                .cmp(&b.date)
                .then_with(|| a.transaction_id.cmp(&b.transaction_id))
        });
        entries
    }

    pub fn spending_csv(
        &self,
        scope: &SpendingScope,
        month: Option<&str>,
        currency: &str,
    ) -> Result<Vec<u8>> {
        let mut writer = csv::Writer::from_writer(vec![]);
        writer
            .write_record([
                "From",
                "Through",
                "Currency",
                "Transaction",
                "Date",
                "Account",
                "Payee",
                "Group",
                "Category",
                "Memo",
                "Spending",
            ])
            .map_err(|e| e.to_string())?;
        for entry in self.spending_details(scope, month) {
            writer
                .write_record([
                    &self.from,
                    &self.through,
                    currency,
                    &entry.transaction_id,
                    &entry.date,
                    &entry.account,
                    &entry.payee,
                    &entry.group,
                    &entry.category_label,
                    &entry.memo,
                    &plain(entry.value, currency),
                ])
                .map_err(|e| e.to_string())?;
        }
        writer.into_inner().map_err(|e| e.to_string())
    }

    pub fn csv(&self, currency: &str, label: impl Fn(&str) -> String) -> Result<Vec<u8>> {
        let mut writer = csv::Writer::from_writer(vec![]);
        if self.kind == 4 {
            writer
                .write_record([
                    "From",
                    "Through",
                    "Currency",
                    "Month",
                    "Assets",
                    "Debts",
                    "Net Worth",
                ])
                .map_err(|e| e.to_string())?;
            for month in &self.months {
                writer
                    .write_record([
                        &self.from,
                        &self.through,
                        currency,
                        &month.month,
                        &plain(month.assets, currency),
                        &plain(month.debts, currency),
                        &plain(month.net_worth, currency),
                    ])
                    .map_err(|e| e.to_string())?;
            }
            return writer.into_inner().map_err(|e| e.to_string());
        }
        if self.kind == 2 {
            writer
                .write_record(["From", "Through", "Currency", "Month", "Group", "Spending"])
                .map_err(|e| e.to_string())?;
            for (index, month) in self.months.iter().enumerate() {
                for series in &self.spending_series {
                    writer
                        .write_record([
                            &self.from,
                            &self.through,
                            currency,
                            &month.month,
                            &label(&series.label),
                            &plain(series.values[index], currency),
                        ])
                        .map_err(|e| e.to_string())?;
                }
                writer
                    .write_record([
                        &self.from,
                        &self.through,
                        currency,
                        &month.month,
                        "Total",
                        &plain(month.expense, currency),
                    ])
                    .map_err(|e| e.to_string())?;
            }
            return writer.into_inner().map_err(|e| e.to_string());
        }
        if self.kind == 3 {
            let mut header: Vec<String> = ["From", "Through", "Currency", "Group", "Label"]
                .into_iter()
                .map(str::to_owned)
                .collect();
            header.extend(self.months.iter().map(|m| m.month.clone()));
            header.extend(["Average".into(), "Total".into()]);
            writer.write_record(header).map_err(|e| e.to_string())?;
            for row in &self.matrix {
                for (group, detail) in std::iter::once(("", row))
                    .chain(row.children.iter().map(|child| (row.label.as_str(), child)))
                {
                    let mut record = vec![
                        self.from.clone(),
                        self.through.clone(),
                        currency.into(),
                        label(group),
                        label(&detail.label),
                    ];
                    record.extend(detail.values.iter().map(|value| plain(*value, currency)));
                    record.push(plain(detail.average(), currency));
                    record.push(plain(detail.total(), currency));
                    writer.write_record(record).map_err(|e| e.to_string())?;
                }
            }
            return writer.into_inner().map_err(|e| e.to_string());
        }
        writer
            .write_record(["Report", "From", "Through", "Label", "Amount", "Currency"])
            .map_err(|e| e.to_string())?;
        for (name, value) in &self.rows {
            writer
                .write_record([
                    TITLES[self.kind],
                    &self.from,
                    &self.through,
                    &label(name),
                    &plain(*value, currency),
                    currency,
                ])
                .map_err(|e| e.to_string())?;
        }
        writer.into_inner().map_err(|e| e.to_string())
    }
}
