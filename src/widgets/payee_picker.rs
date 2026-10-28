use crate::classic;
use budgettinker::model::Budget;
use eframe::egui::{self, Key};
use std::collections::BTreeSet;

pub fn suggestions(b: &Budget, query: &str) -> Vec<String> {
    let query = query.trim().to_lowercase();
    let names: BTreeSet<_> = b
        .transactions
        .iter()
        .filter(|t| t.transfer.is_none())
        .map(|t| t.payee.clone())
        .chain(
            b.scheduled
                .iter()
                .filter(|s| s.transaction.transfer.is_none())
                .map(|s| s.transaction.payee.clone()),
        )
        .chain(
            b.transactions
                .iter()
                .chain(b.scheduled.iter().map(|s| &s.transaction))
                .flat_map(|t| t.splits.iter())
                .filter(|s| s.transfer.is_none())
                .filter_map(|s| s.payee.clone()),
        )
        .chain(b.payee_rules.keys().cloned())
        .filter(|name| !name.trim().is_empty() && b.payee_rules.get(name).is_none_or(|r| r.listed))
        .collect();
    let mut names: Vec<_> = names
        .into_iter()
        .filter(|name| {
            name.to_lowercase().contains(&query)
                || b.payee_rules.get(name).is_some_and(|r| {
                    !query.is_empty() && r.rename_contains.to_lowercase().contains(&query)
                })
        })
        .collect();
    names.sort_by_key(|name| {
        (
            !name.to_lowercase().starts_with(&query),
            name.to_lowercase(),
            name.clone(),
        )
    });
    names
}

#[derive(Clone, Default)]
struct State {
    open: bool,
    active: usize,
    dismissed: bool,
}

/// Returns (accepted or lost focus, advance to category).
pub fn show(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: egui::Id,
    value: &mut String,
    b: &Budget,
) -> (bool, bool) {
    let state_id = id.with("suggestions");
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id).unwrap_or_default());
    let focused = ui.memory(|m| m.has_focus(id));
    let rows = suggestions(b, value);
    let submit = focused && ui.input(|i| i.key_pressed(Key::Enter));
    let handling = state.open;
    let down = handling && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown));
    let up = handling && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp));
    let enter = handling && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
    let tab = handling && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Tab));
    let blocked_submit =
        handling && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::Enter));
    let escape = handling && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
    let response = ui.put(
        rect.shrink2(egui::vec2(2., 2.)),
        egui::TextEdit::singleline(value)
            .id(id)
            .font(egui::FontId::proportional(12.))
            .hint_text("Payee")
            .margin(egui::Margin::symmetric(3, 1)),
    );
    if response.changed() || response.clicked() {
        state.dismissed = false;
    }
    if response.changed() || response.clicked() || (response.gained_focus() && !state.dismissed) {
        state.open = true;
        state.active = 0;
    }
    if down {
        state.active = (state.active + 1).min(rows.len().saturating_sub(1));
    }
    if up {
        state.active = state.active.saturating_sub(1);
    }
    state.active = state.active.min(rows.len().saturating_sub(1));
    if escape {
        state.open = false;
        state.dismissed = true;
        response.request_focus();
    }
    if blocked_submit {
        response.request_focus();
    }
    let mut accepted = response.lost_focus() || (submit && !handling);
    let mut selected = ((enter || tab) && !rows.is_empty()).then_some(state.active);
    let mut open = state.open && !rows.is_empty();
    egui::Popup::menu(&response)
        .id(state_id.with("popup"))
        .open_bool(&mut open)
        .frame(classic::register_menu_frame())
        .width(rect.width().max(240.))
        .show(|ui| {
            egui::ScrollArea::vertical()
                .max_height(240.)
                .show(ui, |ui| {
                    for (index, name) in rows.iter().enumerate() {
                        let row = ui.selectable_label(index == state.active, name);
                        if row.clicked() {
                            selected = Some(index);
                        }
                        if index == state.active && (up || down) {
                            row.scroll_to_me(None);
                        }
                    }
                });
        });
    if let Some(index) = selected {
        *value = rows[index].clone();
        accepted = true;
        state.open = false;
        state.dismissed = true;
        if !tab {
            response.request_focus();
        }
    } else {
        state.open &= open;
    }
    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
    (accepted, tab)
}

#[cfg(test)]
mod tests {
    use super::*;
    use budgettinker::model::*;

    #[test]
    fn search_includes_rule_aliases_and_schedules_but_excludes_hidden_payees_and_transfers() {
        let mut b = Budget::new("EUR");
        b.payee_rules.insert(
            "Corner Coffee".into(),
            PayeeRule {
                rename_contains: "CARD CORNER".into(),
                ..Default::default()
            },
        );
        b.payee_rules.insert(
            "Hidden".into(),
            PayeeRule {
                listed: false,
                ..Default::default()
            },
        );
        assert_eq!(suggestions(&b, "cOr"), vec!["Corner Coffee"]);
        assert_eq!(suggestions(&b, "card"), vec!["Corner Coffee"]);
        assert!(suggestions(&b, "hidden").is_empty());
        let account = b.add_account("Bank", "EUR", true, 0, &today()).unwrap();
        let mut d = crate::transaction_editor::Draft::new(&b, Some(&account), None).unwrap();
        d.payee = "Scheduled only".into();
        d.outflow = "1".into();
        b.scheduled.push(Scheduled {
            id: id(),
            transaction: d.build_scheduled(&b).unwrap(),
            next_date: today(),
            monthly: false,
            repeat: None,
            anchor_day: None,
        });
        assert_eq!(suggestions(&b, "scheduled"), vec!["Scheduled only"]);
        b.scheduled[0].transaction.transfer = Some(id());
        assert!(suggestions(&b, "scheduled").is_empty());
    }

    #[test]
    fn mouse_selection_accepts_the_suggestion() {
        let ctx = egui::Context::default();
        let mut b = Budget::new("EUR");
        b.payee_rules
            .insert("Corner Coffee".into(), PayeeRule::default());
        let id = egui::Id::new("payee_test");
        let mut value = "Cor".to_owned();
        let mut accepted = false;
        let mut frame = |events: Vec<egui::Event>, focus: bool| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800., 600.),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        if focus {
                            ui.memory_mut(|m| m.request_focus(id));
                        }
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(200., 27.), egui::Sense::hover());
                        accepted |= show(ui, rect, id, &mut value, &b).0;
                    });
                },
            )
        };
        frame(vec![], true);
        frame(vec![], false);
        let output = frame(vec![], false);
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == "Corner Coffee"
                {
                    Some(text.pos + text.galley.size() * 0.5)
                } else {
                    None
                }
            })
            .expect("suggestion shown");
        for pressed in [true, false] {
            frame(
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                false,
            );
        }
        assert_eq!(value, "Corner Coffee");
        assert!(accepted);
    }
}
