//! Shared searchable category menu for transaction and schedule editors.
use crate::{classic, i18n};
use budgettinker::model::{Budget, valid_date};
use eframe::egui::{self, Color32, Key, Vec2};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    Uncategorized,
    Income(bool),
    Category(String),
    Split,
    Create(NewCategory),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewCategory {
    pub group: String,
    pub name: String,
}

pub struct Selection {
    pub choice: Choice,
    pub advance: bool,
}

pub struct Options<'a> {
    pub selected: Choice,
    pub label: String,
    pub date: &'a str,
    pub width: f32,
    pub special: bool,
}

#[derive(Clone, Default)]
struct State {
    open: bool,
    query: String,
    active: usize,
    focus: bool,
    creating: bool,
    name: String,
    group: String,
    error: String,
}

#[derive(Debug)]
struct Entry {
    choice: Choice,
    label: String,
    group: String,
    balance: Option<i64>,
}

fn entries(b: &Budget, date: &str, query: &str, special: bool) -> Vec<Entry> {
    let month = valid_date(date).then(|| &date[..7]);
    let query = query.trim().to_lowercase();
    let matches = |label: &str| {
        let label = label.to_lowercase();
        query.split_whitespace().all(|word| label.contains(word))
    };
    let mut rows = vec![];
    if special {
        for (choice, label) in [
            (Choice::Uncategorized, "Uncategorized".to_owned()),
            (Choice::Split, "Split (Multiple Categories)".to_owned()),
        ] {
            if matches(&label) {
                rows.push(Entry {
                    choice,
                    label,
                    group: String::new(),
                    balance: None,
                });
            }
        }
        if let Some(month) = month {
            for next in [false, true] {
                let label = budgettinker::income::income_label(&budgettinker::model::month_shift(
                    month,
                    i32::from(next),
                ));
                if matches(&label) {
                    rows.push(Entry {
                        choice: Choice::Income(next),
                        label,
                        group: "Income".into(),
                        balance: None,
                    });
                }
            }
        }
    }
    for group in b.group_names() {
        let hidden_group = b
            .category_groups
            .iter()
            .any(|g| g.name == group && g.hidden);
        for category in b.categories.iter().filter(|c| c.group == group) {
            let mut label = format!(
                "{}: {}",
                i18n::Language::En.translate(&group),
                i18n::Language::En.translate(&category.name)
            );
            if category.hidden || hidden_group {
                label.push_str(" (hidden)");
            }
            if matches(&label) {
                rows.push(Entry {
                    choice: Choice::Category(category.id.clone()),
                    label,
                    group: group.clone(),
                    balance: month.map(|month| b.category_balance(month, &category.id)),
                });
            }
        }
    }
    rows
}

pub fn category_label(ctx: &egui::Context, b: &Budget, id: &str) -> String {
    b.categories
        .iter()
        .find(|c| c.id == id)
        .map(|c| format!("{}: {}", i18n::tr(ctx, &c.group), i18n::tr(ctx, &c.name)))
        .unwrap_or_else(|| "Uncategorized".into())
}

// The category menu uses open_bool so nested group menus can coexist with it.
// Track it for the enclosing editor's Enter/Escape shortcuts as well.
pub fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<u64>(egui::Id::new("category_menu_frame")))
        == Some(ctx.cumulative_frame_nr())
}

pub fn show(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    b: &Budget,
    options: Options<'_>,
) -> Option<Selection> {
    show_inner(ui, id, b, options, true)
}

