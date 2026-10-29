use crate::model::*;
pub mod upgrade;
use rusqlite::{Connection, params};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Store {
    conn: Connection,
    pub path: PathBuf,
}
/// Inspect an existing file without creating tables or writing to a foreign document.
pub fn read_budget(path: &Path) -> Result<Budget> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let json: String = conn
        .query_row("SELECT json FROM document WHERE id=1", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let budget = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    validate(&budget)?;
    Ok(budget)
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS document (id INTEGER PRIMARY KEY CHECK(id=1), json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS revisions (id INTEGER PRIMARY KEY, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, json TEXT NOT NULL);").map_err(|e|e.to_string())?;
        Ok(Self {
            conn,
            path: path.into(),
        })
    }
    pub fn load(&self) -> Result<Option<Budget>> {
        use rusqlite::OptionalExtension;
        let data: Option<String> = self
            .conn
            .query_row("SELECT json FROM document WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        data.map(|s| {
            let b: Budget = serde_json::from_str(&s).map_err(|e| e.to_string())?;
            validate(&b)?;
            Ok(b)
        })
        .transpose()
    }
    pub fn save(&mut self, budget: &Budget) -> Result<()> {
        validate(budget)?;
        let json = serde_json::to_string(budget).map_err(|e| e.to_string())?;
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        use rusqlite::OptionalExtension;
        let previous: Option<String> = tx
            .query_row("SELECT json FROM document WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(previous) = previous {
            let previous: Budget = serde_json::from_str(&previous).map_err(|e| e.to_string())?;
            if previous.version != budget.version {
                return Err("Schema changes require the backed upgrade command".into());
            }
            if previous.version == 2 && previous.base != budget.base {
                return Err("The budget base currency is immutable".into());
            }
        }
        tx.execute(
            "INSERT INTO revisions(json) SELECT json FROM document WHERE id=1",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO document(id,json) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET json=excluded.json",params![json]).map_err(|e|e.to_string())?;
        tx.execute("DELETE FROM revisions WHERE id NOT IN (SELECT id FROM revisions ORDER BY id DESC LIMIT 100)",[]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn versions(&self) -> Result<Vec<(i64, String)>> {
        let mut query = self.conn.prepare("SELECT id, datetime(created_at, 'localtime') FROM revisions ORDER BY id DESC LIMIT 20").map_err(|e| e.to_string())?;
        query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn version(&self, id: i64) -> Result<Budget> {
        let json: String = self
            .conn
            .query_row("SELECT json FROM revisions WHERE id=?1", [id], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let budget = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        validate(&budget)?;
        Ok(budget)
    }
}
pub fn default_path() -> PathBuf {
    if let Some(p) = std::env::var_os("BUDGETTINKER_DATA") {
        return PathBuf::from(p);
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join("Library/Application Support/BudgetTinker/budget.budgettinker")
    }

    #[cfg(target_os = "windows")]
    {
        let data_dir = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        data_dir.join("BudgetTinker").join("budget.budgettinker")
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let data_dir = std::env::var_os("XDG_DATA_HOME")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".local/share")
            });
        data_dir.join("budgettinker").join("budget.budgettinker")
    }
}
pub fn validate(b: &Budget) -> Result<()> {
    if ![1, 2].contains(&b.version) || !CURRENCIES.contains(&b.base.as_str()) {
        return Err("Неподдерживаемая версия бюджета или валюта".into());
    }
    let unique = |ids: Vec<&str>| {
        let count = ids.len();
        ids.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            == count
    };
    if !unique(b.accounts.iter().map(|x| x.id.as_str()).collect())
        || !unique(b.categories.iter().map(|x| x.id.as_str()).collect())
        || !unique(b.transactions.iter().map(|x| x.id.as_str()).collect())
        || !unique(b.snapshots.iter().map(|x| x.id.as_str()).collect())
    {
        return Err("Повторяющиеся идентификаторы в бюджете".into());
    }
    if !unique(
        b.transactions
            .iter()
            .map(|t| t.id.as_str())
            .chain(b.scheduled.iter().map(|s| s.transaction.id.as_str()))
            .collect(),
    ) {
        return Err("Duplicate transaction identity across register and schedules".into());
    }
    let mut group_names = std::collections::HashSet::new();
    for group in &b.category_groups {
        if group.name.trim().is_empty()
            || group.name != group.name.trim()
            || !group_names.insert(&group.name)
        {
            return Err("Invalid or duplicate master category".into());
        }
    }
    let mut keys = std::collections::HashSet::new();
    for category in &b.categories {
        if let Some(account) = crate::credit::debt_account(&category.id) {
            b.account(account)?;
        }
    }
    b.validate_monthly_fx()?;
    for s in &b.snapshots {
        if !valid_month(&s.month)
            || s.rate <= 0
            || s.rate > 100_000_000_000_000
            || !valid_date(&s.rate_date)
            || s.rate_date > Budget::month_end(&s.month)
            || !CURRENCIES.contains(&s.currency.as_str())
            || s.currency == b.base
            || !keys.insert((&s.month, &s.currency))
        {
            return Err("Некорректный или повторяющийся снимок курса".into());
        }
    }
    for rule in b.payee_rules.values() {
        if rule
            .category
            .as_ref()
            .is_some_and(|id| !b.categories.iter().any(|c| &c.id == id))
        {
            return Err("Неизвестная категория в настройках получателя".into());
        }
    }
    for a in &b.accounts {
        if !CURRENCIES.contains(&a.currency.as_str()) {
            return Err("Неизвестная валюта счёта".into());
        }
    }
    for t in &b.transactions {
        b.account(&t.account)?;
        t.validate_income_month()?;
        if !valid_date(&t.date) || t.amount.unsigned_abs() > 100_000_000_000_000 {
            return Err("Некорректная дата или сумма операции".into());
        }
        if let Some(c) = &t.category
            && !b.categories.iter().any(|x| x.id == *c)
        {
            return Err("Неизвестная категория".into());
        }
        if (b.version == 2) != t.operation_fx.is_some() {
            return Err("Operation FX is required exactly for schema v2 postings".into());
        }
        let (base, snapshot) = b.transaction_value(t)?;
        if base != t.base_amount || snapshot != t.snapshot {
            return Err("Сумма операции не соответствует сохранённому снимку курса".into());
        }
        b.validate_splits(t)?;
        b.split_base_amounts(t)?;
        if t.reconciled && !t.cleared {
            return Err("Сверенная операция должна быть подтверждена".into());
        }
    }
    b.validate_transfer_rows(&b.transactions)?;
    for key in b.allocations.keys().chain(b.overspending_rules.keys()) {
        let Some((m, c)) = key.split_once('/') else {
            return Err("Некорректное назначение бюджета".into());
        };
        if !valid_month(m) || !b.categories.iter().any(|x| x.id == c) {
            return Err("Неизвестная категория назначения".into());
        }
    }
    for s in &b.scheduled {
        b.account(&s.transaction.account)?;
        s.transaction.validate_income_month()?;
        if !valid_date(&s.next_date)
            || s.transaction.amount.unsigned_abs() > 100_000_000_000_000
            || s.transaction.reconciled
            || s.transaction
                .category
                .as_ref()
                .is_some_and(|c| !b.categories.iter().any(|x| x.id == *c))
        {
            return Err("Некорректный план операции".into());
        }
        if s.transaction.operation_fx.is_some() {
            return Err("Scheduled templates must not contain posted operation FX".into());
        }
        b.validate_splits(&s.transaction)?;
    }
    let mut schedule_ids = std::collections::HashSet::new();
    b.validate_transfer_rows(b.scheduled.iter().map(|s| &s.transaction))?;
    for s in &b.scheduled {
        if !schedule_ids.insert(&s.id) || s.anchor_day.is_some_and(|d| !(1..=31).contains(&d)) {
            return Err("Invalid schedule identity or anchor day".into());
        }
        for peer in b.scheduled.iter().filter(|p| {
            p.id != s.id
                && p.transaction
                    .transfer_links()
                    .any(|link| s.transaction.transfer_links().any(|x| x == link))
        }) {
            if peer.next_date != s.next_date
                || peer.frequency() != s.frequency()
                || peer
                    .anchor_day
                    .unwrap_or_else(|| peer.next_date[8..].parse().unwrap())
                    != s.anchor_day
                        .unwrap_or_else(|| s.next_date[8..].parse().unwrap())
            {
                return Err("Scheduled transfer dates, frequencies and anchors must agree".into());
            }
        }
    }
    // IDs restored by Unmatch/Reject must be free even while their originals
    // are hidden in backups. legacy keeps the bank ID on the merged row;
    // locally created matches keep the manual ID instead.
    let mut restored_ids: std::collections::HashSet<_> = b
        .transactions
        .iter()
        .filter(|t| {
            b.register_meta
                .get(&t.id)
                .is_none_or(|m| m.matched.is_none())
        })
        .map(|t| t.id.as_str())
        .chain(b.scheduled.iter().map(|s| s.transaction.id.as_str()))
        .collect();
    for (tid, state) in &b.register_meta {
        let row = b
            .transactions
            .iter()
            .find(|t| &t.id == tid)
            .ok_or("Import state refers to a missing transaction")?;
        if let Some(pair) = &state.matched {
            if (pair.manual.id != *tid && pair.imported.id != *tid)
                || pair.manual.id == pair.imported.id
                || !restored_ids.insert(&pair.manual.id)
                || !restored_ids.insert(&pair.imported.id)
                || pair.manual.account != row.account
                || pair.imported.account != row.account
                || pair.manual.amount != row.amount
                || pair.imported.amount != row.amount
                || pair.imported.has_transfers()
                || pair
                    .manual_state
                    .as_ref()
                    .is_some_and(|m| m.matched.is_some())
            {
                return Err("Invalid matched import".into());
            }
            let mut originals = b.clone();
            originals.register_meta.clear();
            originals.scheduled.clear();
            originals.transactions.retain(|t| t.id != *tid);
            originals
                .transactions
                .extend([pair.manual.clone(), pair.imported.clone()]);
            validate(&originals)?;
        }
    }
    Ok(())
}
pub fn export_json(b: &Budget, path: &Path) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(b).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("budgettinker.tmp");
    fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())
}
pub fn import_json(path: &Path) -> Result<Budget> {
    let b = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    validate(&b)?;
    Ok(b)
}
pub fn export_csv(b: &Budget, path: &Path) -> Result<()> {
    let mut w = csv::Writer::from_path(path).map_err(|e| e.to_string())?;
    w.write_record([
        "Date",
        "Account",
        "Currency",
        "Payee",
        "Category",
        "Memo",
        "Outflow",
        "Inflow",
        "Cleared",
        "Transfer",
        "Income Month",
    ])
    .map_err(|e| e.to_string())?;
    for t in &b.transactions {
        let a = b.account(&t.account)?;
        w.write_record([
            t.date.clone(),
            a.name.clone(),
            a.currency.clone(),
            t.payee.clone(),
            if t.splits.is_empty() {
                b.transaction_category_name(t)
            } else {
                "Split".into()
            },
            t.memo.clone(),
            if t.amount < 0 {
                plain(-t.amount, &a.currency)
            } else {
                String::new()
            },
            if t.amount >= 0 {
                plain(t.amount, &a.currency)
            } else {
                String::new()
            },
            t.cleared.to_string(),
            t.transfer.clone().unwrap_or_default(),
            t.income_month.clone().unwrap_or_default(),
        ])
        .map_err(|e| e.to_string())?;
    }
    w.flush().map_err(|e| e.to_string())
}
/// Bank files may use comma grouping even though product input uses spaces.
/// Preserve supported statement conventions without accepting malformed groups.
pub(crate) fn imported_amount(value: &str, currency: &str) -> Result<i64> {
    let value = value.trim();
    if value.contains(',') {
        if value.contains(' ') {
            return Err("Mixed amount grouping separators".into());
        }
        amount(&value.replace(',', " "), currency)
    } else {
        amount(value, currency)
    }
}

