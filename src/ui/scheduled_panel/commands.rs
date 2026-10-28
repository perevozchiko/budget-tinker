use super::*;

impl App {
    pub(in crate::ui) fn schedule_menu(&mut self, ui: &mut egui::Ui, rows: &[Scheduled]) {
        let selected: Vec<_> = rows
            .iter()
            .filter(|s| self.scheduled_selected.contains(&s.id))
            .cloned()
            .collect();
        if ui
            .add_enabled(
                !selected.is_empty(),
                egui::Button::new("Enter in Register Now").frame_when_inactive(false),
            )
            .clicked()
        {
            self.change(|b| {
                let mut handled = HashSet::new();
                for s in &selected {
                    if handled.contains(&s.id) {
                        continue;
                    }
                    handled.extend(b.schedule_ids(&s.id)?);
                    b.enter_scheduled_now(&s.id)?;
                }
                Ok(())
            });
            self.scheduled_selected.clear();
            ui.close();
        }
        if ui
            .add_enabled(
                !selected.is_empty(),
                egui::Button::new("Skip this Payment").frame_when_inactive(false),
            )
            .clicked()
        {
            self.change(|b| {
                let mut handled = HashSet::new();
                for s in &selected {
                    if handled.contains(&s.id) {
                        continue;
                    }
                    handled.extend(b.schedule_ids(&s.id)?);
                    b.skip_scheduled(&s.id)?;
                }
                Ok(())
            });
            self.scheduled_selected.clear();
            ui.close();
        }
        if ui
            .add_enabled(
                !selected.is_empty(),
                egui::Button::new("Edit Transactions").frame_when_inactive(false),
            )
            .clicked()
        {
            if selected.len() == 1 {
                open_schedule_editor(self, &selected[0]);
            } else {
                self.notify("Редактирование доступно для одного плана", true);
            }
            ui.close();
        }
        ui.add_enabled_ui(!selected.is_empty(), |ui| {
            ui.menu_button("Move to Account", |ui| {
                let accounts = self.b.accounts.clone();
                for account in accounts.iter().filter(|a| !a.closed) {
                    let eligible = selected.iter().all(|s| {
                        let mut next = self.b.clone();
                        s.transaction.account != account.id
                            && next.move_scheduled_account(&s.id, &account.id).is_ok()
                    });
                    if ui
                        .add_enabled(
                            eligible,
                            egui::Button::new(&account.name).frame_when_inactive(false),
                        )
                        .clicked()
                    {
                        self.change(|b| {
                            let mut links = HashSet::new();
                            for s in &selected {
                                if let Some(link) = &s.transaction.transfer
                                    && !links.insert(link.clone())
                                {
                                    continue;
                                }
                                b.move_scheduled_account(&s.id, &account.id)?;
                            }
                            Ok(())
                        });
                        self.scheduled_selected.clear();
                        ui.close();
                    }
                }
            });
        });
        if ui
            .add_enabled(
                !selected.is_empty(),
                egui::Button::new("Delete").frame_when_inactive(false),
            )
            .clicked()
        {
            self.change(|b| b.delete_scheduled(&selected.iter().map(|s| s.id.clone()).collect()));
            self.scheduled_selected.clear();
            ui.close();
        }
    }
}
