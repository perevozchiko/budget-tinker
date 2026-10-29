use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickBudget {
    PreviousBudget,
    PreviousOutflows,
    AverageThreeMonths,
    AverageYear,
    Scheduled,
    ZeroBalance,
    ZeroAllocations,
}

impl Budget {
    /// Fill zero-valued cells; only ZeroAllocations replaces existing allocations.
    /// Compute every value before mutating so a missing rate cannot partially apply.
    pub fn quick_budget_month(&mut self, month: &str, action: QuickBudget) -> Result<()> {
        if !valid_month(month) {
            return Err("Invalid month".into());
        }
        let previous = month_shift(month, -1);
        let values: Result<Vec<_>> = self
            .categories
            .iter()
            .filter(|c| action == QuickBudget::ZeroAllocations || self.allocated(month, &c.id) == 0)
            .map(|c| {
                let value = match action {
                    QuickBudget::PreviousBudget => self.allocated(&previous, &c.id),
                    QuickBudget::PreviousOutflows => self
                        .activity(&previous, &c.id)
                        .min(0)
                        .checked_neg()
                        .ok_or("Budget amount is too large")?,
                    QuickBudget::AverageThreeMonths => {
                        self.average_category_outflow_over(month, &c.id, 3).0
                    }
                    QuickBudget::AverageYear => self.average_category_outflow(month, &c.id).0,
                    QuickBudget::Scheduled => self.scheduled_category_outflows(month, &c.id)?,
                    QuickBudget::ZeroBalance => self
                        .allocated(month, &c.id)
                        .checked_sub(self.category_balance(month, &c.id))
                        .ok_or("Budget amount is too large")?,
                    QuickBudget::ZeroAllocations => 0,
                };
                Ok((c.id.clone(), value))
            })
            .collect();
        for (category, value) in values? {
            self.allocate(month, &category, value)?;
        }
        Ok(())
    }
}
