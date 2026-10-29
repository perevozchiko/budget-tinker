use super::*;

/// Changes remain a draft until Done; Cancel, Escape and outside clicks discard it.
pub(super) fn multi_select_combo(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    selection: &mut Selection,
    show_budget_preset: bool,
    content: impl FnOnce(&mut egui::Ui, &mut Selection),
) {
    let draft_id = ui.make_persistent_id((id, "draft"));
    let trigger_id = ui.make_persistent_id(id);
    let popup_id = trigger_id.with("report_popup");
    let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    let mut draft = if open {
        ui.data_mut(|data| data.get_temp::<Selection>(draft_id))
            .unwrap_or_else(|| selection.clone())
    } else {
        selection.clone()
    };
    let trigger = popover::trigger(ui, id, label, open);
    let title = if show_budget_preset {
        "Accounts"
    } else if id.contains("payee") {
        "Payees"
    } else {
        "Categories"
    };
    let popup = egui::Popup::menu(&trigger)
        .id(popup_id)
        .align(egui::RectAlign::BOTTOM)
        .width(298.)
        .gap(16.)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(popover::frame())
        .show(|ui| {
            popover::header(ui, 298., title);
            popover::band(ui, 298., Color32::WHITE, 3, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.;
                    if popover::link(ui, "Select All", draft.is_all()).clicked() {
                        draft = Selection::All;
                    }
                    if popover::link(ui, "Select None", draft.is_empty()).clicked() {
                        draft = Selection::NoneSelected;
                    }
                });
            });
            popover::divider(ui, 298.);
            popover::band(ui, 298., Color32::from_gray(245), 7, |ui| {
                ui.set_min_height(208.);
                ui.spacing_mut().item_spacing.y = 1.;
                ui.spacing_mut().interact_size.y = 19.;
                content(ui, &mut draft);
            });
            if popover::footer(ui, 298., !draft.is_empty()) {
                *selection = draft.clone();
            }
        });
    if let Some(popup) = popup {
        popover::pointer(ui, &trigger, popup.response.rect);
    }
    let still_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    ui.data_mut(|data| {
        if still_open {
            data.insert_temp(draft_id, draft);
        } else {
            data.remove::<Selection>(draft_id);
        }
    });
}

pub(super) fn category_selection_items(
    ui: &mut egui::Ui,
    selection: &mut Selection,
    budget: &budgettinker::model::Budget,
) {
    let mut items: Vec<_> = budget
        .categories
        .iter()
        .map(|category| {
            (
                category.id.as_str(),
                i18n::tr(ui.ctx(), &category.name),
                true,
            )
        })
        .collect();
    items.push((
        budgettinker::model::UNCATEGORIZED_CATEGORY,
        "Uncategorized Transactions".into(),
        true,
    ));
    let hidden = |category: &budgettinker::model::Category| {
        category.hidden
            || budget
                .category_groups
                .iter()
                .any(|g| g.name == category.group && g.hidden)
    };
    egui::ScrollArea::vertical()
        .max_height(208.)
        .show(ui, |ui| {
            for group in budget.group_names() {
                let members: Vec<_> = budget
                    .categories
                    .iter()
                    .filter(|c| c.group == group && !hidden(c))
                    .map(|c| c.id.as_str())
                    .collect();
                if !members.is_empty() {
                    selection_group(
                        ui,
                        selection,
                        &i18n::tr(ui.ctx(), &group),
                        &members,
                        &items,
                        true,
                    );
                }
            }
            let mut checked = selection_item_checked(
                selection,
                budgettinker::model::UNCATEGORIZED_CATEGORY,
                true,
            );
            if popover::checkbox(
                ui,
                &mut checked,
                RichText::new("Uncategorized Transactions").font(crate::palette::bold(12.)),
                false,
            )
            .changed()
            {
                toggle_selection_item(
                    selection,
                    budgettinker::model::UNCATEGORIZED_CATEGORY,
                    checked,
                    &items,
                );
            }
            let members: Vec<_> = budget
                .categories
                .iter()
                .filter(|c| hidden(c))
                .map(|c| c.id.as_str())
                .collect();
            if !members.is_empty() {
                selection_group(ui, selection, "Hidden Categories", &members, &items, false);
            } else {
                let mut checked = selection.is_all();
                ui.add_enabled(
                    false,
                    egui::Checkbox::new(
                        &mut checked,
                        RichText::new("Hidden Categories").font(crate::palette::bold(12.)),
                    ),
                );
            }
        });
}

