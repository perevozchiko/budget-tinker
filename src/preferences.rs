use crate::i18n::Language;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub language: Language,
    pub budget_collapsed: bool,
    pub budget_summary_collapsed: bool,
    pub sidebar_compact: bool,
    pub off_budget_collapsed: bool,
    pub closed_collapsed: bool,
    pub open_recent: bool,
    pub last_budget: Option<PathBuf>,
    pub diagnostics: bool,
    pub autocomplete_category: bool,
    pub autocomplete_memo: bool,
    pub autocomplete_amount: bool,
    pub hide_reconciled: bool,
    /// Height of the scheduled panel in pixels; restored on startup.
    #[serde(default = "default_scheduled_panel_height")]
    pub scheduled_panel_height: f32,
    /// Report choices belong to this budget, never to the global startup pointer.
    pub reports: Option<ReportPreferences>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportPreferences {
    pub report: usize,
    pub period: usize,
    pub filter: budgettinker::reports::Filter,
    pub spending_accounts: budgettinker::model::Selection,
    pub net_worth_accounts: budgettinker::model::Selection,
}

impl Default for ReportPreferences {
    fn default() -> Self {
        Self {
            report: 0,
            period: 5, // All Dates for a budget with no saved report choices.
            filter: budgettinker::reports::Filter::default(),
            spending_accounts: budgettinker::model::Selection::BudgetAccounts,
            net_worth_accounts: budgettinker::model::Selection::All,
        }
    }
}
fn default_scheduled_panel_height() -> f32 {
    280.
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: Language::En,
            budget_collapsed: false,
            budget_summary_collapsed: false,
            sidebar_compact: false,
            off_budget_collapsed: false,
            closed_collapsed: false,
            open_recent: true,
            last_budget: None,
            diagnostics: false,
            autocomplete_category: true,
            autocomplete_memo: false,
            autocomplete_amount: false,
            hide_reconciled: false,
            scheduled_panel_height: default_scheduled_panel_height(),
            reports: None,
        }
    }
}
impl Preferences {
    /// Clamp only the displayed height, keeping the user's preference for larger windows.
    pub fn scheduled_height(&self, available: f32) -> f32 {
        let max = (available - 64. - 14. - 180.).max(0.);
        let min = 140_f32.min(max);
        let preferred = if self.scheduled_panel_height.is_finite() {
            self.scheduled_panel_height
        } else {
            default_scheduled_panel_height()
        };
        preferred.clamp(min, max)
    }

    pub fn startup_path(default: &Path) -> PathBuf {
        let prefs = Self::load(default).unwrap_or_default();
        if prefs.open_recent
            && let Some(path) = prefs.last_budget
            && path.is_file()
        {
            return path;
        }
        default.to_owned()
    }
    pub fn path(database: &Path) -> PathBuf {
        database.with_extension("preferences.json")
    }
    pub fn load(database: &Path) -> Result<Self, String> {
        match fs::read(Self::path(database)) {
            Ok(bytes) => {
                let mut preferences: Self =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                preferences.language = Language::En;
                Ok(preferences)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    /// Update the startup pointer without copying another budget's report choices.
    pub fn save_with_startup(&self, database: &Path, default: &Path) -> Result<(), String> {
        self.save(database)?;
        if database != default {
            let mut startup = self.clone();
            startup.reports = Self::load(default)?.reports;
            startup.save(default)?;
        }
        Ok(())
    }

    pub fn save(&self, database: &Path) -> Result<(), String> {
        let path = Self::path(database);
        let temp = path.with_extension("json.tmp");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(temp, path).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_preferences_and_global_startup_keep_budget_report_choices_separate() {
        assert!(
            serde_json::from_str::<Preferences>("{}")
                .unwrap()
                .reports
                .is_none()
        );
        let partial: Preferences = serde_json::from_str(r#"{"reports":{"period":6}}"#).unwrap();
        assert_eq!(partial.reports.unwrap().period, 6);
        let root = std::env::temp_dir().join(format!("report-prefs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let default = root.join("default.sqlite3");
        let other = root.join("other.sqlite3");
        let first = Preferences {
            reports: Some(ReportPreferences {
                period: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        first.save(&default).unwrap();
        let second = Preferences {
            last_budget: Some(other.clone()),
            reports: Some(ReportPreferences {
                report: 4,
                period: 6,
                ..Default::default()
            }),
            ..Default::default()
        };
        second.save_with_startup(&other, &default).unwrap();
        let startup = Preferences::load(&default).unwrap();
        assert_eq!(startup.last_budget, Some(other.clone()));
        assert_eq!(startup.reports, first.reports);
        assert_eq!(Preferences::load(&other).unwrap().reports, second.reports);
        assert_eq!(Preferences::startup_path(&other), other); // no SQLite file is needed for settings
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scheduled_height_loads_old_preferences_and_preserves_user_preference() {
        let old: Preferences = serde_json::from_str("{}").unwrap();
        assert!(!old.budget_summary_collapsed);
        assert_eq!(old.scheduled_panel_height, 280.);
        let mut prefs = Preferences {
            scheduled_panel_height: 400.,
            ..Default::default()
        };
        assert_eq!(prefs.scheduled_height(900.), 400.);
        assert_eq!(prefs.scheduled_height(500.), 242.);
        assert_eq!(prefs.scheduled_panel_height, 400.);
        for height in [0., 100., 200., 300.] {
            assert!(prefs.scheduled_height(height).is_finite());
        }
        prefs.scheduled_panel_height = f32::NAN;
        assert!(prefs.scheduled_height(900.).is_finite());
        let path =
            std::env::temp_dir().join(format!("schedule-height-{}.sqlite3", uuid::Uuid::new_v4()));
        prefs.scheduled_panel_height = 350.;
        prefs.save(&path).unwrap();
        assert_eq!(
            Preferences::load(&path).unwrap().scheduled_panel_height,
            350.
        );
        fs::remove_file(Preferences::path(&path)).unwrap();
    }
    #[test]
    fn persists_language_and_account_disclosure_independently_of_budget() {
        let path = std::env::temp_dir().join(format!(
            "budgettinker-prefs-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let mut p = Preferences::load(&path).unwrap();
        assert_eq!(p.language, Language::En);
        p.language = Language::Ru;
        p.budget_collapsed = true;
        p.sidebar_compact = true;
        p.save(&path).unwrap();
        let p = Preferences::load(&path).unwrap();
        assert_eq!(p.language, Language::En);
        assert!(p.budget_collapsed);
        assert!(p.sidebar_compact);
        assert!(!path.exists());
        fs::remove_file(Preferences::path(&path)).unwrap();
    }
}
