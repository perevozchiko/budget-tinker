use super::*;

fn foreign() -> (Budget, String, Draft, egui::Context) {
    let mut b = Budget::new("EUR");
    b.version = 2;
    let a = b
        .add_account("Synthetic RSD", "RSD", true, 0, &today())
        .unwrap();
    let mut d = Draft::new(&b, Some(&a), None).unwrap();
    d.outflow = "1 180.00".into();
    d.payee = "Synthetic FX Shop".into();
    d.category = b.categories[0].id.clone();
    let ctx = date_context();
    date_frame(&ctx, &mut d, &b, false, vec![]);
    (b, a, d, ctx)
}

fn click(ctx: &egui::Context, d: &mut Draft, b: &Budget, text: &str) {
    let out = date_frame(ctx, d, b, false, vec![]);
    let pos = date_text_position(&out, text);
    for pressed in [true, false] {
        date_frame(
            ctx,
            d,
            b,
            false,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn inline_rate_entry_without_monthly_quote_preserves_invalid_draft_then_posts() {
    let (mut b, _, mut d, ctx) = foreign();
    assert!(d.build(&b).is_err());
    click(&ctx, &mut d, &b, "Enter operation FX");
    click(&ctx, &mut d, &b, "Rate");
    date_frame(
        &ctx,
        &mut d,
        &b,
        false,
        vec![egui::Event::Text("0,00855".into())],
    );
    assert!(d.build(&b).is_err());
    assert_eq!(d.fx.rate, "0,00855");
    assert_eq!(d.payee, "Synthetic FX Shop");
    d.fx.rate = "0.00855000".into();
    let t = d.build(&b).unwrap();
    assert_eq!(t.base_amount, -1009);
    b.put_transaction(t).unwrap();
    assert!(b.snapshots.is_empty());
    assert_eq!(b.transactions.last().unwrap().base_amount, -1009);
}

#[test]
fn exact_base_only_derives_quote_and_split_entries_must_sum_exactly() {
    let (mut b, _, mut d, ctx) = foreign();
    click(&ctx, &mut d, &b, "Enter operation FX");
    click(&ctx, &mut d, &b, "Exact budget amount (signed)");
    d.fx.base = "-10.09".into();
    let t = d.build(&b).unwrap();
    assert_eq!(t.base_amount, -1009);
    assert!(t.operation_fx.as_ref().unwrap().exact_base);
    d.splits = vec![
        (d.category.clone(), "-708.00".into()),
        (d.category.clone(), "-472.00".into()),
    ];
    d.fx.parts = vec!["-6.05".into(), "-4.03".into()];
    assert!(d.build(&b).is_err());
    d.fx.parts[1] = "-4.04".into();
    let t = d.build(&b).unwrap();
    assert_eq!(b.split_base_amounts(&t).unwrap(), [-605, -404]);
    b.put_transaction(t.clone()).unwrap();
    let mut edit = Draft::new(&b, None, Some(&t.id)).unwrap();
    edit.memo = "Preserve exact FX".into();
    assert_eq!(edit.build(&b).unwrap().operation_fx, t.operation_fx);
    date_frame(&ctx, &mut edit, &b, false, vec![]);
    click(&ctx, &mut edit, &b, "Enter operation FX");
    assert_eq!(edit.fx.base, "-10.09");
    assert_eq!(edit.fx.parts, ["-6.05", "-4.04"]);
    assert_eq!(
        edit.build(&b).unwrap().operation_fx.unwrap().splits,
        t.operation_fx.unwrap().splits
    );
}

#[test]
fn earlier_quote_is_labeled_stale_and_requires_explicit_operation_confirmation() {
    let (mut b, _, mut d, ctx) = foreign();
    let prior = month_shift(&current_month(), -1);
    b.add_snapshot(
        &prior,
        "RSD",
        855000,
        &format!("{prior}-01"),
        "Synthetic old quote",
    )
    .unwrap();
    d.fx.context = Default::default();
    date_frame(&ctx, &mut d, &b, false, vec![]);
    assert!(!d.fx.enabled);
    assert!(d.build(&b).is_err());
    click(&ctx, &mut d, &b, "Enter operation FX");
    let out = date_frame(&ctx, &mut d, &b, false, vec![]);
    date_text_position(
        &out,
        &format!(
            "Stale suggestion: {prior}-01 · Synthetic old quote. Save confirms this operation only."
        ),
    );
    let t = d.build(&b).unwrap();
    assert_eq!(t.base_amount, -1009);
    assert!(matches!(
        t.operation_fx.unwrap().rate.unwrap().origin,
        budgettinker::model::operation_fx::RateOrigin::MonthlyDefault { .. }
    ));
    d.date = format!("{prior}-02");
    assert!(d.build(&b).is_err());
}
