use super::*;

/// A legacymatch keeps the bank row's ID on the visible parent. Each child
/// encodes its original ID; identify the bank child by that ID, not array order.
pub(super) fn transaction_with_match(
    b: &mut Budget,
    row: &Value,
    payees: &BTreeMap<&str, &Value>,
    warnings: &mut Vec<String>,
) -> Result<(Transaction, Option<ImportState>)> {
    let tid = text(row, "entityId")?;
    let result = (|| {
        let children = sorted(row, "matchedTransactions")?;
        let mut parent = row.clone();
        parent
            .as_object_mut()
            .ok_or("Invalid transaction")?
            .remove("matchedTransactions");
        let t = transaction(b, &parent, payees, false)?;
        let pending = !boolean(row, "accepted", true)?;
        let source_id = optional(row, "FITID")?
            .filter(|s| !s.is_empty())
            .or(optional(row, concat!("Y", "NABID"))?.filter(|s| !s.is_empty()))
            .map(|id| format!("bank:{}:{id}", t.account));
        let imported = source_id.is_some() || optional(row, "source")? == Some("Imported");
        if children.is_empty() {
            return Ok((
                t,
                (pending || imported).then_some(ImportState {
                    source_id,
                    pending,
                    matched: None,
                }),
            ));
        }
        if children.len() != 2
            || t.has_transfers()
            || optional(row, "transferTransactionId")?.is_some()
        {
            return Err("Match requires two originals without transfers".into());
        }
        let prefix = format!("MT/{tid}/MatchedTransaction/");
        let mut bank = None;
        let mut manual = None;
        for child in children {
            if optional(child, "parentTransactionIdIfMatched")? != Some(tid) {
                return Err("Matched original refers to a different parent".into());
            }
            let child_id = text(child, "entityId")?;
            let original_id = child_id
                .strip_prefix(&prefix)
                .filter(|id| !id.is_empty())
                .ok_or("Invalid matched original ID")?;
            let mut original = child.clone();
            original
                .as_object_mut()
                .unwrap()
                .remove("parentTransactionIdIfMatched");
            original["entityId"] = Value::String(original_id.into());
            let original = transaction(b, &original, payees, false)?;
            if original.has_transfers()
                || optional(child, "transferTransactionId")?.is_some()
                || original.account != t.account
                || original.amount != t.amount
            {
                return Err("Matched originals require equal amounts in the same account, without transfers".into());
            }
            // Keep check numbers and split memos from both originals separately
            // from those of the visible parent, including after Unmatch.
            preserve_transaction_notes(b, child, child_id, warnings)?;
            if original_id == tid {
                bank = Some((original, !boolean(child, "accepted", true)?));
            } else if manual.replace(original).is_some() {
                return Err("Match has no bank original with the parent ID".into());
            }
        }
        let (imported, original_pending) = bank.ok_or("Match is missing its bank original")?;
        let manual = manual.ok_or("Match is missing its manual original")?;
        Ok((
            t,
            Some(ImportState {
                source_id,
                pending,
                matched: Some(MatchBackup {
                    manual_state: None,
                    manual,
                    imported,
                    pending: original_pending,
                }),
            }),
        ))
    })();
    result.map_err(|error: String| format!("Transaction {tid}: {error}"))
}

pub(super) fn preserve_transaction_notes(
    b: &mut Budget,
    v: &Value,
    tid: &str,
    warnings: &mut Vec<String>,
) -> Result<()> {
    if let Some(check) = optional(v, "checkNumber")?
        && !check.is_empty()
    {
        b.notes
            .insert(format!("transaction:{tid}:check"), check.into());
        warnings.push(format!(
            "{tid}: check number preserved in backup notes; no separate check-number column."
        ));
    }
    for (i, s) in sorted(v, "subTransactions")?.iter().enumerate() {
        if let Some(memo) = optional(s, "memo")?
            && !memo.is_empty()
        {
            b.notes
                .insert(format!("transaction:{tid}:split:{i}"), memo.into());
        }
    }
    Ok(())
}
