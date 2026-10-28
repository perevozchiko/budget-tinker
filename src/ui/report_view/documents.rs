use super::*;

impl App {
    pub(in crate::ui) fn export_current_report(&mut self, ctx: &egui::Context) {
        let result = self.report_data().and_then(|report| {
            if let Some(current) = self.report_navigation.last() {
                report.spending_csv(&current.scope, current.month.as_deref(), &self.b.base)
            } else {
                report.csv(&self.b.base, |s| i18n::tr(ctx, s))
            }
        });
        match result {
            Ok(bytes) => {
                if let Some(path) = self.choose_file(true, "csv", ctx) {
                    match std::fs::write(path, bytes) {
                        Ok(()) => self.notify("Report exported.", false),
                        Err(error) => self.notify(error.to_string(), true),
                    }
                }
            }
            Err(error) => self.notify(error, true),
        }
    }

    pub(in crate::ui) fn report_print_document(
        &self,
    ) -> Result<budgettinker::report_print::Document> {
        let report = self.report_data()?;
        let mut metadata = vec![format!(
            "Accounts: {}",
            self.selection_label(&self.report_filter.accounts, "accounts")
        )];
        if self.report != 4 {
            metadata.push(format!(
                "Categories: {}",
                self.selection_display_names(&self.report_filter.category, "categories")
            ));
            metadata.push(format!(
                "Payees: {}",
                self.selection_display_names(&self.report_filter.payee, "payees")
            ));
        }
        let detail = self
            .report_navigation
            .last()
            .map(|d| budgettinker::report_print::Detail {
                scope: &d.scope,
                month: d.month.as_deref(),
                label: &d.label,
            });
        Ok(budgettinker::report_print::layout(
            &report,
            &self.b.name,
            &report_period_caption(&self.report_filter, &today()),
            &metadata,
            &self.b.base,
            detail,
            |value| i18n::money(value, &self.b.base),
        ))
    }
}
