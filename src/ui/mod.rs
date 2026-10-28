use crate::i18n::{self, LocalizedPainter as _, LocalizedResponse as _, LocalizedUi as _};
use crate::transaction_editor::{Action as RegisterAction, Draft as RegisterDraft};
use crate::{
    budget_editor::{Cell as BudgetCell, CellOptions as BudgetCellOptions, Editor as BudgetEditor},
    classic,
};
use budgettinker::register::{Command as RegisterCommand, Repeat};
use budgettinker::{
    model::*,
    store::{self, Store},
};
use eframe::egui::{self, Align, Color32, FontId, Frame, Layout, Margin, RichText, Stroke, Vec2};
use std::{collections::HashSet, path::Path};

mod bank_form;
mod budget_categories;
mod chrome;
mod forms;
mod legacy_import;
mod reconciliation;
mod register_footer;
mod report_view;
mod scheduled_panel;

const TEAL: Color32 = crate::palette::SIDEBAR;
const DARK: Color32 = Color32::from_rgb(54, 68, 77);
const GREEN: Color32 = Color32::from_rgb(124, 162, 31);
const INK: Color32 = crate::palette::TEXT;
const PALE: Color32 = crate::palette::ROW;
const LINE: Color32 = crate::palette::GRID;
const RED: Color32 = Color32::from_rgb(190, 68, 61);
const MONTHS: [&str; 12] = [
    "ЯНВАРЬ",
    "ФЕВРАЛЬ",
    "МАРТ",
    "АПРЕЛЬ",
    "МАЙ",
    "ИЮНЬ",
    "ИЮЛЬ",
    "АВГУСТ",
    "СЕНТЯБРЬ",
    "ОКТЯБРЬ",
    "НОЯБРЬ",
    "ДЕКАБРЬ",
];
fn month_name(ctx: &egui::Context, month: &str) -> String {
    let i = month[5..].parse::<usize>().unwrap_or(1) - 1;
    format!("{} {}", i18n::tr(ctx, MONTHS[i]), &month[..4])
}
fn text(ui: &mut egui::Ui, s: impl Into<String>, size: f32, color: Color32) {
    ui.l_label(RichText::new(s).size(size).color(color));
}
fn budget_row_fill(
    p: &egui::Painter,
    rect: egui::Rect,
    label: f32,
    column: f32,
    count: i32,
    color: Color32,
) {
    p.rect_filled(
        egui::Rect::from_min_size(
            rect.min + Vec2::new(1., 0.),
            Vec2::new(label - 2., rect.height()),
        ),
        0.,
        color,
    );
    for i in 0..count {
        p.rect_filled(
            egui::Rect::from_min_size(
                rect.min + Vec2::new(label + 9. + i as f32 * (column + 8.), 0.),
                Vec2::new(column - 2., rect.height()),
            ),
            0.,
            color,
        );
    }
}

