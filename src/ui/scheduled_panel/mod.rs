use super::*;

pub(super) struct TransferDraft<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub amount: i64,
    pub date: &'a str,
    pub memo: &'a str,
    pub repeat: Repeat,
}

pub(super) fn schedule_transfer(b: &mut Budget, draft: TransferDraft<'_>) -> Result<()> {
    let from = b.account(draft.from)?;
    let to = b.account(draft.to)?;
    if draft.from == draft.to
        || from.closed
        || to.closed
        || from.currency != to.currency
        || draft.amount <= 0
        || draft.amount > 100_000_000_000_000
        || !valid_date(draft.date)
    {
        return Err("Проверьте счета, дату и положительные суммы перевода".into());
    }
    let link = id();
    for (account, amount, payee) in [
        (draft.from, -draft.amount, to.name.clone()),
        (draft.to, draft.amount, from.name.clone()),
    ] {
        b.scheduled.push(Scheduled {
            id: id(),
            next_date: draft.date.into(),
            monthly: draft.repeat == Repeat::Monthly,
            repeat: Some(draft.repeat),
            anchor_day: None,
            transaction: Transaction {
                check_number: None,
                flag: None,
                id: id(),
                account: account.into(),
                date: draft.date.into(),
                payee: format!("Transfer : {payee}"),
                memo: draft.memo.into(),
                amount,
                base_amount: 0,
                snapshot: None,
                income_month: None,
                category: None,
                cleared: false,
                reconciled: false,
                transfer: Some(link.clone()),
                splits: vec![],
            },
        });
    }
    Ok(())
}

fn save_scheduled_draft(b: &mut Budget, editing: &RegisterDraft, repeat: Repeat) -> Result<()> {
    if let Some(sid) = &editing.id {
        let mut transaction = editing.build_scheduled(b)?;
        if let Some(target) = &editing.transfer_to {
            transaction.transfer = b
                .scheduled
                .iter()
                .find(|s| &s.id == sid)
                .ok_or("Schedule not found")?
                .transaction
                .transfer
                .clone();
            return b.edit_scheduled_transfer(sid, transaction, target, repeat);
        }
        return b.edit_scheduled(sid, transaction, repeat, None);
    }
    if let Some(target) = &editing.transfer_to {
        let value = editing.signed_amount(b)?;
        let (from, to) = if value < 0 {
            (editing.account.as_str(), target.as_str())
        } else {
            (target.as_str(), editing.account.as_str())
        };
        schedule_transfer(
            b,
            TransferDraft {
                from,
                to,
                amount: value.abs(),
                date: &editing.date,
                memo: &editing.memo,
                repeat,
            },
        )
    } else {
        let transaction = editing.build_scheduled(b)?;
        b.add_split_schedule(transaction, repeat)
    }
}

/// Open an existing scheduled transaction for editing.
fn open_schedule_editor(app: &mut App, s: &Scheduled) {
    if app.transaction_editor.is_some() {
        app.notify("Сначала сохраните или отмените текущую строку", true);
        return;
    }
    if app.scheduled_editor.is_some() {
        app.notify("Сначала сохраните или отмените текущую строку", true);
        return;
    }
    match RegisterDraft::from_scheduled(&app.b, s) {
        Ok(draft) => app.scheduled_editor = Some((draft, s.frequency())),
        Err(error) => app.notify(error, true),
    }
}

mod commands;
mod view;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lifecycle_ui_tests;
