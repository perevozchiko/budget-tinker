use super::*;

impl App {
    pub(in crate::ui) fn screen(&mut self, screen: Screen) {
        self.dialog = None;
        self.chrome.overlay = None;
        self.chrome.screen = Some(screen);
        self.chrome.print_preview = false;
        self.chrome.name = if screen == Screen::New {
            i18n::tr_language(self.preferences.language, "Мой бюджет")
        } else {
            self.b.name.clone()
        };
        self.chrome.currency = self.b.base.clone();
        self.chrome.error.clear();
    }
    pub(in crate::ui) fn switch_budget(&mut self, path: &Path, new: Option<Budget>) -> Result<()> {
        if new.is_some() && path.exists() {
            return Err("Выберите новое имя файла: этот бюджет уже существует".into());
        }
        let existing = if new.is_none() {
            Some(store::read_budget(path)?)
        } else {
            None
        };
        let mut store = Store::open(path)?;
        let budget = match new {
            Some(budget) => {
                store.save(&budget)?;
                budget
            }
            None => existing.expect("read-only budget validation"),
        };
        self.persist_report_preferences();
        self.store = store;
        self.b = budget;
        self.preferences.reports = crate::preferences::Preferences::load(path)
            .unwrap_or_default()
            .reports;
        self.restore_report_preferences();
        self.report_navigation.clear();
        self.report_selected_month = None;
        self.account = None;
        self.reconciliation = None;
        self.selected.clear();
        self.search.clear();
        self.register_scheduled_open = false;
        self.scheduled_selected.clear();
        self.scheduled_editor = None;
        self.collapsed.clear();
        self.dragging_category = None;
        self.undo.clear();
        self.redo.clear();
        self.budget_editor = BudgetEditor::default();
        self.transaction_editor = None;
        self.month = current_month();
        self.page = Page::Budget;
        self.preferences.last_budget = Some(path.to_owned());
        self.save_preferences();
        self.chrome.versions_dirty = true;
        self.notify("Бюджет открыт", false);
        Ok(())
    }
}
