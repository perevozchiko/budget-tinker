use budgettinker::model::{Budget, Result, Transaction, current_month};

pub const CLEARED: &str = "is:Cleared or Reconciled";
pub const UNCLEARED: &str = "is:Uncleared";

pub fn shows_reconciled(transaction: &Transaction, hide: bool, search: &str) -> bool {
    !hide || !search.trim().is_empty() || !transaction.reconciled
}

pub fn matches(budget: &Budget, transaction: &Transaction, search: &str) -> bool {
    let query = search.trim().to_lowercase();
    match query.as_str() {
        "is:cleared or reconciled" => transaction.cleared || transaction.reconciled,
        "is:cleared" => transaction.cleared,
        "is:reconciled" => transaction.reconciled,
        "is:uncleared" => !transaction.cleared && !transaction.reconciled,
        "flag:red" => transaction.flag == Some(budgettinker::model::TransactionFlag::Red),
        "flag:orange" => transaction.flag == Some(budgettinker::model::TransactionFlag::Orange),
        "flag:yellow" => transaction.flag == Some(budgettinker::model::TransactionFlag::Yellow),
        "flag:green" => transaction.flag == Some(budgettinker::model::TransactionFlag::Green),
        "flag:blue" => transaction.flag == Some(budgettinker::model::TransactionFlag::Blue),
        "flag:purple" => transaction.flag == Some(budgettinker::model::TransactionFlag::Purple),
        "flag:none" => transaction.flag.is_none(),
        "" => true,
        _ => format!(
            "{} {} {} {} {}",
            transaction.date,
            transaction.payee,
            transaction.memo,
            budget.transaction_category_name(transaction),
            transaction
                .splits
                .iter()
                .map(|s| format!(
                    "{} {} {} {}",
                    s.payee.as_deref().unwrap_or(""),
                    s.memo,
                    s.income_month
                        .as_ref()
                        .map(|m| budgettinker::income::income_label(m))
                        .unwrap_or_else(|| budget.category_name(&s.category)),
                    s.transfer
                        .as_ref()
                        .and_then(|x| budget.account(&x.account).ok())
                        .map(|a| a.name.as_str())
                        .unwrap_or("")
                ))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .to_lowercase()
        .contains(&query),
    }
}

/// A single account uses its native amounts. All Accounts uses the current
/// month's snapshots, matching the register's working balance valuation.
pub fn total<'a>(
    budget: &Budget,
    mut transactions: impl Iterator<Item = &'a Transaction>,
    single_account: bool,
) -> Result<i64> {
    transactions.try_fold(0, |sum, t| {
        let amount = if single_account {
            t.amount
        } else {
            budget
                .to_base(
                    t.amount,
                    &budget.account(&t.account)?.currency,
                    &current_month(),
                )?
                .0
        };
        Ok(sum + amount)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_includes_hidden_reconciled_rows_and_flags_filter_independently() {
        let mut b = Budget::new("EUR");
        b.add_account("Cash", "EUR", true, 100, "2026-01-01")
            .unwrap();
        let mut t = b.transactions[0].clone();
        t.reconciled = true;
        t.flag = Some(budgettinker::model::TransactionFlag::Purple);
        assert!(!shows_reconciled(&t, true, ""));
        assert!(shows_reconciled(&t, false, ""));
        assert!(shows_reconciled(&t, true, CLEARED));
        assert!(shows_reconciled(&t, true, "flag:purple"));
        assert!(matches(&b, &t, "flag:purple"));
        assert!(!matches(&b, &t, "flag:red"));
        assert!(!matches(&b, &t, "flag:none"));
    }
    #[test]
    fn balance_shortcuts_filter_status_without_mutating_transactions() {
        let mut b = Budget::new("EUR");
        b.add_account("Cash", "EUR", true, 100, "2026-01-01")
            .unwrap();
        let mut t = b.transactions[0].clone();
        let saved = serde_json::to_string(&b).unwrap();
        for (cleared, reconciled, yes, no) in [
            (false, false, UNCLEARED, CLEARED),
            (true, false, CLEARED, UNCLEARED),
            (true, true, CLEARED, UNCLEARED),
        ] {
            t.cleared = cleared;
            t.reconciled = reconciled;
            assert!(matches(&b, &t, yes));
            assert!(!matches(&b, &t, no));
        }
        assert_eq!(serde_json::to_string(&b).unwrap(), saved);
    }
    #[test]
    fn editing_or_clearing_search_replaces_status_filter_and_keeps_text_search() {
        let mut b = Budget::new("EUR");
        b.add_account("Cash", "EUR", true, 100, "2026-01-01")
            .unwrap();
        let mut t = b.transactions[0].clone();
        t.payee = "Supermarket".into();
        t.cleared = true;
        assert!(matches(&b, &t, " IS:CLEARED OR RECONCILED "));
        assert!(!matches(&b, &t, UNCLEARED));
        assert!(matches(&b, &t, "market"));
        assert!(matches(&b, &t, "2026-01"));
        assert!(matches(&b, &t, ""));
        assert!(!matches(&b, &t, "is:unknown"));
    }
    #[test]
    fn search_totals_use_native_currency_for_one_account_and_snapshots_for_all() {
        let mut b = Budget::new("EUR");
        let month = current_month();
        let date = format!("{month}-01");
        b.add_snapshot(&month, "USD", 90_000_000, &date, "test")
            .unwrap();
        b.add_account("Cash", "EUR", true, 100, &date).unwrap();
        let usd = b.add_account("Dollar", "USD", true, 200, &date).unwrap();
        assert_eq!(total(&b, b.transactions.iter(), false).unwrap(), 280);
        assert_eq!(
            total(&b, b.transactions.iter().filter(|t| t.account == usd), true).unwrap(),
            200
        );
        assert_eq!(
            total(&b, b.transactions.iter().filter(|_| false), true).unwrap(),
            0
        );
    }
}