fn selection_group(
    ui: &mut egui::Ui,
    selection: &mut Selection,
    label: &str,
    members: &[&str],
    items: &[(&str, String, bool)],
    children: bool,
) {
    let checked_count = members
        .iter()
        .filter(|id| {
            let item = items.iter().find(|(item, _, _)| item == *id).unwrap();
            selection_item_checked(selection, id, item.2)
        })
        .count();
    let mut checked = checked_count == members.len();
    let partial = checked_count > 0 && !checked;
    if popover::checkbox(
        ui,
        &mut checked,
        RichText::new(label).font(crate::palette::bold(12.)),
        partial,
    )
    .changed()
    {
        for member in members {
            toggle_selection_item(selection, member, checked, items);
        }
    }
    if children {
        ui.indent(label, |ui| {
            for member in members {
                let item = items.iter().find(|(id, _, _)| id == member).unwrap();
                let mut checked = selection_item_checked(selection, member, item.2);
                if popover::checkbox(ui, &mut checked, RichText::new(&item.1).size(12.), false)
                    .changed()
                {
                    toggle_selection_item(selection, member, checked, items);
                }
            }
        });
    }
}

pub(super) fn account_selection_items(
    ui: &mut egui::Ui,
    selection: &mut Selection,
    items: &[(&str, String, bool)],
    budget: &Budget,
) {
    egui::ScrollArea::vertical()
        .max_height(208.)
        .show(ui, |ui| {
            for (label, on_budget, closed) in [
                ("Budget Accounts", true, false),
                ("Off-Budget Accounts", false, false),
                ("Closed Accounts", false, true),
            ] {
                let members: Vec<_> = budget
                    .accounts
                    .iter()
                    .filter(|a| a.closed == closed && (closed || a.on_budget == on_budget))
                    .map(|a| a.id.as_str())
                    .collect();
                if !members.is_empty() {
                    selection_group(ui, selection, label, &members, items, true);
                }
            }
        });
    if let Selection::Selected(ids) = selection {
        let budget_ids: Vec<_> = budget
            .accounts
            .iter()
            .filter(|a| a.on_budget)
            .map(|a| &a.id)
            .collect();
        if !budget_ids.is_empty()
            && ids.len() == budget_ids.len()
            && budget_ids.iter().all(|id| ids.contains(id))
        {
            *selection = Selection::BudgetAccounts;
        }
    }
}

/// Whether a particular item is checked. Unlike Selection::ids(), this
/// preserves the distinction between BudgetAccounts and All Accounts.
pub(super) fn selection_item_checked(selection: &Selection, id: &str, on_budget: bool) -> bool {
    match selection {
        Selection::All => true,
        Selection::BudgetAccounts => on_budget,
        Selection::NoneSelected => false,
        Selection::Selected(ids) => ids.iter().any(|selected| selected.as_str() == id),
    }
}

/// Update selection using the visible universe; works from all four states.
/// Explicit selections are kept distinct from the Budget and All presets.
pub(super) fn toggle_selection_item(
    selection: &mut Selection,
    target: &str,
    checked: bool,
    items: &[(&str, String, bool)],
) {
    let mut chosen: Vec<String> = items
        .iter()
        .filter(|(id, _, on_budget)| selection_item_checked(selection, id, *on_budget))
        .map(|(id, _, _)| (*id).to_owned())
        .collect();
    if checked {
        if !chosen.iter().any(|id| id == target) {
            chosen.push(target.to_owned());
        }
    } else {
        chosen.retain(|id| id != target);
    }
    *selection = if chosen.is_empty() {
        Selection::NoneSelected
    } else {
        Selection::Selected(chosen)
    };
}

/// Checkbox changes must update the actual Selection, never a temporary bool.
pub(super) fn multi_select_items(
    ui: &mut egui::Ui,
    selection: &mut Selection,
    items: &[(&str, String, bool)],
) {
    egui::ScrollArea::vertical()
        .max_height(208.)
        .show(ui, |ui| {
            for (id, label, on_budget) in items {
                let mut checked = selection_item_checked(selection, id, *on_budget);
                if popover::checkbox(ui, &mut checked, RichText::new(label).size(12.), false)
                    .changed()
                {
                    toggle_selection_item(selection, id, checked, items);
                }
            }
        });
}