pub fn import_csv(b: &Budget, path: &Path, account: &str) -> Result<(Budget, usize)> {
    import_csv_bytes(b, &fs::read(path).map_err(|e| e.to_string())?, account)
}
pub(crate) fn import_csv_bytes(
    b: &Budget,
    source: &[u8],
    account: &str,
) -> Result<(Budget, usize)> {
    let a = b.account(account)?;
    let mut next = b.clone();
    let mut count = 0;
    let mut seen = std::collections::HashMap::new();
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(source);
    let headers = reader.headers().map_err(|e| e.to_string())?.clone();
    let column = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));
    if column("Date").is_none()
        || (column("Amount").is_none()
            && (column("Outflow").is_none() || column("Inflow").is_none()))
    {
        return Err(
            "Нужны столбцы Date, Payee, Category, Memo и Amount либо Outflow, Inflow".into(),
        );
    }
    for (index, row) in reader.records().enumerate() {
        let row = row.map_err(|e| e.to_string())?;
        let get = |name: &str| column(name).and_then(|i| row.get(i)).unwrap_or("");
        let error = |e: String| format!("Строка {}: {e}", index + 2);
        if !get("Transfer").is_empty()
            || get("Payee").starts_with("Transfer :")
            || get("Payee").starts_with("Перевод ")
            || get("Category") == "Split"
        {
            return Err(error(
                "Переводы и split-операции импортируйте через JSON-резервную копию".into(),
            ));
        }
        if !get("Currency").is_empty() && get("Currency") != a.currency {
            return Err(error("Валюта строки не совпадает с валютой счёта".into()));
        }
        let date = get("Date");
        let parsed = ["%Y-%m-%d", "%m/%d/%y", "%m/%d/%Y", "%d.%m.%Y"]
            .iter()
            .find_map(|f| chrono::NaiveDate::parse_from_str(date, f).ok())
            .ok_or_else(|| error("Не удалось распознать дату".into()))?
            .to_string();
        let read_amount = |s: &str| {
            if s.is_empty() {
                Ok(0)
            } else {
                imported_amount(&s.replace(['$', '€', '£'], ""), &a.currency)
            }
        };
        let value = if column("Amount").is_some() {
            read_amount(get("Amount")).map_err(error)?
        } else {
            let incoming = read_amount(get("Inflow")).map_err(error)?;
            let outgoing = read_amount(get("Outflow")).map_err(error)?;
            if incoming < 0 || outgoing < 0 || (incoming > 0 && outgoing > 0) {
                return Err(error("Проверьте расход и приход строки".into()));
            }
            incoming - outgoing
        };
        let payee = b.imported_payee(get("Payee"));
        let memo = get("Memo").to_owned();
        let occurrence = seen
            .entry((parsed.clone(), value, payee.clone(), memo.clone()))
            .or_insert(0_usize);
        *occurrence += 1;
        let existing = b
            .import_history()
            .filter(|t| {
                t.account == account
                    && t.date == parsed
                    && t.amount == value
                    && t.payee == payee
                    && t.memo == memo
            })
            .count();
        if *occurrence <= existing {
            continue;
        }
        let category_name = get("Category");
        let category = if category_name.is_empty() {
            next.payee_rules
                .get(&payee)
                .and_then(|r| r.category.clone())
        } else if category_name.starts_with("Income") || category_name == "Доход / без категории"
        {
            None
        } else {
            let (group, name) = category_name
                .split_once(':')
                .map(|(g, n)| (g.trim(), n.trim()))
                .unwrap_or(("Импорт", category_name));
            if let Some(c) = next.categories.iter().find(|c| c.name == name) {
                Some(c.id.clone())
            } else {
                let cid = id();
                next.categories.push(Category {
                    id: cid.clone(),
                    group: group.into(),
                    name: name.into(),
                    hidden: false,
                });
                Some(cid)
            }
        };
        let t = Transaction {
            operation_fx: None,
            check_number: (!get("Check Number").is_empty()).then(|| get("Check Number").to_owned()),
            flag: None,
            id: id(),
            account: account.into(),
            date: parsed.clone(),
            payee,
            memo,
            amount: value,
            base_amount: 0,
            snapshot: None,
            income_month: crate::income::imported_month(
                category_name,
                get("Income Month"),
                &parsed,
            )
            .map_err(error)?,
            category,
            cleared: matches!(get("Cleared"), "true" | "Cleared" | "Reconciled" | "c"),
            reconciled: false,
            transfer: None,
            splits: vec![],
        };
        let tid = t.id.clone();
        next.put_transaction(t).map_err(error)?;
        next.register_meta.insert(
            tid,
            crate::register::ImportState {
                source_id: None,
                pending: true,
                matched: None,
            },
        );
        count += 1;
    }
    validate(&next)?;
    Ok((next, count))
}

