use super::*;

pub(super) fn link_transfers(
    ts: &mut [Transaction],
    source: &[&Value],
    scheduled: bool,
    warnings: &mut Vec<String>,
) -> Result<()> {
    struct End<'a> {
        row: usize,
        part: Option<usize>,
        source: &'a Value,
        id: String,
        account: String,
        target: String,
        date: String,
        amount: i64,
    }
    let mut ends = Vec::new();
    let mut identities = BTreeSet::new();
    for (i, t) in ts.iter().enumerate() {
        let source = source
            .iter()
            .find(|v| text(v, "entityId").ok() == Some(&t.id))
            .ok_or("Missing source transaction")?;
        if !identities.insert(t.id.clone()) {
            return Err("Duplicate transaction identity".into());
        }
        if let Some(target) = &t.transfer {
            ends.push(End {
                row: i,
                part: None,
                source,
                id: t.id.clone(),
                account: t.account.clone(),
                target: target.clone(),
                date: t.date.clone(),
                amount: t.amount,
            });
        }
        let parts = sorted(source, "subTransactions")?;
        for (j, part) in t.splits.iter().enumerate() {
            let sid = text(parts[j], "entityId")?;
            if !identities.insert(sid.into()) {
                return Err(format!("Duplicate split identity: {sid}"));
            }
            if let Some(transfer) = &part.transfer {
                ends.push(End {
                    row: i,
                    part: Some(j),
                    source: parts[j],
                    id: sid.into(),
                    account: t.account.clone(),
                    target: transfer.account.clone(),
                    date: t.date.clone(),
                    amount: part.amount,
                });
            }
        }
    }
    let mut paired = BTreeSet::new();
    for (i, a) in ends.iter().enumerate() {
        if paired.contains(&i) {
            continue;
        }
        let explicit = optional(a.source, "transferTransactionId")?;
        let candidates: Vec<_> = ends
            .iter()
            .enumerate()
            .filter(|(j, b)| {
                *j != i
                    && !paired.contains(j)
                    && a.row != b.row
                    && a.account == b.target
                    && a.target == b.account
                    && a.date == b.date
                    && a.amount == -b.amount
                    && explicit.is_none_or(|id| id == b.id)
            })
            .collect();
        if candidates.len() != 1 {
            return Err(format!(
                "Transfer {}: missing or ambiguous counterpart",
                a.id
            ));
        }
        let (j, b) = candidates[0];
        let other = optional(b.source, "transferTransactionId")?;
        if other.is_some_and(|id| id != a.id)
            || (!scheduled && (explicit.is_none() || other != Some(a.id.as_str())))
        {
            return Err(format!("Non-reciprocal transfer: {}", a.id));
        }
        if scheduled && text(source[a.row], "frequency")? != text(source[b.row], "frequency")? {
            return Err("Scheduled transfer frequencies differ".into());
        }
        if scheduled && explicit.is_none() {
            warnings.push(format!(
                "Scheduled transfer {} paired by unique counterpart.",
                a.id
            ));
        }
        let link = format!("legacy-transfer/{}", a.id.clone().min(b.id.clone()));
        for end in [a, b] {
            if let Some(part) = end.part {
                ts[end.row].splits[part].transfer.as_mut().unwrap().link = link.clone();
            } else {
                ts[end.row].transfer = Some(link.clone());
            }
        }
        paired.insert(i);
        paired.insert(j);
    }
    Ok(())
}

// legacystores ordinary scheduled transfers as a single template. Craft keeps
// both sides so that posting, editing and Undo remain atomic.
pub(super) fn expand_schedules(data: &Value, warnings: &mut Vec<String>) -> Result<Vec<Value>> {
    let mut values: Vec<_> = sorted(data, "scheduledTransactions")?
        .into_iter()
        .cloned()
        .collect();
    let originals = values.clone();
    let payees: BTreeMap<_, _> = sorted(data, "payees")?
        .into_iter()
        .map(|p| Ok((text(p, "entityId")?, p)))
        .collect::<Result<_>>()?;
    let currency = text(&data["budgetMetaData"], "currencyISOSymbol")?;
    let mut endpoints = Vec::new();
    for row in &originals {
        if transfer_target(row, &payees)?.is_some() {
            endpoints.push((row, None, row));
        }
        for part in sorted(row, "subTransactions")? {
            if transfer_target(part, &payees)?.is_some() {
                endpoints.push((row, Some(text(part, "entityId")?), part));
            }
        }
    }
    for (row, part_id, endpoint) in &endpoints {
        let target = transfer_target(endpoint, &payees)?.unwrap();
        if optional(endpoint, "transferTransactionId")?.is_some() {
            continue;
        }
        let account = text(row, "accountId")?;
        let amount = money(&endpoint["amount"], currency)?;
        let mut has_opposite = false;
        for (other_row, _, other) in &endpoints {
            if text(other_row, "accountId")? == target
                && transfer_target(other, &payees)?.as_deref() == Some(account)
                && other_row.get("date") == row.get("date")
                && money(&other["amount"], currency)? == -amount
            {
                has_opposite = true;
            }
        }
        if has_opposite {
            continue;
        }
        let tid = text(endpoint, "entityId")?;
        let peer_id = format!("legacy-counterpart/{tid}");
        if values
            .iter()
            .any(|v| v.get("entityId").and_then(Value::as_str) == Some(&peer_id))
        {
            return Err("Generated schedule ID collision".into());
        }
        let mut peer = (*row).clone();
        peer["entityId"] = Value::String(peer_id.clone());
        peer["accountId"] = Value::String(target);
        peer["targetAccountId"] = Value::String(account.into());
        peer["transferTransactionId"] = Value::String(tid.into());
        peer["payeeId"] = Value::Null;
        peer["categoryId"] = Value::Null;
        peer["subTransactions"] = Value::Array(vec![]);
        peer["amount"] =
            serde_json::from_str(&plain(-amount, currency)).map_err(|e| e.to_string())?;
        peer["memo"] = Value::String(optional(endpoint, "memo")?.unwrap_or("").into());
        let original = values
            .iter_mut()
            .find(|v| {
                v.get("entityId").and_then(Value::as_str)
                    == row.get("entityId").and_then(Value::as_str)
            })
            .unwrap();
        if let Some(sid) = part_id {
            let part = original["subTransactions"]
                .as_array_mut()
                .ok_or("Missing schedule parts")?
                .iter_mut()
                .find(|p| p.get("entityId").and_then(Value::as_str) == Some(*sid))
                .ok_or("Missing schedule part")?;
            part["transferTransactionId"] = Value::String(peer_id);
        } else {
            original["transferTransactionId"] = Value::String(peer_id);
        }
        values.push(peer);
        warnings.push(format!(
            "Scheduled transfer {tid}: generated the matching account side for atomic posting."
        ));
    }
    Ok(values)
}
