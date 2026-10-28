use super::*;

impl App {
    pub(in crate::ui) fn payee_settings(&mut self, ui: &mut egui::Ui, c: &mut Chrome) {
        let mut names: Vec<_> = self
            .b
            .transactions
            .iter()
            .filter(|t| t.transfer.is_none())
            .map(|t| t.payee.clone())
            .filter(|n| !n.is_empty())
            .collect();
        names.extend(
            self.b
                .transactions
                .iter()
                .chain(self.b.scheduled.iter().map(|s| &s.transaction))
                .flat_map(|t| t.splits.iter())
                .filter(|s| s.transfer.is_none())
                .filter_map(|s| s.payee.clone())
                .filter(|s| !s.is_empty()),
        );
        names.extend(self.b.payee_rules.keys().cloned());
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup();
        ui.horizontal_top(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("payees_list")
                .max_height(365.)
                .show(ui, |ui| {
                    ui.set_min_width(190.);
                    for (i, name) in names.iter().enumerate() {
                        Frame::new()
                            .fill(if name == &c.payee {
                                Color32::from_rgb(255, 247, 168)
                            } else if i % 2 == 0 {
                                PALE
                            } else {
                                Color32::WHITE
                            })
                            .show(ui, |ui| {
                                if ui.selectable_label(name == &c.payee, name).clicked() {
                                    c.payee = name.clone();
                                    c.renamed_payee = name.clone();
                                    c.rename_pattern = self
                                        .b
                                        .payee_rules
                                        .get(name)
                                        .map(|r| r.rename_contains.clone())
                                        .unwrap_or_default();
                                }
                            });
                    }
                });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(410.);
                ui.set_min_height(365.);
                if c.payee.is_empty() {
                    ui.add_space(160.);
                    ui.l_label("Выберите получателя для редактирования");
                } else {
                    ui.l_label(RichText::new(&c.payee).font(crate::palette::bold(16.)));
                    let count = self
                        .b
                        .transactions
                        .iter()
                        .filter(|t| {
                            t.payee == c.payee
                                || t.splits.iter().any(|s| {
                                    s.transfer.is_none() && s.payee.as_deref() == Some(&c.payee)
                                })
                        })
                        .count();
                    ui.l_label(format!("Операций: {count}"));
                    ui.add_space(15.);
                    ui.l_label("Новое название:");
                    ui.add(egui::TextEdit::singleline(&mut c.renamed_payee).desired_width(300.));
                    if classic::button(ui, "Переименовать").clicked() {
                        if c.renamed_payee.trim().is_empty() {
                            c.error = "Введите название получателя".into();
                        } else {
                            let old = c.payee.clone();
                            let name = c.renamed_payee.trim().to_owned();
                            let mut b = self.b.clone();
                            let renamed = b.rename_payee(&old, &name);
                            match renamed.and_then(|()| self.commit(b)) {
                                Ok(()) => {
                                    c.payee = name;
                                    c.error.clear();
                                }
                                Err(e) => c.error = e,
                            }
                        }
                    }
                    ui.add_space(12.);
                    ui.separator();
                    let mut rule = self
                        .b
                        .payee_rules
                        .get(&c.payee)
                        .cloned()
                        .unwrap_or_default();
                    let mut changed = ui
                        .l_checkbox(
                            &mut rule.listed,
                            "Показывать и автозаполнять этого получателя",
                        )
                        .changed();
                    ui.l_label("Автоматически выбирать категорию:");
                    let before = rule.category.clone();
                    let mut category = rule.category.clone().unwrap_or_default();
                    category_combo(ui, "payee_category", &mut category, &self.b);
                    rule.category = if category.is_empty() {
                        None
                    } else {
                        Some(category)
                    };
                    changed |= before != rule.category;
                    ui.add_space(8.);
                    ui.l_label("Переименовывать импорт, если получатель содержит:");
                    let input = ui
                        .add(egui::TextEdit::singleline(&mut c.rename_pattern).desired_width(300.));
                    rule.rename_contains = c.rename_pattern.clone();
                    changed |= input.lost_focus()
                        && self
                            .b
                            .payee_rules
                            .get(&c.payee)
                            .map(|r| r.rename_contains.as_str())
                            .unwrap_or("")
                            != rule.rename_contains;
                    if changed {
                        let mut b = self.b.clone();
                        b.payee_rules.insert(c.payee.clone(), rule);
                        if let Err(e) = self.commit(b) {
                            c.error = e;
                        }
                    }
                    ui.add_space(20.);
                    if classic::button(ui, "Показать операции").clicked() {
                        self.page = Page::Accounts;
                        self.account = None;
                        self.search = c.payee.clone();
                        c.overlay = None;
                    }
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.l_label("Автозаполнение:");
            let changed = ui
                .l_checkbox(&mut self.preferences.autocomplete_category, "Категория")
                .changed()
                | ui.l_checkbox(&mut self.preferences.autocomplete_memo, "Заметка")
                    .changed()
                | ui.l_checkbox(&mut self.preferences.autocomplete_amount, "Сумма")
                    .changed();
            if changed {
                self.save_preferences();
            }
        });
    }
}
