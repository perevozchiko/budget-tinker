use super::*;

impl App {
    pub(in crate::ui) fn budget(&mut self, ui: &mut egui::Ui) {
        let mut editor = std::mem::take(&mut self.budget_editor);
        if ui.is_enabled() && self.dialog.is_none() && self.chrome.overlay.is_none() {
            editor.begin(ui.ctx());
        }
        let label_w = 184.;
        let count = (((ui.available_width() - label_w) / 285.).floor() as i32).clamp(1, 4);
        let col_w = (ui.available_width() - label_w - count as f32 * 8.) / count as f32;
        self.budget_navigation(ui, label_w, count);
        let months: Vec<_> = (0..count).map(|n| month_shift(&self.month, n)).collect();
        self.budget_summary(ui, &months, label_w, col_w);
        ui.spacing_mut().item_spacing.y = 0.;
        let body = ui.available_rect_before_wrap();
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                body.min + Vec2::new(0., 36.),
                Vec2::new(label_w, body.height() - 36.),
            ),
            6.,
            Color32::WHITE,
        );
        ui.painter().rect_stroke(
            egui::Rect::from_min_size(
                body.min + Vec2::new(0., 36.),
                Vec2::new(label_w, body.height() - 36.),
            ),
            6.,
            Stroke::new(1_f32, classic::BORDER),
            egui::StrokeKind::Inside,
        );
        for i in 0..count {
            let frame = egui::Rect::from_min_size(
                body.min + Vec2::new(label_w + 8. + i as f32 * (col_w + 8.), 36.),
                Vec2::new(col_w, body.height() - 36.),
            );
            ui.painter().rect_filled(frame, 6., Color32::WHITE);
            ui.painter().rect_stroke(
                frame,
                6.,
                Stroke::new(1_f32, classic::BORDER),
                egui::StrokeKind::Inside,
            );
        }
        let pointer = ui.ctx().input(|i| i.pointer.latest_pos());
        let released = ui
            .ctx()
            .input(|i| i.pointer.button_released(egui::PointerButton::Primary));
        let mut drop_target: Option<(String, Option<(String, bool)>)> = None;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let total = label_w + count as f32 * (col_w + 8.);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(total, 35.), egui::Sense::hover());
                let category_header =
                    egui::Rect::from_min_size(rect.min, Vec2::new(label_w, rect.height()));
                match budget_categories::header(ui, category_header) {
                    budget_categories::HeaderAction::Add => {
                        let draft = egui::Rect::from_min_size(
                            category_header.left_bottom(),
                            Vec2::new(label_w, 23.),
                        );
                        self.open_at(DialogKind::NewGroup, draft)
                    }
                    budget_categories::HeaderAction::Expand => self.collapsed.clear(),
                    budget_categories::HeaderAction::Collapse => {
                        self.collapsed =
                            self.b.categories.iter().map(|c| c.group.clone()).collect();
                    }
                    budget_categories::HeaderAction::None => {}
                }
                for (i, month) in months.iter().enumerate() {
                    let x = rect.left() + label_w + 8. + i as f32 * (col_w + 8.);
                    let panel = egui::Rect::from_min_size(
                        egui::pos2(x, rect.top() + 1.),
                        Vec2::new(col_w, 32.),
                    );
                    classic::gradient(
                        ui.painter(),
                        panel,
                        5.,
                        Color32::from_rgb(245, 250, 236),
                        Color32::from_rgb(231, 240, 222),
                    );
                    ui.painter().rect_stroke(
                        panel,
                        5.,
                        Stroke::new(1_f32, classic::BORDER),
                        egui::StrokeKind::Inside,
                    );
                    let totals = [
                        self.b
                            .categories
                            .iter()
                            .map(|c| self.b.allocated(month, &c.id))
                            .sum::<i64>(),
                        self.b
                            .categories
                            .iter()
                            .map(|c| self.b.activity(month, &c.id))
                            .sum::<i64>(),
                        self.b
                            .categories
                            .iter()
                            .map(|c| self.b.category_balance(month, &c.id))
                            .sum::<i64>(),
                    ];
                    for (j, label) in ["Назначено", "Расход / доход", "Остаток"]
                        .iter()
                        .enumerate()
                    {
                        let center = x + col_w * (j as f32 + 0.5) / 3.;
                        ui.painter().l_text(
                            egui::pos2(center, rect.top() + 10.),
                            egui::Align2::CENTER_CENTER,
                            label,
                            FontId::proportional(11.),
                            INK,
                        );
                        ui.painter().l_text(
                            egui::pos2(center, rect.top() + 24.),
                            egui::Align2::CENTER_CENTER,
                            i18n::money(totals[j], &self.b.base),
                            FontId::proportional(11.),
                            INK,
                        );
                    }
                }
                let mut cats = Vec::new();
                if let Some(d) = self
                    .dialog
                    .as_ref()
                    .filter(|d| matches!(d.kind, DialogKind::NewGroup))
                {
                    cats.push(Category {
                        id: String::new(),
                        group: d.fields[0].clone(),
                        name: String::new(),
                        hidden: false,
                    });
                }
                for group in self.b.group_names() {
                    let visible: Vec<_> = self
                        .b
                        .categories
                        .iter()
                        .filter(|c| c.group == group && !c.hidden)
                        .cloned()
                        .collect();
                    let metadata = self.b.category_groups.iter().find(|g| g.name == group);
                    if visible.is_empty() && metadata.is_some_and(|g| !g.hidden) {
                        cats.push(Category {
                            id: String::new(),
                            group: group.clone(),
                            name: String::new(),
                            hidden: false,
                        });
                    }
                    cats.extend(visible);
                    if let Some(d) = self
                        .dialog
                        .as_ref()
                        .filter(|d| matches!(d.kind, DialogKind::Category) && d.fields[0] == group)
                    {
                        cats.push(Category {
                            id: "__draft_category".into(),
                            group,
                            name: d.fields[1].clone(),
                            hidden: false,
                        });
                    }
                }
                cats.extend(
                    self.b
                        .categories
                        .iter()
                        .filter(|c| c.hidden)
                        .cloned()
                        .map(|mut c| {
                            c.group = "Скрытые категории".into();
                            c
                        }),
                );
                for group in self.b.category_groups.iter().filter(|g| g.hidden) {
                    cats.push(Category {
                        id: format!("__hidden_group:{}", group.name),
                        group: "Скрытые категории".into(),
                        name: group.name.clone(),
                        hidden: true,
                    });
                }
                let has_hidden = self.b.categories.iter().any(|c| c.hidden)
                    || self.b.category_groups.iter().any(|g| g.hidden);
                let mut group = String::new();
                for c in cats {
                    if group != c.group {
                        group = c.group.clone();
                        let collapsed = if group == "Скрытые категории" {
                            !self.collapsed.contains("__show_hidden")
                        } else {
                            self.collapsed.contains(&group)
                        };
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(total, 23.), egui::Sense::hover());
                        if group != "Скрытые категории"
                            && self.dragging_category.is_some()
                            && pointer.is_some_and(|pos| rect.contains(pos))
                        {
                            drop_target = Some((group.clone(), None));
                            ui.painter().rect_stroke(
                                rect.shrink(1.),
                                2.,
                                Stroke::new(2_f32, Color32::from_rgb(0, 151, 204)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        budget_row_fill(
                            ui.painter(),
                            rect,
                            label_w,
                            col_w,
                            count,
                            if pointer.is_some_and(|pos| {
                                rect.contains(pos) && pos.x < rect.left() + label_w
                            }) {
                                crate::palette::CATEGORY_HOVER
                            } else {
                                crate::palette::GROUP
                            },
                        );
                        let group_label =
                            egui::Rect::from_min_size(rect.min, Vec2::new(label_w, rect.height()));
                        let note_key = format!("group:{group}");
                        let has_note = self
                            .b
                            .notes
                            .get(&note_key)
                            .is_some_and(|note| !note.trim().is_empty());
                        if group == "Скрытые категории" {
                            if let Some(action) =
                                budget_categories::hidden_row(ui, group_label, &self.b)
                            {
                                self.change(|budget| {
                                    match action {
                                        budget_categories::UnhideAction::Category(id) => {
                                            if let Some(category) =
                                                budget.categories.iter_mut().find(|c| c.id == id)
                                            {
                                                category.hidden = false;
                                                let group = category.group.clone();
                                                for metadata in &mut budget.category_groups {
                                                    if metadata.name == group {
                                                        metadata.hidden = false;
                                                    }
                                                }
                                            }
                                        }
                                        budget_categories::UnhideAction::Group(group) => {
                                            budget.set_group_hidden(&group, false)?
                                        }
                                        budget_categories::UnhideAction::All => {
                                            for category in &mut budget.categories {
                                                category.hidden = false;
                                            }
                                            for group in &mut budget.category_groups {
                                                group.hidden = false;
                                            }
                                        }
                                    }
                                    Ok(())
                                });
                            }
                        } else {
                            match budget_categories::group_row(
                                ui,
                                group_label,
                                &group,
                                collapsed,
                                has_note,
                                group == "Скрытые категории",
                            ) {
                                budget_categories::GroupAction::Edit => {
                                    self.open_at(DialogKind::EditGroup(group.clone()), group_label)
                                }
                                budget_categories::GroupAction::Toggle => {
                                    let key = if group == "Скрытые категории" {
                                        "__show_hidden"
                                    } else {
                                        &group
                                    };
                                    if !self.collapsed.remove(key) {
                                        self.collapsed.insert(key.to_owned());
                                    }
                                }
                                budget_categories::GroupAction::Add => {
                                    self.collapsed.remove(&group);
                                    let children = self
                                        .b
                                        .categories
                                        .iter()
                                        .filter(|c| c.group == group && !c.hidden)
                                        .count();
                                    let draft = egui::Rect::from_min_size(
                                        group_label.left_bottom()
                                            + Vec2::new(0., children as f32 * 24.),
                                        Vec2::new(label_w, 24.),
                                    );
                                    self.open_at(DialogKind::Category, draft);
                                    if let Some(d) = &mut self.dialog {
                                        d.fields[0] = group.clone();
                                    }
                                }
                                budget_categories::GroupAction::Note => {
                                    self.open_at(DialogKind::Note(note_key), group_label)
                                }
                                budget_categories::GroupAction::None => {}
                            }
                        }
                        for (i, month) in months.iter().enumerate() {
                            let cats: Vec<_> = self
                                .b
                                .categories
                                .iter()
                                .filter(|c| {
                                    if group == "Скрытые категории" {
                                        c.hidden
                                    } else {
                                        !c.hidden && c.group == group
                                    }
                                })
                                .collect();
                            let totals = [
                                cats.iter()
                                    .map(|c| self.b.allocated(month, &c.id))
                                    .sum::<i64>(),
                                cats.iter()
                                    .map(|c| self.b.activity(month, &c.id))
                                    .sum::<i64>(),
                                cats.iter()
                                    .map(|c| self.b.category_balance(month, &c.id))
                                    .sum::<i64>(),
                            ];
                            let x = rect.left() + label_w + 8. + i as f32 * (col_w + 8.);
                            for (j, value) in totals.iter().enumerate() {
                                ui.painter().l_text(
                                    egui::pos2(
                                        x + col_w * (j as f32 + 1.) / 3. - 7.,
                                        rect.center().y,
                                    ),
                                    egui::Align2::RIGHT_CENTER,
                                    i18n::number(*value, &self.b.base),
                                    crate::palette::bold(12.),
                                    if j == 2 && *value < 0 { RED } else { DARK },
                                );
                            }
                        }
                    }
                    if c.id.is_empty() {
                        continue;
                    }
                    if group == "Скрытые категории"
                        || (group != "Скрытые категории" && self.collapsed.contains(&group))
                    {
                        continue;
                    }
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(total, 24.), egui::Sense::hover());
                    #[cfg(test)]
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("budget_category_row", &c.id)), rect));
                    let row_hovered = ui.rect_contains_pointer(rect);
                    budget_row_fill(
                        ui.painter(),
                        rect,
                        label_w,
                        col_w,
                        count,
                        if row_hovered {
                            crate::palette::CATEGORY_HOVER
                        } else {
                            Color32::WHITE
                        },
                    );
                    ui.painter().add(egui::Shape::dashed_line(
                        &[rect.left_bottom(), rect.right_bottom()],
                        Stroke::new(0.6_f32, LINE),
                        1.,
                        2.,
                    ));
                    ui.painter().l_text(
                        rect.left_center() + Vec2::new(23., 0.),
                        egui::Align2::LEFT_CENTER,
                        &c.name,
                        FontId::proportional(13.),
                        INK,
                    );
                    let label_response = ui.interact(
                        egui::Rect::from_min_max(
                            rect.min,
                            egui::pos2(rect.left() + label_w, rect.bottom()),
                        ),
                        ui.id().with(("category_label", &c.id)),
                        egui::Sense::click_and_drag(),
                    );
                    let note_key = format!("category:{}", c.id);
                    let note = self
                        .b
                        .notes
                        .get(&note_key)
                        .filter(|note| !note.trim().is_empty())
                        .cloned();
                    let note_response = if note.is_some() {
                        let name_width = ui
                            .painter()
                            .layout_no_wrap(
                                i18n::tr(ui.ctx(), &c.name),
                                FontId::proportional(13.),
                                INK,
                            )
                            .size()
                            .x;
                        let x = (rect.left() + 27. + name_width).min(rect.left() + label_w - 18.);
                        let icon = egui::Rect::from_min_size(
                            egui::pos2(x, rect.center().y - 8.),
                            Vec2::new(13., 16.),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(icon, 1., Color32::from_rgb(237, 241, 242));
                        painter.rect_stroke(
                            icon,
                            1.,
                            Stroke::new(1_f32, Color32::from_rgb(88, 105, 112)),
                            egui::StrokeKind::Inside,
                        );
                        for offset in [5., 8., 11.] {
                            painter.hline(
                                icon.left() + 3. ..=icon.right() - 3.,
                                icon.top() + offset,
                                Stroke::new(1_f32, Color32::from_rgb(88, 105, 112)),
                            );
                        }
                        Some(ui.interact(
                            icon,
                            ui.id().with(("category_note", &c.id)),
                            egui::Sense::click(),
                        ))
                    } else {
                        None
                    };
                    let note_clicked = note_response.as_ref().is_some_and(egui::Response::clicked);
                    let tooltip_response = note_response
                        .as_ref()
                        .filter(|response| response.hovered())
                        .unwrap_or(&label_response);
                    category_hover_tooltip(tooltip_response, &c.name, note.as_deref());
                    if label_response.drag_started_by(egui::PointerButton::Primary) {
                        self.dragging_category = Some(c.id.clone());
                    }
                    if !c.hidden
                        && self.dragging_category.is_some()
                        && self.dragging_category.as_deref() != Some(c.id.as_str())
                        && let Some(pos) = pointer
                        && rect.contains(pos)
                    {
                        let after = pos.y > rect.center().y;
                        drop_target = Some((c.group.clone(), Some((c.id.clone(), after))));
                        let y = if after { rect.bottom() } else { rect.top() };
                        ui.painter().hline(
                            rect.left()..=rect.left() + label_w,
                            y,
                            Stroke::new(2_f32, Color32::from_rgb(0, 151, 204)),
                        );
                    }
                    if note_clicked {
                        self.open_at(DialogKind::Note(note_key), rect);
                    } else if label_response.clicked() && c.id != "__draft_category" {
                        if let Some(name) = c.id.strip_prefix("__hidden_group:") {
                            self.open_at(DialogKind::EditGroup(name.into()), rect);
                        } else {
                            self.open_at(DialogKind::EditCategory(c.id.clone()), rect);
                        }
                    }
                    for (i, month) in months.iter().enumerate() {
                        let x = rect.left() + label_w + 8. + i as f32 * (col_w + 8.);
                        let values = [
                            self.b.allocated(month, &c.id),
                            self.b.activity(month, &c.id),
                            self.b.category_balance(month, &c.id),
                        ];
                        for (j, value) in values.into_iter().enumerate() {
                            let cell = egui::Rect::from_min_size(
                                egui::pos2(x + j as f32 * col_w / 3., rect.top()),
                                Vec2::new(col_w / 3., rect.height()),
                            );
                            if j == 0 {
                                #[cfg(test)]
                                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("budget_cell", month, &c.id)), cell));
                                ui.painter().rect_stroke(
                                    cell,
                                    0.,
                                    Stroke::new(0.7_f32, crate::palette::CELL),
                                    egui::StrokeKind::Inside,
                                );
                                let editing_cell = BudgetCell { month: month.clone(), category: c.id.clone() };
                                let options = BudgetCellOptions::for_budget(&self.b, &editing_cell, row_hovered, editor.is_editing(&editing_cell));
                                editor.show(
                                    ui,
                                    cell,
                                    editing_cell,
                                    value,
                                    &self.b.base,
                                    options,
                                );
                                continue;
                            }
                            if j > 0 {
                                ui.painter().add(egui::Shape::dashed_line(
                                    &[cell.left_top(), cell.left_bottom()],
                                    Stroke::new(0.6_f32, LINE),
                                    1.,
                                    2.,
                                ));
                            }
                            if j == 2 && value < 0 {
                                ui.painter().rect_filled(
                                    cell.shrink2(Vec2::new(3., 6.)),
                                    4.,
                                    if value < 0 {
                                        Color32::from_rgb(251, 224, 221)
                                    } else {
                                        Color32::from_rgb(231, 241, 214)
                                    },
                                );
                                let response = ui.interact(cell, ui.id().with(("overspending", month, &c.id)), egui::Sense::click())
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .l_on_hover_text("Нажмите, чтобы выбрать правило перерасхода для этой категории.");
                                if response.clicked() {
                                    self.open_at(DialogKind::Overspending { month: month.clone(), category: c.id.clone() }, cell);
                                }
                            }
                            ui.painter().l_text(
                                cell.right_center() - Vec2::new(8., 0.),
                                egui::Align2::RIGHT_CENTER,
                                if value == 0 {
                                    String::new()
                                } else {
                                    i18n::number(value, &self.b.base)
                                },
                                FontId::proportional(12.),
                                if j == 2 && value < 0 { RED } else { INK },
                            );
                        }
                    }
                }
                if !has_hidden {
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(total, 23.), egui::Sense::hover());
                    budget_row_fill(
                        ui.painter(),
                        rect,
                        label_w,
                        col_w,
                        count,
                        crate::palette::GROUP,
                    );
                    ui.painter().l_text(
                        rect.left_center() + Vec2::new(8., 0.),
                        egui::Align2::LEFT_CENTER,
                        "Скрытые категории",
                        crate::palette::bold(12.),
                        INK,
                    );
                    for i in 0..count {
                        for j in 0..3 {
                            ui.painter().text(
                                egui::pos2(
                                    rect.left()
                                        + label_w
                                        + 8.
                                        + i as f32 * (col_w + 8.)
                                        + col_w * (j as f32 + 1.) / 3.
                                        - 7.,
                                    rect.center().y,
                                ),
                                egui::Align2::RIGHT_CENTER,
                                "0.00",
                                crate::palette::bold(12.),
                                Color32::from_gray(155),
                            );
                        }
                    }
                }
            });
        ui.painter().rect_stroke(
            egui::Rect::from_min_size(
                body.min + Vec2::new(0., 36.),
                Vec2::new(label_w, body.height() - 36.),
            ),
            6.,
            Stroke::new(1.2_f32, classic::BORDER),
            egui::StrokeKind::Inside,
        );
        if released
            && let Some(source) = self.dragging_category.take()
            && let Some((group, neighbor)) = drop_target
        {
            self.change(|budget| {
                budget.move_category(
                    &source,
                    &group,
                    neighbor.as_ref().map(|(id, after)| (id.as_str(), *after)),
                )
            });
        }
        let order: Vec<_> = months
            .iter()
            .flat_map(|m| {
                self.b
                    .categories
                    .iter()
                    .filter(|c| {
                        if c.hidden {
                            false
                        } else {
                            !self.collapsed.contains(&c.group)
                        }
                    })
                    .map(|c| {
                        (
                            BudgetCell {
                                month: m.clone(),
                                category: c.id.clone(),
                            },
                            self.b.allocated(m, &c.id),
                        )
                    })
            })
            .collect();
        let base = self.b.base.clone();
        if ui.is_enabled() && self.dialog.is_none() && self.chrome.overlay.is_none() {
            editor.finish(ui.ctx(), &base, &order, |cell, value| {
                let mut next = self.b.clone();
                next.allocate(&cell.month, &cell.category, value)?;
                self.commit(next)
            });
        }
        self.budget_editor = editor;
    }
}

mod navigation;
mod summary;

mod quick_menu;

mod income_popup;

#[cfg(test)]
mod editor_tests;
