use super::*;

impl App {
    pub(in crate::ui) fn report_filters(&mut self, ui: &mut egui::Ui) {
        Frame::new()
            .fill(Color32::from_gray(243))
            .stroke(Stroke::new(1_f32, Color32::from_gray(221)))
            .corner_radius(3)
            .inner_margin(10)
            .show(ui, |ui| {
                ui.set_min_width((ui.available_width() - 1.).max(0.));
                ui.spacing_mut().button_padding.y = 1.;
                ui.spacing_mut().interact_size.y = 20.;
                egui::Grid::new("report_filters")
                    .spacing([14., 3.])
                    .show(ui, |ui| {
                        ui.label("Timeframe:");
                        timeframe::timeframe(
                            ui,
                            &self.b,
                            &mut self.report_period,
                            &mut self.report_filter,
                        );
                        ui.end_row();
                        ui.label("Categories:");
                        ui.add_enabled_ui(self.report != 4, |ui| {
                            self.multi_select_category(ui);
                        });
                        ui.end_row();
                        ui.label("Payees:");
                        ui.add_enabled_ui(self.report != 4, |ui| {
                            self.multi_select_payee(ui);
                        });
                        ui.end_row();
                        ui.label("Accounts:");
                        self.multi_select_account(ui);
                        ui.end_row();
                    });
            });
    }

    /// Multi-select control for categories. Show group names to disambiguate duplicates.
    pub(in crate::ui) fn multi_select_category(&mut self, ui: &mut egui::Ui) {
        let count = self.b.categories.len() + 1;
        let label = match &self.report_filter.category {
            Selection::All | Selection::BudgetAccounts => "All Categories".to_owned(),
            Selection::NoneSelected => "No Categories".to_owned(),
            Selection::Selected(ids) => {
                let chosen = self
                    .b
                    .categories
                    .iter()
                    .filter(|c| ids.contains(&c.id))
                    .count()
                    + usize::from(
                        ids.iter()
                            .any(|id| id == budgettinker::model::UNCATEGORIZED_CATEGORY),
                    );
                if chosen == 0 {
                    "No Categories".to_owned()
                } else {
                    format!("{chosen}/{count} Categories")
                }
            }
        };
        multi_select_combo(
            ui,
            "report_category",
            &label,
            &mut self.report_filter.category,
            false,
            |ui, selection| {
                category_selection_items(ui, selection, &self.b);
            },
        );
    }

    /// Multi-select control for payees, including the empty payee.
    pub(in crate::ui) fn multi_select_payee(&mut self, ui: &mut egui::Ui) {
        let mut payees: Vec<_> = self
            .b
            .transactions
            .iter()
            .flat_map(|t| {
                std::iter::once(t.payee.clone())
                    .chain(t.splits.iter().map(|s| self.b.split_payee_name(t, s)))
            })
            .collect();
        payees.sort();
        payees.dedup();
        let count = payees.len();
        let label = match &self.report_filter.payee {
            Selection::All | Selection::BudgetAccounts => "All Payees".to_owned(),
            Selection::NoneSelected => "No Payees".to_owned(),
            Selection::Selected(ids) => {
                let chosen = payees.iter().filter(|p| ids.contains(p)).count();
                if chosen == 0 {
                    "No Payees".to_owned()
                } else {
                    format!("{chosen}/{count} Payees")
                }
            }
        };
        let items: Vec<_> = payees
            .iter()
            .map(|p| {
                (
                    p.as_str(),
                    if p.is_empty() {
                        "[No Payee]".to_owned()
                    } else {
                        p.clone()
                    },
                    true,
                )
            })
            .collect();
        multi_select_combo(
            ui,
            "report_payee",
            &label,
            &mut self.report_filter.payee,
            false,
            |ui, selection| {
                multi_select_items(ui, selection, &items);
            },
        );
    }

    /// Budget Accounts and All Accounts are separate presets.
    pub(in crate::ui) fn multi_select_account(&mut self, ui: &mut egui::Ui) {
        let count = self.b.accounts.len();
        let label = match &self.report_filter.accounts {
            Selection::BudgetAccounts => "Budget Accounts".to_owned(),
            Selection::All => "All Accounts".to_owned(),
            Selection::NoneSelected => "No Accounts".to_owned(),
            Selection::Selected(ids) => {
                let chosen = self
                    .b
                    .accounts
                    .iter()
                    .filter(|a| ids.contains(&a.id))
                    .count();
                if chosen == 0 {
                    "No Accounts".to_owned()
                } else {
                    format!("{chosen}/{count} Accounts")
                }
            }
        };
        let items: Vec<_> = self
            .b
            .accounts
            .iter()
            .map(|a| (a.id.as_str(), a.name.clone(), a.on_budget))
            .collect();
        multi_select_combo(
            ui,
            "report_accounts",
            &label,
            &mut self.report_filter.accounts,
            true,
            |ui, selection| {
                account_selection_items(ui, selection, &items, &self.b);
            },
        );
    }
}