fn category_hover_tooltip(response: &egui::Response, name: &str, note: Option<&str>) {
    let mut tooltip = egui::Tooltip::for_enabled(response).at_pointer().gap(8.);
    tooltip.popup = tooltip.popup.frame(
        Frame::new()
            .fill(Color32::from_rgb(255, 253, 190))
            .stroke(Stroke::new(1_f32, Color32::from_rgb(224, 219, 155)))
            .corner_radius(4)
            .inner_margin(Margin::symmetric(7, 5))
            .shadow(egui::epaint::Shadow {
                offset: [1, 2],
                blur: 5,
                spread: 0,
                color: Color32::from_black_alpha(70),
            }),
    );
    tooltip.show(|ui| {
        ui.set_max_width(300.);
        ui.spacing_mut().item_spacing.y = 4.;
        ui.l_label(
            RichText::new(name)
                .font(crate::palette::bold(12.))
                .color(DARK),
        );
        ui.l_label(
            RichText::new(
                note.unwrap_or("Нажмите, чтобы переименовать, скрыть или удалить эту категорию."),
            )
            .size(12.)
            .color(DARK),
        );
    });
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.l_label(label);
    ui.add(egui::TextEdit::singleline(value).desired_width(340.));
    ui.end_row();
}
fn currency_combo(ui: &mut egui::Ui, id: &str, value: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(i18n::tr(ui.ctx(), value.as_str()))
        .show_ui(ui, |ui| {
            for c in CURRENCIES {
                ui.l_selectable_value(value, (*c).into(), *c);
            }
        });
}
fn account_combo(ui: &mut egui::Ui, id: &str, value: &mut String, b: &Budget) {
    egui::ComboBox::from_id_salt(id)
        .width(330.)
        .selected_text(i18n::tr(
            ui.ctx(),
            b.account(value)
                .map(|a| a.name.clone())
                .unwrap_or_else(|_| "Выберите счёт".into()),
        ))
        .show_ui(ui, |ui| {
            for a in b.accounts.iter().filter(|a| !a.closed) {
                ui.l_selectable_value(value, a.id.clone(), a.name.clone());
            }
        });
}
fn category_combo(ui: &mut egui::Ui, id: &str, value: &mut String, b: &Budget) {
    egui::ComboBox::from_id_salt(id)
        .width(330.)
        .selected_text(i18n::tr(
            ui.ctx(),
            if value.is_empty() {
                "Доход / без категории".into()
            } else {
                b.category_name(&Some(value.clone()))
            },
        ))
        .show_ui(ui, |ui| {
            ui.l_selectable_value(value, String::new(), "Доход / без категории");
            for c in &b.categories {
                ui.l_selectable_value(
                    value,
                    c.id.clone(),
                    format!(
                        "{} / {}",
                        i18n::tr(ui.ctx(), &c.group),
                        i18n::tr(ui.ctx(), &c.name)
                    ),
                );
            }
        });
}
impl App {
    fn choose_file(&mut self, save: bool, extension: &str, ctx: &egui::Context) -> Option<String> {
        let prompt = i18n::tr(
            ctx,
            if save {
                "Сохранить файл"
            } else {
                "Выберите файл для импорта"
            },
        );
        match crate::desktop::choose_file(save, extension, &prompt) {
            Ok(path) => path,
            Err(error) => {
                self.notify(error, true);
                None
            }
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Page {
    Budget,
    Accounts,
    Reports,
}
#[derive(Clone)]
enum DialogKind {
    Register {
        ids: HashSet<String>,
        command: RegisterCommand,
    },
    Account,
    #[expect(
        dead_code,
        reason = "Legacy transaction dialog remains available for compatibility"
    )]
    Transaction(Option<String>),
    Transfer,
    Category,
    NewGroup,
    EditCategory(String),
    EditGroup(String),
    Overspending {
        month: String,
        category: String,
    },
    Reconcile(String),
    Import,
    ImportLegacyBudget,
    Restore,
    AccountSettings(String),
    AdjustBalance(String),
    Note(String),
}
struct Dialog {
    bank_preview: Option<Box<budgettinker::bank::Preview>>,
    legacy_preview: Option<Box<budgettinker::legacy_budget::Preview>>,
    kind: DialogKind,
    fields: Vec<String>,
    check: bool,
    second: bool,
    error: String,
    splits: Vec<(String, String)>,
    focus: bool,
    parent: Option<Box<Dialog>>,
    anchor: Option<egui::Pos2>,
}
impl Dialog {
    fn new(kind: DialogKind) -> Self {
        Self {
            bank_preview: None,
            legacy_preview: None,
            kind,
            fields: vec![String::new(); 10],
            check: false,
            second: false,
            error: String::new(),
            splits: vec![],
            focus: true,
            parent: None,
            anchor: None,
        }
    }
}
pub struct App {
    store: Store,
    b: Budget,
    page: Page,
    month: String,
    account: Option<String>,
    search: String,
    register_timeframe: crate::register_timeframe::Timeframe,
    register_ascending: bool,
    register_sort: usize,
    register_scheduled_open: bool,
    scheduled_selected: HashSet<String>,
    scheduled_editor: Option<(RegisterDraft, Repeat)>,
    reconciliation: Option<reconciliation::Session>,
    dialog: Option<Dialog>,
    notice: String,
    is_error: bool,
    undo: Vec<Budget>,
    redo: Vec<Budget>,
    selected: HashSet<String>,
    report: usize,
    report_filter: budgettinker::reports::Filter,
    report_accounts: report_view::ReportAccounts,
    report_period: usize,
    report_selected_month: Option<String>,
    report_navigation: Vec<report_view::Drill>,
    collapsed: HashSet<String>,
    dragging_category: Option<String>,
    budget_editor: BudgetEditor,
    transaction_editor: Option<RegisterDraft>,
    preferences: crate::preferences::Preferences,
    fabric: egui::TextureHandle,
    window_title: String,
    native_menu: crate::native_menu::NativeMenu,
    chrome: chrome::Chrome,
}

mod accounts;
mod application;
mod budget;
mod dialogs;
mod register_commands;
mod sidebar;
mod update;

#[cfg(test)]
#[path = "tests/credit.rs"]
mod credit_tests;
#[cfg(test)]
#[path = "tests/legacy_import.rs"]
mod legacy_import_tests;

fn repeat_combo(ui: &mut egui::Ui, id: &str, value: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(Repeat::from_code(value).label())
        .show_ui(ui, |ui| {
            for repeat in Repeat::ALL {
                ui.selectable_value(value, repeat.code().to_owned(), repeat.label());
            }
        });
}
