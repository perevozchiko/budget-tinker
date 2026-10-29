use super::*;
use budgettinker::reports::{self, Report, Selection, SpendingRow, SpendingScope};

#[derive(Clone, Debug)]
pub(super) struct Drill {
    scope: SpendingScope,
    label: String,
    month: Option<String>,
}

const RAIL: Color32 = Color32::from_rgb(69, 87, 95);

#[derive(Clone, Debug)]
pub(super) struct ReportAccounts {
    spending: Selection,
    net_worth: Selection,
}

impl Default for ReportAccounts {
    fn default() -> Self {
        Self {
            spending: Selection::BudgetAccounts,
            net_worth: Selection::All,
        }
    }
}

impl ReportAccounts {
    fn switch(&mut self, current: usize, next: usize, filter: &mut reports::Filter) {
        if current == 4 {
            self.net_worth = filter.accounts.clone();
        } else {
            self.spending = filter.accounts.clone();
        }
        filter.accounts = if next == 4 {
            self.net_worth.clone()
        } else {
            self.spending.clone()
        };
    }
}

use budgettinker::report_print::group_color;

fn report_month(month: &str) -> String {
    let names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let index = month[5..7].parse::<usize>().unwrap_or(1).clamp(1, 12) - 1;
    format!("{} {}", names[index], &month[..4])
}

const PERIODS: [&str; 8] = [
    "This Month",
    "Last Month",
    "Latest 3 Months",
    "Last 6 Months",
    "This Year",
    "All Dates",
    "Custom",
    "Last Year",
];
mod periods;
use periods::*;
mod selectors;
use selectors::*;
mod spending;
use spending::*;
mod matrix;
use matrix::*;
mod trends;
use trends::*;
mod net_worth;
use net_worth::*;

mod controls;
mod documents;
mod labels;
mod popover;
mod preferences;
mod timeframe;
mod view;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod matrix_tests;
