use super::*;

pub(super) fn note(b: &mut Budget, v: &Value, field: &str, key: String) -> Result<()> {
    if let Some(s) = optional(v, field)?
        && !s.is_empty()
    {
        b.notes.insert(key, s.into());
    }
    Ok(())
}
pub(super) fn sorted<'a>(v: &'a Value, key: &str) -> Result<Vec<&'a Value>> {
    let mut a = Vec::new();
    let mut ids = BTreeSet::new();
    for row in rows(v, key)? {
        let eid = text(row, "entityId")?;
        if eid.is_empty() || !ids.insert(eid) {
            return Err(format!("Duplicate or empty entity ID in {key}: {eid}"));
        }
        if active(row)? {
            a.push(row);
        }
    }
    a.sort_by_key(|v| v.get("sortableIndex").and_then(Value::as_i64).unwrap_or(0));
    Ok(a)
}
pub(super) fn ordinary_category(b: &Budget, category: Option<&str>) -> Result<Option<String>> {
    match category {
        None | Some("") => Ok(None),
        Some(c) if b.categories.iter().any(|x| x.id == c) => Ok(Some(c.into())),
        Some(c) => Err(format!("Unknown or unsupported category: {c}")),
    }
}

pub(super) fn convert(data: &Value, name: &str) -> Result<(Budget, Vec<String>)> {
    if text(&data["fileMetaData"], "budgetDataVersion")? != "4.2" {
        return Err("Only legacy data version 4.2 is supported".into());
    }
    // Required collections must not turn a foreign or truncated document into an empty budget.
    for key in [
        "accounts",
        "masterCategories",
        "transactions",
        "payees",
        "monthlyBudgets",
        "scheduledTransactions",
    ] {
        if !data.get(key).is_some_and(Value::is_array) {
            return Err(format!("Missing legacy collection: {key}"));
        }
    }
    let base = text(&data["budgetMetaData"], "currencyISOSymbol")?;
    if !CURRENCIES.contains(&base) {
        return Err(format!("Unsupported budget currency: {base}"));
    }
    let mut b = Budget::new(base);
    b.name = name.into();
    b.categories.clear();
    let mut warnings = Vec::new();
    for a in sorted(data, "accounts")? {
        let kind = match text(a, "accountType")? {
            "Checking" => "checking",
            "Savings" => "savings",
            "Cash" => "cash",
            "CreditCard" => "credit",
            "LineOfCredit" => "line_of_credit",
            "InvestmentAccount" => "investment",
            "Paypal" => "paypal",
            "MerchantAccount" => "merchant",
            "Mortgage" => "mortgage",
            "OtherAsset" => "asset",
            "OtherLiability" => "liability",
            x => return Err(format!("Unsupported account type: {x}")),
        };
        let aid = text(a, "entityId")?.to_string();
        b.accounts.push(Account {
            id: aid.clone(),
            name: text(a, "accountName")?.into(),
            currency: base.into(),
            on_budget: boolean(a, "onBudget", true)?,
            closed: boolean(a, "hidden", false)?,
            account_type: kind.into(),
        });
        note(&mut b, a, "note", format!("account:{aid}"))?;
    }
    let groups = sorted(data, "masterCategories")?;
    let names: BTreeMap<_, _> = groups
        .iter()
        .map(|g| Ok((text(g, "entityId")?, text(g, "name")?)))
        .collect::<Result<_>>()?;
    let mut category_ids = BTreeSet::new();
    for g in &groups {
        let gid = text(g, "entityId")?;
        let hidden = gid == "MasterCategory/__Hidden__";
        let group_name = text(g, "name")?;
        if !hidden {
            if b.category_groups.iter().any(|x| x.name == group_name) {
                return Err(format!("Duplicate master category name: {group_name}"));
            }
            b.category_groups.push(CategoryGroup {
                name: group_name.into(),
                hidden: false,
            });
            note(&mut b, g, "note", format!("group:{group_name}"))?;
        }
        for c in sorted(g, "subCategories")? {
            let cid = text(c, "entityId")?;
            if !category_ids.insert(cid.to_string()) {
                return Err(format!("Duplicate category: {cid}"));
            }
            let raw = text(c, "name")?;
            let (group, category) = if hidden {
                if let Some((old_group, rest)) = raw.split_once(" ` ")
                    && let Some((name, old_id)) = rest.rsplit_once(" ` ")
                {
                    (names.get(old_id).copied().unwrap_or(old_group), name)
                } else {
                    ("Hidden Categories", raw)
                }
            } else {
                (group_name, raw)
            };
            if group.trim().is_empty() || category.trim().is_empty() {
                return Err("Empty category or group name".into());
            }
            b.categories.push(Category {
                id: cid.into(),
                group: group.into(),
                name: category.into(),
                hidden,
            });
            note(&mut b, c, "note", cid.into())?;
        }
    }
    // Restore hidden-only groups after all visible groups, without losing membership.
    for c in &b.categories {
        if !b.category_groups.iter().any(|g| g.name == c.group) {
            b.category_groups.push(CategoryGroup {
                name: c.group.clone(),
                hidden: false,
            });
        }
    }
    let payees = sorted(data, "payees")?;
    let payee_map: BTreeMap<_, _> = payees
        .iter()
        .map(|p| Ok((text(p, "entityId")?, *p)))
        .collect::<Result<_>>()?;
    for p in &payees {
        if optional(p, "targetAccountId")?.is_some() {
            continue;
        }
        let name = text(p, "name")?;
        if b.payee_rules.contains_key(name) {
            return Err(format!(
                "Duplicate payee name cannot be mapped safely: {name}"
            ));
        }
        let cat = optional(p, "autoFillCategoryId")?;
        let category = if matches!(
            cat,
            Some(
                "Category/__ImmediateIncome__"
                    | "Category/__DeferredIncome__"
                    | "Category/__Split__"
            )
        ) {
            None
        } else {
            ordinary_category(&b, cat)?
        };
        b.payee_rules.insert(
            name.into(),
            PayeeRule {
                listed: boolean(p, "enabled", true)?,
                category,
                rename_contains: String::new(),
            },
        );
    }
    for row in sorted(data, "transactions")? {
        let (t, state) = transaction_with_match(&mut b, row, &payee_map, &mut warnings)?;
        let tid = t.id.clone();
        if let Some(state) = state {
            b.register_meta.insert(tid.clone(), state);
        }
        preserve_transaction_notes(&mut b, row, &tid, &mut warnings)?;
        b.transactions.push(t);
    }
    link_transfers(
        &mut b.transactions,
        &sorted(data, "transactions")?,
        false,
        &mut warnings,
    )?;
    let schedule_values = expand_schedules(data, &mut warnings)?;
    let schedule_rows: Vec<_> = schedule_values.iter().collect();
    for row in &schedule_rows {
        let t = transaction(&b, row, &payee_map, true)?;
        let repeat = match text(row, "frequency")? {
            "Never" | "None" | "Once" => Repeat::Never,
            "Daily" => Repeat::Daily,
            "Weekly" => Repeat::Weekly,
            "EveryOtherWeek" => Repeat::EveryOtherWeek,
            "TwiceAMonth" => Repeat::TwiceAMonth,
            "Every4Weeks" => Repeat::Every4Weeks,
            "Monthly" => Repeat::Monthly,
            "EveryOtherMonth" => Repeat::EveryOtherMonth,
            "Every3Months" => Repeat::Every3Months,
            "Every4Months" => Repeat::Every4Months,
            "TwiceAYear" => Repeat::TwiceAYear,
            "Yearly" => Repeat::Yearly,
            "EveryOtherYear" => Repeat::EveryOtherYear,
            s => return Err(format!("Unsupported schedule frequency: {s}")),
        };
        let day = t.date[8..]
            .parse::<u32>()
            .map_err(|_| "Invalid schedule date")?;
        let anchor = if repeat == Repeat::TwiceAMonth {
            row.get("twiceAMonthStartDay")
                .and_then(Value::as_u64)
                .filter(|d| (1..=31).contains(d))
                .ok_or("Invalid twice-a-month anchor")? as u32
        } else {
            day
        };
        preserve_transaction_notes(&mut b, row, &t.id, &mut warnings)?;
        b.scheduled.push(Scheduled {
            id: t.id.clone(),
            next_date: t.date.clone(),
            transaction: t,
            monthly: repeat == Repeat::Monthly,
            repeat: Some(repeat),
            anchor_day: Some(anchor),
        });
    }
    let mut ts: Vec<_> = b.scheduled.iter().map(|s| s.transaction.clone()).collect();
    link_transfers(&mut ts, &schedule_rows, true, &mut warnings)?;
    for (s, t) in b.scheduled.iter_mut().zip(ts) {
        s.transaction = t;
    }
    for m in sorted(data, "monthlyBudgets")? {
        let date = text(m, "month")?;
        if !valid_date(date) || !date.ends_with("-01") {
            return Err("Invalid monthly budget date".into());
        }
        let month = &date[..7];
        note(&mut b, m, "note", month.into())?;
        for c in sorted(m, "monthlySubCategoryBudgets")? {
            let cid = text(c, "categoryId")?;
            ordinary_category(&b, Some(cid))?;
            let key = format!("{month}/{cid}");
            if b.allocations
                .insert(key.clone(), money(&c["budgeted"], base)?)
                .is_some()
            {
                return Err(format!("Duplicate allocation: {key}"));
            }
            let rule = match optional(c, "overspendingHandling")?.unwrap_or("AffectsBuffer") {
                "AffectsBuffer" => OverspendingRule::AvailableToBudget,
                "Confined" => OverspendingRule::CategoryBalance,
                s => return Err(format!("Unsupported overspending handling: {s}")),
            };
            b.overspending_rules.insert(key, rule);
        }
    }
    store::validate(&b)?;
    // Check budget arithmetic too, including empty months and debt carry.
    for key in b.allocations.keys() {
        b.available(&key[..7])?;
    }
    Ok((b, warnings))
}
