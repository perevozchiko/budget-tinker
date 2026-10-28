use super::*;

#[test]
fn renaming_and_hiding_group_preserves_category_ids_allocations_and_notes() {
    let (mut b, eur, _) = fixture();
    let old = b.categories[0].group.clone();
    let ids: Vec<_> = b
        .categories
        .iter()
        .filter(|c| c.group == old)
        .map(|c| c.id.clone())
        .collect();
    b.notes
        .insert(format!("group:{old}"), "Monthly note".into());
    b.allocate("2026-01", &ids[0], 10_000).unwrap();
    b.put_transaction(expense(&eur, &ids[0], -1000, "2026-01-20"))
        .unwrap();
    let activity = b.activity("2026-01", &ids[0]);
    let balance = b.category_balance("2026-01", &ids[0]);
    let before = serde_json::to_string(&b).unwrap();
    assert!(b.rename_group(&old, "  ").is_err());
    assert!(
        b.rename_group(
            &old,
            &b.categories
                .iter()
                .find(|c| c.group != old)
                .unwrap()
                .group
                .clone()
        )
        .is_err()
    );
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    b.rename_group(&old, "Household").unwrap();
    assert_eq!(b.notes.get("group:Household"), Some(&"Monthly note".into()));
    assert!(!b.notes.contains_key(&format!("group:{old}")));
    assert!(
        ids.iter()
            .all(|id| b.categories.iter().find(|c| &c.id == id).unwrap().group == "Household")
    );
    assert_eq!(b.activity("2026-01", &ids[0]), activity);
    assert_eq!(b.category_balance("2026-01", &ids[0]), balance);
    b.set_group_hidden("Household", true).unwrap();
    assert!(
        ids.iter()
            .all(|id| b.categories.iter().find(|c| &c.id == id).unwrap().hidden)
    );
    b.set_group_hidden("Household", false).unwrap();
    assert!(
        ids.iter()
            .all(|id| !b.categories.iter().find(|c| &c.id == id).unwrap().hidden)
    );
    let before = serde_json::to_string(&b).unwrap();
    assert!(b.delete_group("Household").is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    validate(&b).unwrap();
}

#[test]
fn deleting_unused_group_removes_references_and_rejects_scheduled_use() {
    let (mut b, eur, _) = fixture();
    let group = b.categories[0].group.clone();
    let ids: Vec<_> = b
        .categories
        .iter()
        .filter(|c| c.group == group)
        .map(|c| c.id.clone())
        .collect();
    b.notes
        .insert(format!("group:{group}"), "group note".into());
    b.notes
        .insert(format!("category:{}", ids[0]), "category note".into());
    b.payee_rules.insert(
        "Shop".into(),
        PayeeRule {
            listed: true,
            category: Some(ids[0].clone()),
            rename_contains: String::new(),
        },
    );
    b.allocate("2026-01", &ids[0], 0).unwrap();
    let planned = Scheduled {
        id: id(),
        transaction: expense(&eur, &ids[0], -100, "2026-01-20"),
        next_date: "2026-01-20".into(),
        monthly: false,
        repeat: None,
        anchor_day: None,
    };
    b.scheduled.push(planned);
    let before = serde_json::to_string(&b).unwrap();
    assert!(b.delete_group(&group).is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    b.scheduled.clear();
    b.delete_group(&group).unwrap();
    assert!(b.categories.iter().all(|c| c.group != group));
    assert!(!b.notes.contains_key(&format!("group:{group}")));
    assert!(!b.notes.contains_key(&format!("category:{}", ids[0])));
    assert!(b.payee_rules["Shop"].category.is_none());
    assert!(b.allocations.is_empty());
    validate(&b).unwrap();
}

#[test]
fn dragging_categories_preserves_order_group_and_accounting_after_reload() {
    let (mut budget, eur, _) = fixture();
    let first = budget.categories[0].id.clone();
    let second = budget.categories[1].id.clone();
    let destination = budget.categories[4].id.clone();
    let destination_group = budget.categories[4].group.clone();
    budget.allocate("2026-01", &first, 12_345).unwrap();
    budget
        .put_transaction(expense(&eur, &first, -500, "2026-01-10"))
        .unwrap();
    budget
        .notes
        .insert(format!("category:{first}"), "Remember this".into());

    budget
        .move_category(&first, &destination_group, Some((&destination, false)))
        .unwrap();
    let moved = budget
        .categories
        .iter()
        .position(|c| c.id == first)
        .unwrap();
    assert_eq!(budget.categories[moved + 1].id, destination);
    assert_eq!(budget.categories[moved].group, destination_group);
    assert_eq!(budget.allocated("2026-01", &first), 12_345);
    assert_eq!(budget.activity("2026-01", &first), -500);
    assert_eq!(budget.notes[&format!("category:{first}")], "Remember this");

    let original_order: Vec<_> = budget.categories.iter().map(|c| c.id.clone()).collect();
    budget
        .move_category(&second, &destination_group, None)
        .unwrap();
    assert_eq!(
        budget
            .categories
            .iter()
            .rposition(|c| c.group == destination_group)
            .unwrap(),
        budget
            .categories
            .iter()
            .position(|c| c.id == second)
            .unwrap()
    );
    let after_move = serde_json::to_string(&budget).unwrap();
    let restored: Budget = serde_json::from_str(&after_move).unwrap();
    assert_eq!(
        restored
            .categories
            .iter()
            .map(|c| &c.id)
            .collect::<Vec<_>>(),
        budget.categories.iter().map(|c| &c.id).collect::<Vec<_>>()
    );
    assert_ne!(
        original_order,
        budget
            .categories
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>()
    );
    validate(&restored).unwrap();
}

#[test]
fn empty_master_categories_survive_reload_and_support_rename_hide_delete() {
    let mut b = Budget::new("EUR");
    let original_count = b.categories.len();
    b.add_category_group("New Master Category").unwrap();
    assert_eq!(b.categories.len(), original_count);
    assert_eq!(b.group_names()[0], "New Master Category");
    b.notes
        .insert("group:New Master Category".into(), "Note".into());
    b.rename_group("New Master Category", "My group").unwrap();
    assert_eq!(b.notes["group:My group"], "Note");
    b.set_group_hidden("My group", true).unwrap();
    let path = temp("sqlite3");
    {
        let mut store = Store::open(&path).unwrap();
        store.save(&b).unwrap();
        b = store.load().unwrap().unwrap();
    }
    assert!(b.category_groups[0].hidden);
    b.set_group_hidden("My group", false).unwrap();
    assert!(!b.category_groups[0].hidden);
    let existing = b.categories[0].group.clone();
    let before = serde_json::to_string(&b).unwrap();
    assert!(b.rename_group("My group", &existing).is_err());
    assert!(b.add_category_group(&existing).is_err());
    assert!(b.add_category_group("  ").is_err());
    assert_eq!(serde_json::to_string(&b).unwrap(), before);
    b.delete_group("My group").unwrap();
    assert!(b.category_groups.is_empty());
    assert_eq!(b.categories.len(), original_count);
    assert!(!b.notes.contains_key("group:My group"));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_category_can_move_into_an_empty_master_category() {
    let (mut b, account, _) = fixture();
    let category = b.categories[0].id.clone();
    b.put_transaction(expense(&account, &category, -200, "2026-01-02"))
        .unwrap();
    b.add_category_group("Empty destination").unwrap();
    let before = b.balance(&account, None, false);
    b.move_category(&category, "Empty destination", None)
        .unwrap();
    assert_eq!(
        b.categories
            .iter()
            .find(|c| c.id == category)
            .unwrap()
            .group,
        "Empty destination"
    );
    assert_eq!(b.balance(&account, None, false), before);
    validate(&b).unwrap();
    assert!(b.delete_group("Empty destination").is_err());
}

#[test]
fn older_budgets_without_master_category_metadata_still_load() {
    let b = Budget::new("EUR");
    let names = b.group_names();
    let mut document = serde_json::to_value(&b).unwrap();
    document.as_object_mut().unwrap().remove("category_groups");
    let legacy: Budget = serde_json::from_value(document).unwrap();
    assert!(legacy.category_groups.is_empty());
    assert_eq!(legacy.group_names(), names);
    validate(&legacy).unwrap();
}

#[test]
fn adding_a_category_preserves_group_order_and_round_trips_without_changing_money() {
    let (mut b, account, _) = fixture();
    b.add_category_group("Empty Group").unwrap();
    let group = b.categories[0].group.clone();
    let balance = b.balance(&account, None, false);
    let category = b.add_category(&group, "  Coffee  ").unwrap();
    let index = b.categories.iter().position(|c| c.id == category).unwrap();
    assert_eq!(b.categories[index].name, "Coffee");
    assert!(b.categories[index + 1..].iter().all(|c| c.group != group));
    assert!(b.add_category(&group, "coffee").is_err());
    assert!(b.add_category(&group, "  ").is_err());
    assert!(b.add_category("Missing Group", "New").is_err());
    assert!(b.add_category("Empty Group", "Coffee").is_ok());
    let path = temp("sqlite3");
    let mut store = Store::open(&path).unwrap();
    store.save(&b).unwrap();
    let restored = store.load().unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(&restored.categories).unwrap(),
        serde_json::to_value(&b.categories).unwrap()
    );
    assert_eq!(restored.balance(&account, None, false), balance);
    drop(store);
    let _ = std::fs::remove_file(path);
}
