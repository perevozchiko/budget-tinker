use super::*;

impl App {
    pub(in crate::ui) fn apply_upgrade(
        &mut self,
        preview: &store::upgrade::UpgradePreview,
    ) -> Result<()> {
        let parent = self
            .store
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_owned();
        let (budget, backup) = self.store.upgrade(preview, &parent)?;
        self.b = budget;
        self.undo.clear();
        self.redo.clear();
        self.transaction_editor = None;
        self.budget_editor = BudgetEditor::default();
        self.scheduled_editor = None;
        self.reconciliation = None;
        self.dialog = None;
        self.selected.clear();
        self.scheduled_selected.clear();
        self.chrome.screen = None;
        self.chrome.overlay = Some(Overlay::Upgraded(backup.display().to_string()));
        self.chrome.upgrade = None;
        self.chrome.versions_dirty = true;
        self.notify(
            format!(
                "Currency data upgraded. Recovery copy: {}",
                backup.display()
            ),
            false,
        );
        Ok(())
    }

    pub(in crate::ui) fn upgrade_dialog(&mut self, ctx: &egui::Context) {
        let Some(preview) = self.chrome.upgrade.take() else {
            return;
        };
        let mut confirm = false;
        let mut cancel = false;
        let response = modal(ctx, "currency_upgrade").show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            header(ui, "Upgrade currency data", 560.);
            body(ui, 560., |ui| {
                ui.label("Freeze each operation's exchange rate and split base amounts independently of monthly rates?");
                ui.add_space(12.);
                ui.label(format!("{} operations and {} saved revisions have passed validation.",
                    preview.current.budget.transactions.len(), preview.revisions.len()));
                ui.add_space(12.);
                ui.label("A verified recovery copy will be saved beside this budget before any changes. All saved revisions will be upgraded together. Undo and Redo history will be cleared. Older application versions cannot open the upgraded file.");
                ui.add_space(8.);
                ui.label("To recover the original, open original.budgettinker in the pre-upgrade backup folder.");
            });
            footer(ui, 560., |ui| {
                confirm = classic::button(ui, "Back Up and Upgrade").clicked();
                cancel = classic::button(ui, "Cancel").clicked();
            });
        });
        cancel |= response.should_close()
            || ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if confirm {
            if let Err(error) = self.apply_upgrade(&preview) {
                self.chrome.error = error.clone();
                self.notify(error, true);
            }
        } else if !cancel {
            self.chrome.upgrade = Some(preview);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        app: App,
        ctx: egui::Context,
        root: std::path::PathBuf,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("upgrade-ui-{}", id()));
        let mut b = Budget::new("EUR");
        b.add_account("Synthetic", "EUR", true, 10000, "2026-01-01")
            .unwrap();
        let mut store = Store::open(&root.join("demo.budgettinker")).unwrap();
        store.save(&b).unwrap();
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            store,
            b,
            false,
        );
        for name in ["First", "Second"] {
            let mut b = app.b.clone();
            b.name = name.into();
            app.commit(b).unwrap();
        }
        app.undo(false);
        app.screen(Screen::Properties);
        Fixture { app, ctx, root }
    }
    fn frame(f: &mut Fixture, events: Vec<egui::Event>) -> egui::FullOutput {
        f.ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1050., 680.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                if f.app.chrome.upgrade.is_some() {
                    egui::CentralPanel::default().show(ctx, |_| {});
                    f.app.upgrade_dialog(ctx);
                } else {
                    f.app.chrome_screen(ctx);
                }
            },
        )
    }
    fn click(f: &mut Fixture, label: &str) {
        frame(f, vec![]);
        let output = frame(f, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + Vec2::splat(3.)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {label}"));
        for pressed in [true, false] {
            frame(
                f,
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
    fn value(b: &Budget) -> serde_json::Value {
        serde_json::to_value(b).unwrap()
    }

    #[test]
    fn upgrade_confirmation_cancel_escape_and_success_preserve_or_reset_history() {
        let mut f = fixture();
        let original = value(&f.app.b);
        let histories = (f.app.undo.len(), f.app.redo.len());
        assert!(histories.0 > 0 && histories.1 > 0);
        click(&mut f, "Upgrade currency data…");
        assert!(f.app.chrome.upgrade.is_some());
        assert_eq!(value(&f.app.store.load().unwrap().unwrap()), original);
        click(&mut f, "Cancel");
        assert!(f.app.chrome.upgrade.is_none());
        click(&mut f, "Upgrade currency data…");
        frame(
            &mut f,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(f.app.chrome.upgrade.is_none());
        assert_eq!((f.app.undo.len(), f.app.redo.len()), histories);
        assert_eq!(value(&f.app.b), original);
        assert!(
            std::fs::read_dir(&f.root)
                .unwrap()
                .all(|e| !e.unwrap().path().is_dir())
        );
        click(&mut f, "Upgrade currency data…");
        click(&mut f, "Back Up and Upgrade");
        assert_eq!(f.app.b.version, 2);
        assert!(f.app.undo.is_empty() && f.app.redo.is_empty());
        assert_eq!(
            value(&f.app.store.load().unwrap().unwrap()),
            value(&f.app.b)
        );
        let Some(Overlay::Upgraded(path)) = &f.app.chrome.overlay else {
            panic!("missing recovery message")
        };
        assert_eq!(
            value(&store::read_budget(Path::new(path)).unwrap()),
            original
        );
        f.app.undo(false);
        assert_eq!(f.app.b.version, 2);
        let upgraded = value(&f.app.b);
        let mut edited = f.app.b.clone();
        edited.name = "After upgrade".into();
        f.app.commit(edited).unwrap();
        f.app.undo(false);
        assert_eq!(value(&f.app.b), upgraded);
        f.app.undo(true);
        assert_eq!(f.app.b.name, "After upgrade");
        assert_eq!(
            value(&f.app.store.load().unwrap().unwrap()),
            value(&f.app.b)
        );
    }

    #[test]
    fn stale_upgrade_does_not_clear_history_or_replace_in_memory_state() {
        let mut f = fixture();
        let original = value(&f.app.b);
        let histories = (f.app.undo.len(), f.app.redo.len());
        click(&mut f, "Upgrade currency data…");
        let mut changed = f.app.b.clone();
        changed.name = "External change".into();
        f.app.store.save(&changed).unwrap();
        click(&mut f, "Back Up and Upgrade");
        assert!(f.app.is_error && f.app.notice.contains("changed"));
        assert_eq!(value(&f.app.b), original);
        assert_eq!((f.app.undo.len(), f.app.redo.len()), histories);
        assert_eq!(
            value(&f.app.store.load().unwrap().unwrap()),
            value(&changed)
        );
    }
}
