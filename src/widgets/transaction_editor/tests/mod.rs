use super::*;
fn budget() -> (Budget, String) {
    let mut b = Budget::new("EUR");
    let a = b
        .add_account("Bank", "EUR", true, 100_000, &today())
        .unwrap();
    (b, a)
}

fn date_frame(
    ctx: &egui::Context,
    draft: &mut Draft,
    b: &Budget,
    scheduled: bool,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let description = format!("{events:?}");
    let mut action = Action::None;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1200., 700.),
            )),
            events,
            time: Some(ctx.input(|i| i.time) + 0.3),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let widths = [28., 0., 110., 170., 150., 140., 79., 79., 31., 28.];
                action = if scheduled {
                    draft.show_scheduled(ui, &widths, b, &mut Repeat::Monthly)
                } else {
                    draft.show(ui, &widths, b)
                };
            });
        },
    );
    assert!(
        action == Action::None,
        "date interaction saved/cancelled the draft: {description}, scheduled={scheduled}"
    );
    output
}

fn date_context() -> egui::Context {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.insert(
        egui::FontFamily::Name("arial-bold".into()),
        fonts.families[&egui::FontFamily::Proportional].clone(),
    );
    ctx.set_fonts(fonts);
    ctx
}

fn date_key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

fn date_text_position(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(t) if t.galley.text() == text => Some(t.pos + t.galley.size() / 2.),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing text {text}"))
}

fn payee_fixture() -> (Budget, String, String, String) {
    let (mut b, account) = budget();
    let other = b.add_account("Other", "EUR", true, 0, &today()).unwrap();
    let category = b.categories[0].id.clone();
    for (a, date, amount, memo) in [
        (&account, "2026-01-01", "3.50", "local"),
        (&other, "2026-01-02", "9", "latest"),
    ] {
        let mut d = Draft::new(&b, Some(a), None).unwrap();
        d.payee = "Corner Coffee".into();
        d.date = date.into();
        d.outflow = amount.into();
        d.memo = memo.into();
        d.category = category.clone();
        b.transactions.push(d.build(&b).unwrap());
    }
    (b, account, other, category)
}

mod categories;
mod dates;
mod entries;
mod fx;
mod layout;
mod payees;
mod schedules;
mod splits;