pub fn show_part(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    b: &Budget,
    options: Options<'_>,
) -> Option<Selection> {
    show_inner(ui, id, b, options, false)
}
fn show_inner(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    b: &Budget,
    options: Options<'_>,
    allow_split: bool,
) -> Option<Selection> {
    let choices = |query: &str| {
        entries(b, options.date, query, options.special)
            .into_iter()
            .filter(|entry| allow_split || entry.choice != Choice::Split)
            .collect::<Vec<_>>()
    };
    let state_id = ui.make_persistent_id(id);
    let popup_id = state_id.with("popup");
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id).unwrap_or_default());
    let trigger = ui.add_sized(
        [options.width, 20.],
        egui::Button::new(format!("{}  ▾", options.label)).truncate(),
    );
    if ui
        .ctx()
        .data_mut(|d| d.remove_temp::<bool>(state_id.with("request_focus")))
        .unwrap_or(false)
    {
        trigger.request_focus();
    }
    if trigger.clicked() {
        state.open = !state.open;
        if state.open {
            state.query.clear();
            state.active = choices("")
                .iter()
                .position(|row| row.choice == options.selected)
                .unwrap_or(0);
            state.focus = true;
            state.creating = false;
            state.error.clear();
            state.group = match &options.selected {
                Choice::Category(id) => b
                    .categories
                    .iter()
                    .find(|c| c.id == *id)
                    .map(|c| c.group.clone()),
                _ => None,
            }
            .or_else(|| {
                b.group_names().into_iter().find(|name| {
                    !b.category_groups
                        .iter()
                        .any(|g| &g.name == name && g.hidden)
                })
            })
            .unwrap_or_default();
        }
    }
    let mut open = state.open;
    let mut selection = None;
    egui::Popup::menu(&trigger)
        .id(popup_id)
        .open_bool(&mut open)
        .close_behavior(if egui::Popup::is_any_open(ui.ctx()) {
            egui::PopupCloseBehavior::IgnoreClicks
        } else {
            egui::PopupCloseBehavior::CloseOnClickOutside
        })
        .width(390.)
        .frame(classic::register_menu_frame())
        .show(|ui| {
            let frame_nr = ui.ctx().cumulative_frame_nr();
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("category_menu_frame"), frame_nr));
            ui.set_width(390.);
            ui.spacing_mut().item_spacing = Vec2::new(5., 4.);
            // A nested master-category menu handles its own keys.
            let nested = egui::Popup::is_any_open(ui.ctx());
            // Consume these before TextEdit and the enclosing row see them.
            let down = !nested
                && !state.creating
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown));
            let up = !nested
                && !state.creating
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp));
            let name_id = state_id.with("new_name");
            let enter = !nested
                && (!state.creating || ui.memory(|m| m.has_focus(name_id)))
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
            let tab = !nested
                && !state.creating
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Tab));
            let escape =
                !nested && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
            if escape {
                state.open = false;
                ui.close();
                return;
            }
            if state.creating {
                ui.label("Add Category");
                let name = ui.add(
                    egui::TextEdit::singleline(&mut state.name)
                        .id(name_id)
                        .hint_text("Category name")
                        .desired_width(380.),
                );
                if state.focus {
                    name.request_focus();
                    state.focus = false;
                }
                egui::ComboBox::from_id_salt(state_id.with("group"))
                    .selected_text(if state.group.is_empty() {
                        "Select a master category".into()
                    } else {
                        i18n::tr(ui.ctx(), &state.group)
                    })
                    .width(380.)
                    .show_ui(ui, |ui| {
                        for group in b.group_names() {
                            ui.selectable_value(
                                &mut state.group,
                                group.clone(),
                                i18n::tr(ui.ctx(), group),
                            );
                        }
                    });
                if !state.error.is_empty() {
                    ui.colored_label(
                        Color32::from_rgb(180, 45, 35),
                        crate::i18n::tr(ui.ctx(), &state.error),
                    );
                }
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.creating = false;
                        state.focus = true;
                    }
                    if ui.button("Add Category").clicked() || enter {
                        let name = state.name.trim();
                        if name.is_empty() || state.group.is_empty() {
                            state.error = "Enter a name and choose a master category".into();
                        } else if b
                            .categories
                            .iter()
                            .any(|c| c.group == state.group && c.name.eq_ignore_ascii_case(name))
                        {
                            state.error =
                                "A category with this name already exists in this group".into();
                        } else {
                            selection = Some(Selection {
                                choice: Choice::Create(NewCategory {
                                    group: state.group.clone(),
                                    name: name.into(),
                                }),
                                advance: false,
                            });
                            state.open = false;
                            ui.close();
                        }
                    }
                });
                return;
            }
            let search = ui.add(
                egui::TextEdit::singleline(&mut state.query)
                    .id(state_id.with("search"))
                    .hint_text("Search categories or groups")
                    .desired_width(380.),
            );
            let initial_focus = state.focus;
            if state.focus {
                search.request_focus();
                state.focus = false;
            }
            if search.changed() {
                state.active = 0;
            }
            let rows = choices(&state.query);
            if down {
                state.active = (state.active + 1).min(rows.len().saturating_sub(1));
            }
            if up {
                state.active = state.active.saturating_sub(1);
            }
            state.active = state.active.min(rows.len().saturating_sub(1));
            ui.small(if valid_date(options.date) {
                format!("Available in {} · {}", &options.date[..7], b.base)
            } else {
                "Enter a valid date to see available amounts".into()
            });
            let mut selected = None;
            egui::ScrollArea::vertical()
                .id_salt(state_id.with("list"))
                .max_height(270.)
                .show(ui, |ui| {
                    ui.set_min_width(380.);
                    let mut previous = None;
                    for (index, row) in rows.iter().enumerate() {
                        if previous != Some(&row.group) && !row.group.is_empty() {
                            ui.label(egui::RichText::new(i18n::tr(ui.ctx(), &row.group)).strong());
                        }
                        previous = Some(&row.group);
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(380., 24.), egui::Sense::click());
                        if state.active == index || response.hovered() {
                            ui.painter()
                                .rect_filled(rect, 2., Color32::from_rgb(207, 231, 246));
                        }
                        let balance_width = if row.balance.is_some() { 100. } else { 0. };
                        ui.painter()
                            .with_clip_rect(egui::Rect::from_min_max(
                                rect.min,
                                rect.max - Vec2::new(balance_width, 0.),
                            ))
                            .text(
                                rect.left_center() + Vec2::new(5., 0.),
                                egui::Align2::LEFT_CENTER,
                                &row.label,
                                egui::FontId::proportional(12.),
                                classic::TEXT,
                            );
                        if let Some(balance) = row.balance {
                            ui.painter().text(
                                rect.right_center() - Vec2::new(5., 0.),
                                egui::Align2::RIGHT_CENTER,
                                i18n::number(balance, &b.base),
                                egui::FontId::proportional(12.),
                                if balance < 0 {
                                    Color32::from_rgb(180, 45, 35)
                                } else {
                                    classic::TEXT
                                },
                            );
                        }
                        let response = response.on_hover_text(&row.label);
                        if response.clicked() {
                            selected = Some(index);
                        }
                        if state.active == index
                            && (down || up || search.changed() || initial_focus)
                        {
                            response.scroll_to_me(None);
                        }
                    }
                    if rows.is_empty() {
                        ui.label("No matching categories");
                    }
                });
            if (enter || tab) && !rows.is_empty() {
                selected = Some(state.active);
            }
            if let Some(index) = selected {
                selection = Some(Selection {
                    choice: rows[index].choice.clone(),
                    advance: tab,
                });
                state.open = false;
                ui.close();
            }
            ui.separator();
            if ui.button("+ Add Category…").clicked() {
                state.creating = true;
                state.focus = true;
                state.name = state.query.trim().into();
                state.error.clear();
            }
        });
    // Popup can also close on an outside click. Do not retain a stale query next time.
    state.open &= open;
    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
    selection
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Budget, String, String) {
        let mut b = Budget::new("USD");
        b.add_category_group("Travel").unwrap();
        b.add_category_group("Everyday Life").unwrap();
        let coffee = b.add_category("Everyday Life", "Coffee").unwrap();
        let other = b.add_category("Travel", "Coffee").unwrap();
        b.allocate("2026-07", &coffee, 5_000).unwrap();
        b.allocate("2026-08", &coffee, 2_000).unwrap();
        (b, coffee, other)
    }

    #[test]
    fn search_disambiguates_groups_and_uses_the_transaction_month() {
        let (b, coffee, other) = fixture();
        let july = entries(&b, "2026-07-31", "cOFfEe", true);
        assert_eq!(july.len(), 2);
        assert_eq!(july[0].choice, Choice::Category(coffee.clone()));
        assert_eq!(july[1].choice, Choice::Category(other));
        assert_eq!(july[0].label, "Everyday Life: Coffee");
        assert_eq!(july[1].label, "Travel: Coffee");
        assert_eq!(july[0].balance, Some(5_000));
        assert_eq!(
            entries(&b, "2026-08-01", "everyday coffee", true)[0].balance,
            Some(7_000)
        );
        assert_eq!(entries(&b, "2026-07-", "coffee", true)[0].balance, None);
        assert!(entries(&b, "2026-07-01", "nonexistent", true).is_empty());
    }

    #[test]
    fn hidden_categories_and_income_choices_remain_available() {
        let (mut b, coffee, _) = fixture();
        b.categories
            .iter_mut()
            .find(|c| c.id == coffee)
            .unwrap()
            .hidden = true;
        let rows = entries(&b, "2026-12-15", "", true);
        assert!(
            rows.iter()
                .any(|r| r.label == "Everyday Life: Coffee (hidden)")
        );
        assert!(rows.iter().any(|r| r.choice == Choice::Split));
        assert!(
            rows.iter()
                .any(|r| r.choice == Choice::Income(false) && r.label.contains("December"))
        );
        assert!(
            rows.iter()
                .any(|r| r.choice == Choice::Income(true) && r.label.contains("January"))
        );
        assert!(
            entries(&b, "invalid", "", true)
                .iter()
                .all(|r| !matches!(r.choice, Choice::Income(_)))
        );
        assert!(
            entries(&b, "2026-12-15", "", false)
                .iter()
                .all(|r| matches!(r.choice, Choice::Category(_)))
        );
    }

    struct Harness {
        ctx: egui::Context,
        budget: Budget,
        result: Option<Selection>,
    }
    impl Harness {
        fn new() -> Self {
            Self {
                ctx: egui::Context::default(),
                budget: fixture().0,
                result: None,
            }
        }
        fn render(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
            self.ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(900., 600.),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        if let Some(result) = show(
                            ui,
                            "test",
                            &self.budget,
                            Options {
                                selected: Choice::Uncategorized,
                                label: "Uncategorized".into(),
                                date: "2026-07-15",
                                width: 200.,
                                special: true,
                            },
                        ) {
                            self.result = Some(result);
                        }
                    });
                },
            )
        }
        fn click(&mut self, label: &str) {
            self.render(vec![]);
            let output = self.render(vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.text() == label
                    {
                        return Some(text.pos + text.galley.size() * 0.5);
                    }
                    None
                })
                .unwrap_or_else(|| panic!("Missing {label}"));
            for pressed in [true, false] {
                self.render(vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
            }
            self.render(vec![]);
        }
        fn key(&mut self, key: Key) {
            self.render(vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            self.render(vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
    }

    #[test]
    fn typing_arrows_enter_and_tab_select_without_leaking_keys() {
        for key in [Key::Enter, Key::Tab] {
            let mut h = Harness::new();
            h.click("Uncategorized  ▾");
            h.render(vec![egui::Event::Text("Coffee".into())]);
            h.key(Key::ArrowDown);
            h.key(Key::ArrowUp);
            h.key(Key::ArrowDown);
            h.key(key);
            let selection = h.result.take().expect("keyboard selection");
            assert_eq!(
                selection.choice,
                Choice::Category(
                    h.budget
                        .categories
                        .iter()
                        .find(|c| c.group == "Travel" && c.name == "Coffee")
                        .unwrap()
                        .id
                        .clone()
                )
            );
            assert_eq!(selection.advance, key == Key::Tab);
            assert!(!is_open(&h.ctx));
            assert!(!h.ctx.input(|i| i.key_pressed(key)));
        }
    }

    #[test]
    fn escape_dismisses_search_and_creation_cancel_does_not_change_budget() {
        let mut h = Harness::new();
        let before = serde_json::to_value(&h.budget).unwrap();
        h.click("Uncategorized  ▾");
        h.render(vec![egui::Event::Text("Coffee".into())]);
        h.key(Key::Escape);
        assert!(h.result.is_none());
        h.click("Uncategorized  ▾");
        h.click("+ Add Category…");
        h.render(vec![egui::Event::Text("New category".into())]);
        h.click("Cancel");
        assert!(h.result.is_none());
        assert_eq!(serde_json::to_value(&h.budget).unwrap(), before);
        h.key(Key::Escape);
    }

    #[test]
    fn create_returns_a_request_without_mutating_the_budget() {
        let mut h = Harness::new();
        let count = h.budget.categories.len();
        h.click("Uncategorized  ▾");
        h.render(vec![egui::Event::Text("New category".into())]);
        h.click("+ Add Category…");
        h.key(Key::Enter);
        let result = h.result.take().unwrap();
        assert!(
            matches!(result.choice, Choice::Create(NewCategory { ref name, .. }) if name == "New category")
        );
        assert_eq!(h.budget.categories.len(), count);
    }
}
