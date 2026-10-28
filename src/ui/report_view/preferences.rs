use super::*;

impl App {
    pub(in crate::ui) fn report_data(&self) -> Result<Report> {
        reports::calculate(&self.b, self.report, &self.report_filter)
    }

    pub(in crate::ui) fn restore_report_preferences(&mut self) {
        let saved = self.preferences.reports.clone().unwrap_or_default();
        self.report = if saved.report < reports::TITLES.len() {
            saved.report
        } else {
            0
        };
        self.report_period = if saved.period <= 6 { saved.period } else { 5 };
        self.report_filter = saved.filter;
        if self.report_period == 6 && self.report_filter.months().is_err() {
            self.report_period = 5;
        }
        self.report_accounts = ReportAccounts {
            spending: saved.spending_accounts,
            net_worth: saved.net_worth_accounts,
        };
        // Deleted IDs must not turn an explicit selection into All.
        retain_report_choices(&mut self.report_filter.category, |id| {
            id == budgettinker::model::UNCATEGORIZED_CATEGORY
                || self.b.categories.iter().any(|c| c.id == id)
        });
        retain_report_choices(&mut self.report_filter.payee, |payee| {
            self.b.transactions.iter().any(|t| {
                t.payee == payee
                    || t.splits
                        .iter()
                        .any(|s| self.b.split_payee_name(t, s) == payee)
            })
        });
        for selection in [
            &mut self.report_accounts.spending,
            &mut self.report_accounts.net_worth,
        ] {
            retain_report_choices(selection, |id| {
                self.b.accounts.iter().any(|account| account.id == id)
            });
        }
        self.report_filter.accounts = if self.report == 4 {
            self.report_accounts.net_worth.clone()
        } else {
            self.report_accounts.spending.clone()
        };
        self.report_presets();
    }

    pub(in crate::ui) fn persist_report_preferences(&mut self) {
        // Incomplete/invalid custom text stays editable but never replaces saved valid dates.
        if self.report_filter.months().is_err() {
            return;
        }
        self.report_accounts
            .switch(self.report, self.report, &mut self.report_filter);
        let saved = crate::preferences::ReportPreferences {
            report: self.report,
            period: self.report_period,
            filter: self.report_filter.clone(),
            spending_accounts: self.report_accounts.spending.clone(),
            net_worth_accounts: self.report_accounts.net_worth.clone(),
        };
        if self.preferences.reports.as_ref() != Some(&saved) {
            let mut preferences = self.preferences.clone();
            preferences.reports = Some(saved);
            match preferences.save(&self.store.path) {
                Ok(()) => self.preferences.reports = preferences.reports,
                Err(error) => self.notify(error, true),
            }
        }
    }

    pub(in crate::ui) fn report_presets(&mut self) {
        apply_report_period(
            &self.b,
            self.report_period,
            &mut self.report_filter,
            &today(),
        );
    }
}
