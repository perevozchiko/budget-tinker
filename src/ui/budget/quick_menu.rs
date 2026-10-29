use super::*;
use budgettinker::model::QuickBudget;

impl App {
    pub(super) fn month_quick_menu(&mut self, ui: &mut egui::Ui, month: &str) {
        let count = self.b.average_category_outflow(month, "").1;
        let year_label = if count < 12 {
            format!(
                "Average outflows for the last year (only {count} {} available)",
                if count == 1 { "month" } else { "months" }
            )
        } else {
            "Average outflows for the last year".into()
        };
        ui.label("Fill in all empty budget cells with:");
        for (action, label) in [
            (QuickBudget::PreviousBudget, "Budget values used last month"),
            (QuickBudget::PreviousOutflows, "Outflows last month"),
            (
                QuickBudget::AverageThreeMonths,
                "Average outflows for the last 3 months",
            ),
            (QuickBudget::AverageYear, year_label.as_str()),
            (
                QuickBudget::Scheduled,
                "Total scheduled outflows this month",
            ),
            (QuickBudget::ZeroBalance, "Balance to 0.00"),
            (
                QuickBudget::ZeroAllocations,
                "0.00 (Zero all budgeted amounts for this month)",
            ),
        ] {
            if action == QuickBudget::ZeroAllocations {
                ui.separator();
                ui.label("Fill in all budget cells with:");
            }
            let response = ui.button(label);
            #[cfg(test)]
            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    egui::Id::new(("month_quick_option", month, format!("{action:?}"))),
                    response.rect,
                )
            });
            if response.clicked() {
                self.change(|b| b.quick_budget_month(month, action));
                ui.close();
            }
        }
    }
}

#[cfg(test)]
mod tests;
