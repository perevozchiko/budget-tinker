//! Local bank statement import. Parsing is completed before any budget is committed.
use crate::{model::*, store};
use std::{collections::HashMap, fs, path::Path};

/// A parsed statement bound to the exact source and budget that were reviewed.
pub struct Preview {
    budget: Budget,
    original: Vec<u8>,
    source: Vec<u8>,
    path: std::path::PathBuf,
    account: String,
    count: usize,
    rows: Vec<Transaction>,
    matched: usize,
}
impl Preview {
    pub fn budget(&self) -> &Budget {
        &self.budget
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn matched_count(&self) -> usize {
        self.matched
    }
    pub fn transactions(&self) -> &[Transaction] {
        &self.rows
    }
    pub fn apply(&self, current: &Budget, path: &Path, account: &str) -> Result<(Budget, usize)> {
        if account != self.account || path != self.path {
            return Err(
                "The file or destination account changed. Run Check and Preview again.".into(),
            );
        }
        if serde_json::to_vec(current).map_err(|e| e.to_string())? != self.original {
            return Err("The budget changed. Run Check and Preview again.".into());
        }
        if fs::read(path).map_err(|e| e.to_string())? != self.source {
            return Err("The statement changed. Run Check and Preview again.".into());
        }
        Ok((self.budget.clone(), self.count))
    }
}
pub fn preview(b: &Budget, path: &Path, account: &str) -> Result<Preview> {
    let source = fs::read(path).map_err(|e| e.to_string())?;
    let (budget, count) = import_bytes(b, path, account, &source)?;
    let rows: Vec<_> = budget
        .transactions
        .iter()
        .filter(|t| {
            !b.transactions.iter().any(|old| old.id == t.id)
                || (budget
                    .register_meta
                    .get(&t.id)
                    .is_some_and(|m| m.matched.is_some())
                    && b.register_meta
                        .get(&t.id)
                        .is_none_or(|m| m.matched.is_none()))
        })
        .cloned()
        .collect();
    let matched = rows
        .iter()
        .filter(|t| {
            budget
                .register_meta
                .get(&t.id)
                .is_some_and(|m| m.matched.is_some())
        })
        .count();
    Ok(Preview {
        rows,
        matched,
        budget,
        original: serde_json::to_vec(b).map_err(|e| e.to_string())?,
        source,
        path: path.to_owned(),
        account: account.into(),
        count,
    })
}

#[derive(Default)]
struct Entry {
    date: String,
    payee: String,
    memo: String,
    value: String,
    category: String,
    cleared: bool,
    bank_id: Option<String>,
    check_number: Option<String>,
    splits: Vec<(String, String)>,
}
pub fn import(b: &Budget, path: &Path, account: &str) -> Result<(Budget, usize)> {
    import_bytes(
        b,
        path,
        account,
        &fs::read(path).map_err(|e| e.to_string())?,
    )
}
fn import_bytes(b: &Budget, path: &Path, account: &str, source: &[u8]) -> Result<(Budget, usize)> {
    let (mut next, count) = parse_bytes(b, path, account, source)?;
    let ofx = path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("ofx") || s.eq_ignore_ascii_case("qfx"));
    // Only pre-existing rows may be candidates; equal statement rows stay distinct.
    let imported: Vec<_> = next.transactions[b.transactions.len()..].to_vec();
    for row in imported {
        if let Some(candidate) = matching_candidate(b, &next, &row, ofx) {
            next.match_import(&row.id, &candidate)?;
        }
    }
    store::validate(&next)?;
    Ok((next, count))
}
fn parse_bytes(b: &Budget, path: &Path, account: &str, source: &[u8]) -> Result<(Budget, usize)> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "csv" {
        return store::import_csv_bytes(b, source, account);
    }
    let source = std::str::from_utf8(source).map_err(|e| e.to_string())?;
    let currency = &b.account(account)?.currency;
    let entries = match extension.as_str() {
        "ofx" | "qfx" => ofx(source, currency)?,
        "qif" => qif(source)?,
        _ => return Err("Поддерживаются файлы OFX, QFX, QIF и CSV".into()),
    };
    let mut next = b.clone();
    let mut occurrences = HashMap::new();
    let mut count = 0;
    for (index, mut entry) in entries.into_iter().enumerate() {
        entry.payee = b.imported_payee(&entry.payee);
        let parse = |value: &str| store::imported_amount(value, currency);
        let value = parse(&entry.value)?;
        if value == 0 {
            continue;
        }
        let bank_id = entry.bank_id.map(|id| format!("bank:{account}:{id}"));
        if bank_id
            .as_ref()
            .is_some_and(|id| next.imported_id_exists(id))
        {
            continue;
        }
        let occurrence = occurrences
            .entry((
                entry.date.clone(),
                entry.payee.clone(),
                entry.memo.clone(),
                value,
            ))
            .or_insert(0_usize);
        *occurrence += 1;
        if bank_id.is_none()
            && *occurrence
                <= b.import_history()
                    .filter(|t| {
                        t.account == account
                            && t.date == entry.date
                            && t.payee == entry.payee
                            && t.memo == entry.memo
                            && t.amount == value
                    })
                    .count()
        {
            continue;
        }
        let selected_category = if entry.category.is_empty() {
            next.payee_rules
                .get(&entry.payee)
                .and_then(|r| r.category.clone())
        } else {
            category(&mut next, &entry.category)?
        };
        let mut splits = Vec::new();
        for (name, sum) in entry.splits {
            splits.push(Split {
                category: category(&mut next, &name)?,
                amount: parse(&sum)?,
                ..Default::default()
            });
        }
        let source_id = bank_id.clone();
        let tid = bank_id.unwrap_or_else(id);
        next.put_transaction(Transaction {
            operation_fx: None,
            check_number: entry.check_number,
            flag: None,
            id: tid.clone(),
            account: account.into(),
            date: entry.date.clone(),
            payee: entry.payee,
            memo: entry.memo,
            amount: value,
            base_amount: 0,
            snapshot: None,
            income_month: crate::income::imported_month(&entry.category, "", &entry.date)?,
            category: if splits.is_empty() {
                selected_category
            } else {
                None
            },
            cleared: entry.cleared,
            reconciled: false,
            transfer: None,
            splits,
        })
        .map_err(|e| format!("Операция {}: {e}", index + 1))?;
        next.register_meta.insert(
            tid,
            crate::register::ImportState {
                source_id,
                pending: true,
                matched: None,
            },
        );
        count += 1;
    }
    store::validate(&next)?;
    Ok((next, count))
}
fn category(b: &mut Budget, name: &str) -> Result<Option<String>> {
    if name.is_empty() || name.starts_with("Income") {
        return Ok(None);
    }
    if name.starts_with('[') {
        return Err(
            "Импорт перевода QIF требует связанных счетов. Используйте форму перевода.".into(),
        );
    }
    let (group, name) = name
        .split_once(':')
        .map(|(g, n)| (g.trim(), n.trim()))
        .unwrap_or(("Импорт", name));
    if let Some(found) = b
        .categories
        .iter()
        .find(|c| c.name == name && (group == "Импорт" || c.group == group))
    {
        return Ok(Some(found.id.clone()));
    }
    let id = id();
    b.categories.push(Category {
        id: id.clone(),
        group: group.into(),
        name: name.into(),
        hidden: false,
    });
    Ok(Some(id))
}
fn date(source: &str) -> Result<String> {
    let source = source.trim().replace('\'', "/");
    // Chrono's %Y also accepts two digits as year 0026; try %y first.
    ["%Y-%m-%d", "%m/%d/%y", "%m/%d/%Y", "%d.%m.%Y", "%Y%m%d"]
        .iter()
        .find_map(|format| chrono::NaiveDate::parse_from_str(&source, format).ok())
        .map(|d| d.to_string())
        .ok_or_else(|| "Не удалось распознать дату".into())
}
fn ofx(source: &str, currency: &str) -> Result<Vec<Entry>> {
    let upper = source.to_ascii_uppercase();
    if let Some(found) = tag(source, &upper, "CURDEF")
        && found != currency
    {
        return Err("Валюта выписки не совпадает с валютой счёта".into());
    }
    if upper.contains("<CORRECTACTION>") {
        return Err(
            "Выписка содержит исправления банковских операций. Проверьте её перед импортом.".into(),
        );
    }
    let mut remainder = source;
    let mut entries = Vec::new();
    while let Some(start) = remainder.to_ascii_uppercase().find("<STMTTRN>") {
        remainder = &remainder[start + 9..];
        let upper = remainder.to_ascii_uppercase();
        let end = upper
            .find("</STMTTRN>")
            .ok_or("Незавершённая операция OFX")?;
        let record = &remainder[..end];
        let upper = &upper[..end];
        let required = |name: &str| {
            tag(record, upper, name)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("В OFX отсутствует поле {name}"))
        };
        let posted = required("DTPOSTED")?;
        entries.push(Entry {
            date: date(posted.get(..8).ok_or("Некорректная дата OFX")?)?,
            value: required("TRNAMT")?.into(),
            bank_id: tag(record, upper, "FITID")
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            check_number: tag(record, upper, "CHECKNUM")
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            payee: decode(
                tag(record, upper, "NAME")
                    .or_else(|| tag(record, upper, "PAYEEID"))
                    .unwrap_or(""),
            )?,
            memo: decode(tag(record, upper, "MEMO").unwrap_or(""))?,
            cleared: true,
            ..Default::default()
        });
        remainder = &remainder[end + 10..];
    }
    if entries.is_empty() {
        return Err("В выписке OFX не найдены операции".into());
    }
    Ok(entries)
}
fn tag<'a>(source: &'a str, upper: &str, name: &str) -> Option<&'a str> {
    let marker = format!("<{name}>");
    let start = upper.find(&marker)? + marker.len();
    Some(source[start..].split(['<', '\r', '\n']).next()?.trim())
}
fn decode(source: &str) -> Result<String> {
    let mut result = String::new();
    let mut remainder = source;
    while let Some(start) = remainder.find('&') {
        result.push_str(&remainder[..start]);
        let tail = &remainder[start + 1..];
        let Some(end) = tail.find(';') else {
            result.push_str(&remainder[start..]);
            return Ok(result);
        };
        let entity = &tail[..end];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = entity.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    entity.strip_prefix('#').and_then(|n| n.parse().ok())
                };
                code.and_then(char::from_u32)
                    .ok_or("Неизвестный символ XML в выписке")?
            }
        };
        result.push(c);
        remainder = &tail[end + 1..];
    }
    result.push_str(remainder);
    Ok(result)
}
fn qif(source: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut current = Entry::default();
    let mut accounts = 0;
    let mut in_account = false;
    for line in source.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if line.starts_with('!') {
            in_account = line.eq_ignore_ascii_case("!Account");
            if in_account {
                accounts += 1;
                if accounts > 1 {
                    return Err("Выберите QIF с одним счётом для импорта в выбранный счёт".into());
                }
            }
            if let Some(kind) = line.strip_prefix("!Type:")
                && !["Bank", "Cash", "CCard", "Oth A", "Oth L"]
                    .iter()
                    .any(|t| kind.eq_ignore_ascii_case(t))
            {
                return Err("Этот тип операций QIF пока не поддерживается".into());
            }
            continue;
        }
        if in_account {
            continue;
        }
        if line == "^" {
            if !current.date.is_empty() {
                current.date = date(&current.date)?;
                entries.push(std::mem::take(&mut current));
            }
            continue;
        }
        let (code, value) = line.split_at_checked(1).ok_or("Некорректная строка QIF")?;
        match code {
            "D" => current.date = value.into(),
            "T" | "U" => current.value = value.into(),
            "P" => current.payee = value.into(),
            "M" => current.memo = value.into(),
            "N" => current.check_number = (!value.is_empty()).then(|| value.into()),
            "L" => current.category = value.into(),
            "C" => current.cleared = matches!(value, "*" | "X" | "x"),
            "S" => current.splits.push((value.into(), String::new())),
            "$" => {
                current
                    .splits
                    .last_mut()
                    .ok_or("Сумма без категории в split QIF")?
                    .1 = value.into()
            }
            _ => {}
        }
    }
    if !current.date.is_empty() {
        return Err("Незавершённая операция QIF: отсутствует ^".into());
    }
    if entries.is_empty() {
        return Err("В QIF не найдены операции".into());
    }
    Ok(entries)
}

