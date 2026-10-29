use super::*;

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, store: Store, b: Budget, first: bool) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf") {
            fonts
                .font_data
                .insert("arial".into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "arial".into());
        }
        if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Supplemental/Arial Unicode.ttf") {
            fonts
                .font_data
                .insert("unicode".into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("unicode".into());
        }
        if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Supplemental/Arial Bold.ttf") {
            fonts.font_data.insert(
                "arial-bold".into(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts.families.insert(
                egui::FontFamily::Name("arial-bold".into()),
                vec!["arial-bold".into(), "arial".into()],
            );
        }
        if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf")
        {
            fonts.font_data.insert(
                "arial-bold-italic".into(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts.families.insert(
                egui::FontFamily::Name("arial-bold-italic".into()),
                vec!["arial-bold-italic".into(), "arial".into()],
            );
        }
        crate::palette::ensure_font_families(&mut fonts);
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Light));
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::light();
        style.visuals.override_text_color = Some(INK);
        style.visuals.panel_fill = Color32::WHITE;
        style.visuals.window_fill = Color32::from_rgb(247, 249, 249);
        style.visuals.selection.bg_fill = TEAL;
        style.visuals.widgets.inactive.bg_fill = Color32::from_gray(236);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(236);
        style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(3);
        style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(3);
        style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(3);
        style.visuals.selection.bg_fill = Color32::from_rgb(168, 203, 232);
        style.visuals.selection.stroke = Stroke::new(1_f32, INK);
        style.spacing.interact_size.y = 24.;
        style.visuals.widgets.inactive.bg_stroke =
            Stroke::new(1_f32, Color32::from_rgb(175, 188, 192));
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(217, 237, 243);
        style.spacing.item_spacing = Vec2::new(8., 6.);
        style.spacing.button_padding = Vec2::new(10., 6.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(13.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(13.));
        cc.egui_ctx.set_style(style);
        let preferences = crate::preferences::Preferences::load(&store.path).unwrap_or_default();
        preferences.language.install(&cc.egui_ctx);
        let _ = preferences.save(&store.path);
        let fabric = crate::palette::fabric(&cc.egui_ctx);
        let mut app = Self {
            store,
            b,
            page: Page::Budget,
            month: current_month(),
            account: None,
            search: String::new(),
            register_timeframe: crate::register_timeframe::Timeframe::default(),
            register_ascending: false,
            register_sort: 2,
            register_scheduled_open: false,
            scheduled_selected: HashSet::new(),
            scheduled_editor: None,
            reconciliation: None,
            dialog: None,
            notice: "Все изменения сохраняются автоматически на этом Mac".into(),
            is_error: false,
            undo: vec![],
            redo: vec![],
            selected: HashSet::new(),
            report: 0,
            report_filter: budgettinker::reports::Filter::default(),
            report_accounts: report_view::ReportAccounts::default(),
            report_period: 5,
            report_tabs: Default::default(),
            report_selected_month: None,
            report_navigation: vec![],
            collapsed: HashSet::new(),
            dragging_category: None,
            budget_editor: BudgetEditor::default(),
            transaction_editor: None,
            preferences: preferences.clone(),
            fabric,
            window_title: String::new(),
            native_menu: crate::native_menu::NativeMenu::new(preferences.language, &cc.egui_ctx),
            chrome: chrome::Chrome {
                versions_dirty: true,
                ..Default::default()
            },
        };
        app.restore_report_preferences();
        if first {
            app.screen(chrome::Screen::Properties);
        }
        app
    }
    pub(in crate::ui) fn notify(&mut self, s: impl Into<String>, error: bool) {
        self.notice = s.into();
        self.is_error = error;
    }
    pub(in crate::ui) fn commit(&mut self, next: Budget) -> Result<()> {
        self.store.save(&next)?;
        self.chrome.versions_dirty = true;
        self.undo.push(self.b.clone());
        if self.undo.len() > 50 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.b = next;
        if self
            .reconciliation
            .as_ref()
            .is_some_and(|s| self.b.account(&s.account).is_err())
        {
            self.reconciliation = None;
        }
        self.scheduled_selected
            .retain(|id| self.b.scheduled.iter().any(|s| &s.id == id));
        if self.scheduled_editor.as_ref().is_some_and(|(d, _)| {
            d.id.as_ref()
                .is_some_and(|id| !self.b.scheduled.iter().any(|s| &s.id == id))
        }) {
            self.scheduled_editor = None;
        }
        if self
            .account
            .as_ref()
            .is_some_and(|a| self.b.account(a).is_err())
        {
            self.account = None;
        }
        self.notify("Сохранено локально", false);
        Ok(())
    }
    pub(in crate::ui) fn change(&mut self, f: impl FnOnce(&mut Budget) -> Result<()>) {
        let mut next = self.b.clone();
        match f(&mut next).and_then(|_| self.commit(next)) {
            Ok(()) => {}
            Err(e) => self.notify(e, true),
        }
    }
    pub(in crate::ui) fn undo(&mut self, redo: bool) {
        let state = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(next) = state {
            match self.store.save(&next) {
                Ok(()) => {
                    self.chrome.versions_dirty = true;
                    if redo {
                        self.undo.push(self.b.clone());
                    } else {
                        self.redo.push(self.b.clone());
                    }
                    self.b = next;
                    if self
                        .reconciliation
                        .as_ref()
                        .is_some_and(|s| self.b.account(&s.account).is_err())
                    {
                        self.reconciliation = None;
                    }
                    self.notify(
                        if redo {
                            "Повторено"
                        } else {
                            "Отменено"
                        },
                        false,
                    );
                }
                Err(e) => {
                    if redo {
                        self.redo.push(next)
                    } else {
                        self.undo.push(next)
                    }
                    self.notify(e, true);
                }
            }
        }
    }
    pub(in crate::ui) fn save_preferences(&mut self) {
        self.preferences.last_budget = Some(self.store.path.clone());
        // UI tests must never write the user's global startup preferences.
        #[cfg(not(test))]
        let default = store::default_path();
        #[cfg(test)]
        let default = self.store.path.clone();
        if let Err(error) = self
            .preferences
            .save_with_startup(&self.store.path, &default)
        {
            self.notify(error, true);
        }
    }
}
