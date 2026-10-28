use super::*;

pub(super) fn filter_combo(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    content: impl FnOnce(&mut egui::Ui),
) {
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.bg_stroke = Stroke::NONE;
        widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        widgets.inactive.bg_fill = Color32::TRANSPARENT;
        egui::ComboBox::from_id_salt(id)
            .selected_text(
                RichText::new(label)
                    .color(Color32::from_rgb(0, 151, 205))
                    .strong(),
            )
            .width(190.)
            .show_ui(ui, content);
    });
}

/// Changes remain a draft until Done; Cancel, Escape and outside clicks discard it.
pub(super) fn multi_select_combo(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    selection: &mut Selection,
    show_budget_preset: bool,
    content: impl FnOnce(&mut egui::Ui, &mut Selection),
) {
    ui.scope(|ui| {
        let draft_id = ui.make_persistent_id((id, "draft"));
        let combo_id = ui.make_persistent_id(egui::Id::new(id));
        let mut draft = if egui::ComboBox::is_open(ui.ctx(), combo_id) {
            ui.data_mut(|data| data.get_temp::<Selection>(draft_id))
                .unwrap_or_else(|| selection.clone())
        } else {
            selection.clone()
        };
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.bg_stroke = Stroke::NONE;
        widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        widgets.inactive.bg_fill = Color32::TRANSPARENT;
        egui::ComboBox::from_id_salt(id)
            .selected_text(
                RichText::new(label)
                    .color(Color32::from_rgb(0, 151, 205))
                    .strong(),
            )
            .width(190.)
            .height(420.)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show_ui(ui, |ui| {
                if show_budget_preset {
                    ui.selectable_value(&mut draft, Selection::BudgetAccounts, "Budget Accounts");
                    ui.separator();
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!draft.is_all(), classic::Button::new("Select All").small())
                        .clicked()
                    {
                        draft = Selection::All;
                    }
                    if ui
                        .add_enabled(
                            !draft.is_empty(),
                            classic::Button::new("Select None").small(),
                        )
                        .clicked()
                    {
                        draft = Selection::NoneSelected;
                    }
                });
                ui.separator();
                content(ui, &mut draft);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Done").clicked() {
                        *selection = draft.clone();
                        ui.close();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close();
                    }
                });
            });
        let open = egui::ComboBox::is_open(ui.ctx(), combo_id);
        ui.data_mut(|data| {
            if open {
                data.insert_temp(draft_id, draft);
            } else {
                data.remove::<Selection>(draft_id);
            }
        });
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
        .map(|category| (category.id.as_str(), category.name.clone(), true))
        .collect();
    items.push((
        budgettinker::model::UNCATEGORIZED_CATEGORY,
        "Uncategorized Transactions".into(),
        true,
    ));
    let mut groups: Vec<_> = budget
        .categories
        .iter()
        .map(|category| category.group.as_str())
        .collect();
    groups.sort_unstable();
    groups.dedup();
    egui::ScrollArea::vertical()
        .max_height(280.)
        .show(ui, |ui| {
            for group in groups {
                let members: Vec<_> = budget
                    .categories
                    .iter()
                    .filter(|category| category.group == group)
                    .map(|category| category.id.as_str())
                    .collect();
                let checked_count = members
                    .iter()
                    .filter(|id| selection_item_checked(selection, id, true))
                    .count();
                let mut checked = checked_count == members.len();
                let partial = checked_count > 0 && !checked;
                if ui
                    .add(
                        egui::Checkbox::new(&mut checked, RichText::new(group).strong())
                            .indeterminate(partial),
                    )
                    .changed()
                {
                    for member in &members {
                        toggle_selection_item(selection, member, checked, &items);
                    }
                }
                ui.indent(group, |ui| {
                    for member in &members {
                        let item = items.iter().find(|(id, _, _)| id == member).unwrap();
                        let mut checked = selection_item_checked(selection, member, true);
                        if ui.checkbox(&mut checked, &item.1).changed() {
                            toggle_selection_item(selection, member, checked, &items);
                        }
                    }
                });
            }
            let mut checked = selection_item_checked(
                selection,
                budgettinker::model::UNCATEGORIZED_CATEGORY,
                true,
            );
            if ui
                .checkbox(&mut checked, "Uncategorized Transactions")
                .changed()
            {
                toggle_selection_item(
                    selection,
                    budgettinker::model::UNCATEGORIZED_CATEGORY,
                    checked,
                    &items,
                );
            }
        });
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
        .max_height(280.)
        .show(ui, |ui| {
            for (id, label, on_budget) in items {
                let mut checked = selection_item_checked(selection, id, *on_budget);
                if ui.checkbox(&mut checked, label.as_str()).changed() {
                    toggle_selection_item(selection, id, checked, items);
                }
            }
        });
}