/// Numeric check comparison without a fixed integer size (00123 == 123).
fn numeric_check(value: Option<&str>) -> Option<&str> {
    let value = value?.trim();
    if value.is_empty() || !value.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let trimmed = value.trim_start_matches('0');
    Some(if trimmed.is_empty() { "0" } else { trimmed })
}
fn matching_candidate(
    original: &Budget,
    next: &Budget,
    row: &Transaction,
    ofx: bool,
) -> Option<String> {
    let date = chrono::NaiveDate::parse_from_str(&row.date, "%Y-%m-%d").ok()?;
    let check = numeric_check(row.check_number.as_deref());
    original
        .transactions
        .iter()
        .filter_map(|candidate| {
            if candidate.account != row.account
                || candidate.amount != row.amount
                || next
                    .register_meta
                    .get(&candidate.id)
                    .is_some_and(|m| m.matched.is_some())
            {
                return None;
            }
            if !ofx
                && original
                    .register_meta
                    .get(&candidate.id)
                    .is_some_and(|m| m.source_id.is_none() && !candidate.id.starts_with("bank:"))
            {
                return None;
            }
            // Legacy legacyimports stored check numbers as notes.
            let candidate_check = candidate.check_number.as_deref().or_else(|| {
                original
                    .notes
                    .get(&format!("transaction:{}:check", candidate.id))
                    .map(String::as_str)
            });
            let same_check = check.is_some() && check == numeric_check(candidate_check);
            let other = chrono::NaiveDate::parse_from_str(&candidate.date, "%Y-%m-%d").ok()?;
            let days = (date - other).num_days().abs();
            if days > if same_check { 60 } else { 10 } {
                return None;
            }
            Some((
                (
                    !same_check,
                    days,
                    payee_distance(&row.payee, &candidate.payee),
                ),
                candidate.id.clone(),
            ))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, id)| id)
}
/// Unrestricted Damerau–Levenshtein distance, including repeated transpositions.
fn payee_distance(left: &str, right: &str) -> usize {
    let a: Vec<_> = left.to_lowercase().chars().collect();
    let b: Vec<_> = right.to_lowercase().chars().collect();
    let limit = a.len() + b.len();
    let mut d = vec![vec![0; b.len() + 2]; a.len() + 2];
    d[0][0] = limit;
    for i in 0..=a.len() {
        d[i + 1][0] = limit;
        d[i + 1][1] = i;
    }
    for j in 0..=b.len() {
        d[0][j + 1] = limit;
        d[1][j + 1] = j;
    }
    let mut last = HashMap::new();
    for i in 1..=a.len() {
        let mut matched_column = 0;
        for j in 1..=b.len() {
            let previous_row = *last.get(&b[j - 1]).unwrap_or(&0);
            let previous_column = matched_column;
            let cost = usize::from(a[i - 1] != b[j - 1]);
            if cost == 0 {
                matched_column = j;
            }
            d[i + 1][j + 1] = (d[i][j] + cost)
                .min(d[i + 1][j] + 1)
                .min(d[i][j + 1] + 1)
                .min(
                    d[previous_row][previous_column] + i - previous_row - 1 + 1 + j
                        - previous_column
                        - 1,
                );
        }
        last.insert(a[i - 1], i);
    }
    d[a.len() + 1][b.len() + 1]
}