pub fn export_budget_csv(b: &Budget, path: &Path) -> Result<()> {
    let mut months = std::collections::BTreeSet::new();
    months.insert(current_month());
    for key in b.allocations.keys().chain(b.overspending_rules.keys()) {
        months.insert(key[..7].to_owned());
    }
    for t in &b.transactions {
        months.insert(t.date[..7].to_owned());
    }
    let mut writer = csv::Writer::from_path(path).map_err(|e| e.to_string())?;
    writer
        .write_record([
            "Month",
            "Master Category",
            "Sub Category",
            "Budgeted",
            "Outflows",
            "Balance",
            "Currency",
        ])
        .map_err(|e| e.to_string())?;
    for month in months {
        for cat in &b.categories {
            writer
                .write_record([
                    month.clone(),
                    cat.group.clone(),
                    cat.name.clone(),
                    plain(b.allocated(&month, &cat.id), &b.base),
                    plain(b.activity(&month, &cat.id), &b.base),
                    plain(b.category_balance(&month, &cat.id), &b.base),
                    b.base.clone(),
                ])
                .map_err(|e| e.to_string())?;
        }
    }
    writer.flush().map_err(|e| e.to_string())
}

pub fn export_qif(b: &Budget, path: &Path) -> Result<()> {
    let line = |text: &str| text.replace(['\r', '\n'], " ");
    let mut output = String::new();
    for account in &b.accounts {
        output.push_str(&format!(
            "!Account\nN{}\nTBank\n^\n!Type:Bank\n",
            line(&account.name)
        ));
        for t in b.transactions.iter().filter(|t| t.account == account.id) {
            let date = chrono::NaiveDate::parse_from_str(&t.date, "%Y-%m-%d")
                .map_err(|e| e.to_string())?;
            output.push_str(&format!(
                "D{}\nT{}\nP{}\nM{}\n",
                date.format("%m/%d/%Y"),
                plain(t.amount, &account.currency),
                line(&t.payee),
                line(&t.memo)
            ));
            if t.cleared {
                output.push_str("C*\n");
            }
            if let Some(link) = &t.transfer {
                if let Some(other) = b
                    .transactions
                    .iter()
                    .find(|other| other.id != t.id && other.transfer_links().any(|x| x == link))
                {
                    output.push_str(&format!("L[{}]\n", line(&b.account(&other.account)?.name)));
                }
            } else if let Some(category) = &t.category
                && let Some(category) = b.categories.iter().find(|cat| &cat.id == category)
            {
                output.push_str(&format!(
                    "L{}:{}\n",
                    line(&category.group),
                    line(&category.name)
                ));
            } else if t.splits.is_empty() && (t.amount > 0 || t.income_month.is_some()) {
                output.push_str(&format!("L{}\n", b.transaction_category_name(t)));
            }
            for split in &t.splits {
                output.push_str(&format!(
                    "S{}\n${}\n",
                    if let Some(transfer) = &split.transfer {
                        format!("[{}]", line(&b.account(&transfer.account)?.name))
                    } else if split.category.is_none() && split.income_month.is_none() {
                        String::new()
                    } else {
                        line(&b.split_category_name(split))
                    },
                    plain(split.amount, &account.currency)
                ));
                if !split.memo.is_empty() {
                    output.push_str(&format!("E{}\n", line(&split.memo)));
                }
            }
            output.push_str("^\n");
        }
    }
    fs::write(path, output).map_err(|e| e.to_string())
}
