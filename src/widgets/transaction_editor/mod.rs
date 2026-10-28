use crate::classic::{self, Glyph};
use crate::i18n::{self, LocalizedUi as _};
use budgettinker::model::*;
use budgettinker::register::Repeat;
use eframe::egui::{self, Color32, FontId, Rect, Stroke, Vec2};

#[allow(clippy::too_many_arguments)]
pub fn category_combo(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    category: &mut String,
    income_next: &mut Option<bool>,
    date: &str,
    splits: bool,
    b: &Budget,
    width: f32,
) -> Option<crate::category_picker::Selection> {
    use crate::category_picker::{self, Choice};
    let month = if valid_date(date) { &date[..7] } else { "" };
    let selected = if splits {
        Choice::Split
    } else if let Some(next) = *income_next {
        Choice::Income(next)
    } else if category.is_empty() {
        Choice::Uncategorized
    } else {
        Choice::Category(category.clone())
    };
    let label = match &selected {
        Choice::Split => "Split (Multiple Categories)".into(),
        Choice::Income(next) if !month.is_empty() => {
            budgettinker::income::income_label(&month_shift(month, i32::from(*next)))
        }
        Choice::Category(id) => category_picker::category_label(ui.ctx(), b, id),
        _ => "Uncategorized".into(),
    };
    let result = category_picker::show(
        ui,
        id,
        b,
        category_picker::Options {
            selected,
            label,
            date,
            width,
            special: true,
        },
    );
    if let Some(result) = &result {
        match &result.choice {
            Choice::Uncategorized => {
                category.clear();
                *income_next = None;
            }
            Choice::Income(next) => {
                category.clear();
                *income_next = Some(*next);
            }
            Choice::Category(id) => {
                *category = id.clone();
                *income_next = None;
            }
            _ => {}
        }
    }
    result
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    #[default]
    None,
    Save,
    SaveAnother,
    Cancel,
}

#[derive(Clone, Default)]
struct SplitDetails {
    income_next: Option<bool>,
    payee: String,
    memo: String,
    transfer: Option<SplitTransfer>,
    transfer_to: Option<String>,
}
impl SplitDetails {
    fn from_part(part: &Split, date: &str) -> Self {
        Self {
            income_next: part.income_month.as_ref().map(|m| m.as_str() > &date[..7]),
            payee: part.payee.clone().unwrap_or_default(),
            memo: part.memo.clone(),
            transfer: part.transfer.clone(),
            transfer_to: part.transfer.as_ref().map(|x| x.account.clone()),
        }
    }
}

pub struct Draft {
    pub id: Option<String>,
    pub account: String,
    pub date: String,
    pub payee: String,
    pub category: String,
    pub income_next: Option<bool>,
    pub memo: String,
    pub outflow: String,
    pub inflow: String,
    pub cleared: bool,
    pub error: String,
    pub splits: Vec<(String, String)>,
    split_details: Vec<SplitDetails>,
    pub transfer_to: Option<String>,
    pub pending_category: Option<(crate::category_picker::NewCategory, Option<usize>)>,
    date_input: crate::date_input::State,
    focus: bool,
    complete_payee: bool,
    completed_account: String,
    automatic_amount: Option<(String, String)>,
}
mod draft;
mod editor;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
