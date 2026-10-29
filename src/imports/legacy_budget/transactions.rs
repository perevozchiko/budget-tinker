use super::*;

pub(super) fn transfer_target(
    v: &Value,
    payees: &BTreeMap<&str, &Value>,
) -> Result<Option<String>> {
    let direct = optional(v, "targetAccountId")?;
    let from_payee = optional(v, "payeeId")?
        .and_then(|id| payees.get(id))
        .map(|p| optional(p, "targetAccountId"))
        .transpose()?
        .flatten();
    if let (Some(a), Some(b)) = (direct, from_payee)
        && a != b
    {
        return Err("Transfer target conflicts with its payee".into());
    }
    Ok(direct.or(from_payee).map(str::to_owned))
}

pub(super) fn transaction(
    b: &Budget,
    v: &Value,
    payees: &BTreeMap<&str, &Value>,
    scheduled: bool,
) -> Result<Transaction> {
    let tid = text(v, "entityId")?;
    let result = (|| {
        let date = text(v, "date")?;
        if !valid_date(date) {
            return Err("Invalid transaction date".into());
        }
        let account = text(v, "accountId")?;
        b.account(account)?;
        if !rows(v, "matchedTransactions")?.is_empty()
            || optional(v, "parentTransactionIdIfMatched")?.is_some()
        {
            return Err("Nested, scheduled or orphan matched records are not supported".into());
        }
        let payee = optional(v, "payeeId")?
            .map(|id| {
                payees
                    .get(id)
                    .copied()
                    .ok_or_else(|| format!("Missing payee: {id}"))
            })
            .transpose()?;
        let target = transfer_target(v, payees)?;
        let raw_category = optional(v, "categoryId")?;
        let mut t = Transaction {
            id: tid.into(),
            account: account.into(),
            date: date.into(),
            payee: payee
                .map(|p| text(p, "name"))
                .transpose()?
                .unwrap_or("")
                .into(),
            memo: optional(v, "memo")?.unwrap_or("").into(),
            amount: money(&v["amount"], &b.base)?,
            base_amount: money(&v["amount"], &b.base)?,
            snapshot: None,
            category: None,
            cleared: false,
            reconciled: false,
            operation_fx: None,
            check_number: optional(v, "checkNumber")?
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            flag: None,
            income_month: None,
            transfer: target.clone(),
            splits: vec![],
        };
        let cleared = optional(v, "cleared")?.unwrap_or("Uncleared");
        match cleared {
            "Uncleared" => {}
            "Cleared" => t.cleared = true,
            "Reconciled" if !scheduled => {
                t.cleared = true;
                t.reconciled = true;
            }
            _ => return Err(format!("Unsupported cleared state: {cleared}")),
        }
        t.flag = match optional(v, "flag")? {
            None | Some("") => None,
            Some("Red") => Some(TransactionFlag::Red),
            Some("Orange") => Some(TransactionFlag::Orange),
            Some("Yellow") => Some(TransactionFlag::Yellow),
            Some("Green") => Some(TransactionFlag::Green),
            Some("Blue") => Some(TransactionFlag::Blue),
            Some("Purple") => Some(TransactionFlag::Purple),
            Some(s) => return Err(format!("Unsupported flag: {s}")),
        };
        match raw_category {
            Some("Category/__ImmediateIncome__" | "Category/__DeferredIncome__") => {
                t.income_month = Some(month_shift(
                    &date[..7],
                    i32::from(raw_category == Some("Category/__DeferredIncome__")),
                ));
            }
            Some("Category/__Split__") if target.is_none() => {
                for s in sorted(v, "subTransactions")? {
                    if !rows(s, "subTransactions")?.is_empty()
                        || !rows(s, "matchedTransactions")?.is_empty()
                    {
                        return Err("Nested split or matched parts are not supported".into());
                    }
                    let raw = optional(s, "categoryId")?;
                    let income_month = match raw {
                        Some("Category/__ImmediateIncome__" | "Category/__DeferredIncome__") => {
                            Some(month_shift(
                                &date[..7],
                                i32::from(raw == Some("Category/__DeferredIncome__")),
                            ))
                        }
                        _ => None,
                    };
                    let category = if income_month.is_some() {
                        None
                    } else {
                        ordinary_category(b, raw)?
                    };
                    let target = transfer_target(s, payees)?;
                    let payee = optional(s, "payeeId")?
                        .map(|id| {
                            payees
                                .get(id)
                                .ok_or_else(|| format!("Missing split payee: {id}"))
                        })
                        .transpose()?
                        .map(|p| text(p, "name").map(str::to_owned))
                        .transpose()?;
                    let transfer = target.map(|account| SplitTransfer {
                        link: String::new(),
                        account,
                    });
                    t.splits.push(Split {
                        id: String::new(),
                        category,
                        income_month,
                        payee,
                        transfer,
                        memo: optional(s, "memo")?.unwrap_or("").into(),
                        amount: money(&s["amount"], &b.base)?,
                    });
                }
                if t.splits.is_empty() {
                    return Err("Empty split transaction".into());
                }
            }
            _ => t.category = ordinary_category(b, raw_category)?,
        }
        if raw_category != Some("Category/__Split__") && !rows(v, "subTransactions")?.is_empty() {
            return Err("Unexpected split parts".into());
        }
        if let Some(target) = target.as_deref() {
            t.payee = format!("Transfer : {}", b.account(target)?.name);
            if !t.splits.is_empty() {
                return Err("Split transfers are not supported".into());
            }
        }
        Ok(t)
    })();
    result.map_err(|e: String| format!("Transaction {tid}: {e}"))
}
