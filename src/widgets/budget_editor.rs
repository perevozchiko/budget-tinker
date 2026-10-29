use crate::i18n::LocalizedResponse as _;
use budgettinker::model::{Budget, Result, amount, month_shift, plain};
use eframe::egui::{self, Align, Color32, FontId, Rect, Stroke, Vec2};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub month: String,
    pub category: String,
}

pub struct CellOptions {
    pub quick: [i64; 4],
    pub average_months: usize,
    pub scheduled: Result<i64>,
    pub row_hovered: bool,
}
impl Default for CellOptions {
    fn default() -> Self {
        Self {
            quick: [0; 4],
            average_months: 0,
            scheduled: Ok(0),
            row_hovered: false,
        }
    }
}
impl CellOptions {
    pub fn for_budget(budget: &Budget, cell: &Cell, row_hovered: bool, editing: bool) -> Self {
        let previous = month_shift(&cell.month, -1);
        let (average, average_months) = if editing {
            budget.average_category_outflow(&cell.month, &cell.category)
        } else {
            (0, 0)
        };
        let scheduled = if editing {
            budget.scheduled_category_outflows(&cell.month, &cell.category)
        } else {
            Ok(0)
        };
        let allocated = budget.allocated(&cell.month, &cell.category);
        Self {
            quick: [
                budget.allocated(&previous, &cell.category),
                (-budget.activity(&previous, &cell.category)).max(0),
                average,
                allocated - budget.category_balance(&cell.month, &cell.category),
            ],
            average_months,
            scheduled,
            row_hovered,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Harness {
        ctx: egui::Context,
        editor: Editor,
        values: Vec<(Cell, i64)>,
        writes: usize,
        fail: bool,
        time: f64,
        budget: Option<Budget>,
    }
    impl Harness {
        fn new() -> Self {
            let mut h = Self {
                ctx: egui::Context::default(),
                editor: Editor::default(),
                values: vec![
                    (
                        Cell {
                            month: "2026-10".into(),
                            category: "rent".into(),
                        },
                        10000,
                    ),
                    (
                        Cell {
                            month: "2026-10".into(),
                            category: "food".into(),
                        },
                        20000,
                    ),
                ],
                writes: 0,
                fail: false,
                time: 0.,
                budget: None,
            };
            h.frame(vec![]);
            h.frame(vec![]);
            h
        }
        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 0.02;
            let ctx = self.ctx.clone();
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800., 600.))),
                    time: Some(self.time),
                    events,
                    focused: true,
                    ..Default::default()
                },
                |ctx| {
                    self.editor.begin(ctx);
                    egui::CentralPanel::default().show(ctx, |ui| {
                        for (i, (cell, value)) in self.values.iter().enumerate() {
                            let options = self.budget.as_ref().map_or_else(
                                || CellOptions {
                                    quick: [12345, 6789, 4000, -500],
                                    ..Default::default()
                                },
                                |budget| {
                                    CellOptions::for_budget(
                                        budget,
                                        cell,
                                        false,
                                        self.editor.is_editing(cell),
                                    )
                                },
                            );
                            self.editor.show(
                                ui,
                                Rect::from_min_size(
                                    egui::pos2(20., 30. + i as f32 * 30.),
                                    Vec2::new(120., 25.),
                                ),
                                cell.clone(),
                                *value,
                                "EUR",
                                options,
                            );
                        }
                    });
                    let order = self.values.clone();
                    self.editor.finish(ctx, "EUR", &order, |cell, value| {
                        if self.fail {
                            return Err("disk full".into());
                        }
                        if let Some(budget) = &mut self.budget {
                            budget.allocate(&cell.month, &cell.category, value)?;
                        }
                        self.values.iter_mut().find(|(c, _)| c == cell).unwrap().1 = value;
                        self.writes += 1;
                        Ok(())
                    });
                },
            );
        }
        fn click(&mut self, x: f32, y: f32) {
            for pressed in [true, false] {
                self.frame(vec![
                    egui::Event::PointerMoved(egui::pos2(x, y)),
                    egui::Event::PointerButton {
                        pos: egui::pos2(x, y),
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
            }
            self.frame(vec![]);
        }
        fn key(&mut self, key: egui::Key, shift: bool) {
            self.frame(vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers: if shift {
                    egui::Modifiers::SHIFT
                } else {
                    egui::Modifiers::NONE
                },
            }]);
            self.frame(vec![]);
        }
    }
    #[test]
    fn clicking_selects_existing_value_and_enter_saves_then_moves_down() {
        let mut h = Harness::new();
        h.click(80., 40.);
        h.frame(vec![egui::Event::Text("12.34".into())]);
        h.key(egui::Key::Enter, false);
        assert_eq!(h.values[0].1, 1234);
        assert_eq!(h.writes, 1);
        assert!(h.editor.is_editing(&h.values[1].0));
    }
    #[test]
    fn tab_and_shift_tab_follow_categories() {
        let mut h = Harness::new();
        h.click(80., 40.);
        h.key(egui::Key::Tab, false);
        assert!(h.editor.is_editing(&h.values[1].0));
        h.key(egui::Key::Tab, true);
        assert!(h.editor.is_editing(&h.values[0].0));
        assert_eq!(h.writes, 0);
    }
    #[test]
    fn escape_discards_edit_without_writing() {
        let mut h = Harness::new();
        h.click(80., 40.);
        h.frame(vec![egui::Event::Text("99".into())]);
        h.key(egui::Key::Escape, false);
        assert_eq!(h.values[0].1, 10000);
        assert!(h.editor.draft.is_none());
        assert_eq!(h.writes, 0);
    }
    #[test]
    fn invalid_input_keeps_draft_and_original_amount() {
        let mut h = Harness::new();
        h.click(80., 40.);
        h.frame(vec![egui::Event::Text("oops".into())]);
        h.key(egui::Key::Enter, false);
        assert_eq!(h.values[0].1, 10000);
        assert!(h.editor.is_editing(&h.values[0].0));
        assert!(!h.editor.draft.as_ref().unwrap().error.is_empty());
        assert_eq!(h.writes, 0);
    }
    #[test]
    fn failed_persistence_keeps_original_and_editable_draft() {
        let mut h = Harness::new();
        h.click(80., 40.);
        h.frame(vec![egui::Event::Text("17".into())]);
        h.fail = true;
        h.key(egui::Key::Enter, false);
        assert_eq!(h.values[0].1, 10000);
        assert_eq!(h.editor.draft.as_ref().unwrap().text, "17");
        assert!(h.editor.is_editing(&h.values[0].0));
    }
    #[test]
    fn clicking_another_cell_saves_once() {
        let mut h = Harness::new();
        h.click(80., 40.);
        h.frame(vec![egui::Event::Text("23".into())]);
        h.click(80., 70.);
        assert_eq!(h.values[0].1, 2300);
        assert_eq!(h.writes, 1);
        assert!(h.editor.is_editing(&h.values[1].0));
    }
    #[test]
    fn quick_budget_arrow_keeps_editor_open_on_press_and_release() {
        let mut h = Harness::new();
        h.click(80., 40.);
        let pos = egui::pos2(150., 40.);
        h.frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        assert!(h.editor.is_editing(&h.values[0].0));
        h.frame(vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert!(h.editor.is_editing(&h.values[0].0));
        assert!(egui::Popup::is_any_open(&h.ctx));
        h.frame(vec![]);
        let rect: Rect = h.ctx.data(|d| {
            d.get_temp(egui::Id::new((
                "quick_option",
                "Amount budgeted last month",
            )))
            .unwrap()
        });
        h.click(rect.center().x, rect.center().y);
        assert_eq!(h.values[0].1, 12345);
        assert_eq!(h.writes, 1);
    }
    #[test]
    fn quick_budget_options_apply_their_values() {
        for (label, expected) in [
            ("Amount budgeted last month", 12345),
            ("Outflows last month", 6789),
            (
                "Average outflows for last year (only 0 months available)",
                4000,
            ),
            ("Total scheduled outflows this month", 0),
            ("Balance to 0.00", -500),
        ] {
            let mut h = Harness::new();
            h.click(80., 40.);
            h.click(150., 40.);
            let rect: Rect = h
                .ctx
                .data(|d| d.get_temp(egui::Id::new(("quick_option", label))).unwrap());
            h.click(rect.center().x, rect.center().y);
            assert_eq!(h.values[0].1, expected, "{label}");
            assert_eq!(h.writes, 1);
        }
    }
    #[test]
    fn quick_budget_choices_calculate_and_save_against_real_budget() {
        use budgettinker::model::{Scheduled, Transaction, id};
        use budgettinker::register::Repeat;
        let mut budget = Budget::new("EUR");
        let account = budget
            .add_account("Cash", "EUR", true, 100_000, "2026-01-01")
            .unwrap();
        let category = budget.categories[0].id.clone();
        let transaction = |date: &str, value: i64| Transaction {
            id: id(),
            account: account.clone(),
            date: date.into(),
            payee: "Shop".into(),
            memo: String::new(),
            check_number: None,
            amount: value,
            base_amount: 0,
            snapshot: None,
            category: Some(category.clone()),
            cleared: false,
            flag: None,
            income_month: None,
            reconciled: false,
            transfer: None,
            splits: vec![],
        };
        for (date, value) in [
            ("2026-01-05", -5_000),
            ("2026-02-05", -10_000),
            ("2026-03-05", -2_000),
        ] {
            budget.put_transaction(transaction(date, value)).unwrap();
        }
        for (month, value) in [
            ("2026-01", 10_000),
            ("2026-02", 20_000),
            ("2026-03", 10_000),
        ] {
            budget.allocate(month, &category, value).unwrap();
        }
        let scheduled = transaction("2026-03-20", -3_000);
        budget.scheduled.push(Scheduled {
            id: id(),
            next_date: scheduled.date.clone(),
            transaction: scheduled,
            monthly: false,
            repeat: Some(Repeat::Never),
            anchor_day: Some(20),
        });
        for (label, expected) in [
            ("Amount budgeted last month", 20_000),
            ("Outflows last month", 10_000),
            (
                "Average outflows for last year (only 2 months available)",
                7_500,
            ),
            ("Total scheduled outflows this month", 3_000),
            ("Balance to 0.00", -13_000),
        ] {
            let mut h = Harness::new();
            h.values[0] = (
                Cell {
                    month: "2026-03".into(),
                    category: category.clone(),
                },
                10_000,
            );
            h.budget = Some(budget.clone());
            h.frame(vec![]);
            h.frame(vec![]);
            h.click(80., 40.);
            h.click(150., 40.);
            let rect: Rect = h.ctx.data(|d| {
                d.get_temp(egui::Id::new(("quick_option", label)))
                    .unwrap_or_else(|| panic!("missing {label}"))
            });
            h.click(rect.center().x, rect.center().y);
            let actual = h.budget.as_ref().unwrap();
            assert_eq!(actual.allocated("2026-03", &category), expected, "{label}");
            if label == "Balance to 0.00" {
                assert_eq!(actual.category_balance("2026-03", &category), 0);
            }
            assert_eq!(h.writes, usize::from(expected != 10_000));
        }
    }
    #[test]
    fn unavailable_scheduled_amount_cannot_change_the_budget() {
        use budgettinker::model::{Scheduled, Transaction, id};
        use budgettinker::register::Repeat;
        let mut budget = Budget::new("EUR");
        let account = budget
            .add_account("Dollar", "USD", true, 0, "2026-01-01")
            .unwrap();
        let category = budget.categories[0].id.clone();
        let transaction = Transaction {
            id: id(),
            account,
            date: "2026-03-05".into(),
            payee: "Shop".into(),
            memo: String::new(),
            check_number: None,
            amount: -1000,
            base_amount: 0,
            snapshot: None,
            category: Some(category.clone()),
            cleared: false,
            flag: None,
            income_month: None,
            reconciled: false,
            transfer: None,
            splits: vec![],
        };
        budget.scheduled.push(Scheduled {
            id: id(),
            next_date: transaction.date.clone(),
            transaction,
            monthly: false,
            repeat: Some(Repeat::Never),
            anchor_day: Some(5),
        });
        budget.allocate("2026-03", &category, 10_000).unwrap();
        let mut h = Harness::new();
        h.values[0] = (
            Cell {
                month: "2026-03".into(),
                category: category.clone(),
            },
            10_000,
        );
        h.budget = Some(budget);
        h.frame(vec![]);
        h.frame(vec![]);
        h.click(80., 40.);
        h.click(150., 40.);
        let label = "Total scheduled outflows this month";
        let rect: Rect = h
            .ctx
            .data(|d| d.get_temp(egui::Id::new(("quick_option", label))).unwrap());
        h.click(rect.center().x, rect.center().y);
        assert_eq!(
            h.budget.as_ref().unwrap().allocated("2026-03", &category),
            10_000
        );
        assert_eq!(h.writes, 0);
    }
    #[test]
    fn calculator_buttons_apply_the_result() {
        let mut h = Harness::new();
        h.click(80., 40.);
        h.click(150., 40.);
        let rect: Rect = h
            .ctx
            .data(|d| d.get_temp(egui::Id::new("show_calculator")).unwrap());
        h.click(rect.center().x, rect.center().y);
        for label in ["C", "7", "+", "8", "OK"] {
            let rect: Rect = h.ctx.data(|d| {
                d.get_temp(egui::Id::new(("calculator_button", label)))
                    .unwrap()
            });
            h.click(rect.center().x, rect.center().y);
        }
        assert_eq!(h.values[0].1, 1500);
        assert_eq!(h.writes, 1);
    }
    #[test]
    fn calculator_ok_saves_precise_compound_results() {
        for (buttons, expected) in [
            (vec!["0", ".", "0", "1", "/", "3", "×", "3", "0", "0"], 100),
            (vec!["1", "0", "0", "×", "1", ".", "0", "7", "5"], 10750),
        ] {
            let mut h = Harness::new();
            h.click(80., 40.);
            h.click(150., 40.);
            let rect: Rect = h
                .ctx
                .data(|d| d.get_temp(egui::Id::new("show_calculator")).unwrap());
            h.click(rect.center().x, rect.center().y);
            for label in std::iter::once("C")
                .chain(buttons)
                .chain(std::iter::once("OK"))
            {
                let rect: Rect = h.ctx.data(|d| {
                    d.get_temp(egui::Id::new(("calculator_button", label)))
                        .unwrap()
                });
                h.click(rect.center().x, rect.center().y);
            }
            assert_eq!(h.values[0].1, expected);
            assert_eq!(h.writes, 1);
        }
    }
    #[test]
    fn calculator_cancel_does_not_save_and_sign_and_backspace_work() {
        let mut h = Harness::new();
        h.click(80., 40.);
        h.click(150., 40.);
        let rect: Rect = h
            .ctx
            .data(|d| d.get_temp(egui::Id::new("show_calculator")).unwrap());
        h.click(rect.center().x, rect.center().y);
        for label in ["C", "7", "8", "←", "+/-"] {
            let rect: Rect = h.ctx.data(|d| {
                d.get_temp(egui::Id::new(("calculator_button", label)))
                    .unwrap()
            });
            h.click(rect.center().x, rect.center().y);
        }
        assert_eq!(
            h.editor
                .draft
                .as_ref()
                .unwrap()
                .calculator
                .as_ref()
                .unwrap()
                .expression,
            "-(7)"
        );
        let rect: Rect = h.ctx.data(|d| {
            d.get_temp(egui::Id::new(("calculator_button", "Cancel")))
                .unwrap()
        });
        h.click(rect.center().x, rect.center().y);
        assert_eq!(h.values[0].1, 10000);
        assert_eq!(h.writes, 0);
        assert!(h.editor.is_editing(&h.values[0].0));
    }
    #[test]
    fn enter_on_last_cell_never_restores_stale_value() {
        let mut h = Harness::new();
        h.click(80., 70.);
        h.frame(vec![egui::Event::Text("42".into())]);
        h.key(egui::Key::Enter, false);
        h.key(egui::Key::Enter, false);
        assert_eq!(h.values[1].1, 4200);
        assert_eq!(h.writes, 1);
    }
}
fn quick_menu_row(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    enabled: bool,
    selected: bool,
    calculator_icon: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 25.), egui::Sense::click());
    let highlighted = enabled && (response.hovered() || selected);
    if highlighted {
        crate::classic::gradient(
            ui.painter(),
            rect,
            0.,
            Color32::from_rgb(71, 137, 255),
            Color32::from_rgb(13, 84, 245),
        );
    }
    let color = if !enabled {
        Color32::GRAY
    } else if highlighted {
        Color32::WHITE
    } else {
        Color32::from_gray(25)
    };
    let icon_space = if calculator_icon { 21. } else { 0. };
    ui.painter().text(
        rect.right_center() - Vec2::new(8. + icon_space, 0.),
        egui::Align2::RIGHT_CENTER,
        text,
        FontId::proportional(14.),
        color,
    );
    if calculator_icon {
        let icon = Rect::from_center_size(
            rect.right_center() - Vec2::new(15., 0.),
            Vec2::new(13., 16.),
        );
        ui.painter().rect_filled(icon, 0., Color32::from_gray(210));
        ui.painter().rect_stroke(
            icon,
            0.,
            Stroke::new(1_f32, Color32::from_gray(105)),
            egui::StrokeKind::Inside,
        );
        ui.painter().rect_filled(
            Rect::from_min_size(icon.min + Vec2::new(2., 2.), Vec2::new(9., 3.)),
            0.,
            Color32::from_rgb(95, 156, 202),
        );
        for y in 0..2 {
            for x in 0..3 {
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        icon.min + Vec2::new(2. + x as f32 * 3., 7. + y as f32 * 3.),
                        Vec2::splat(2.),
                    ),
                    0.,
                    Color32::from_gray(110),
                );
            }
        }
    }
    response
}

fn calculator_button(ui: &mut egui::Ui, label: &str, size: Vec2) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let (top, bottom) = if response.is_pointer_button_down_on() {
        (Color32::from_gray(195), Color32::from_gray(235))
    } else if response.hovered() {
        (Color32::WHITE, Color32::from_gray(222))
    } else {
        (Color32::from_gray(250), Color32::from_gray(226))
    };
    crate::classic::gradient(ui.painter(), rect, 4., top, bottom);
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        4.,
        Stroke::new(1_f32, Color32::from_gray(130)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.),
        Color32::from_rgb(11, 49, 57),
    );
    response
}

#[derive(Clone)]
struct Target {
    cell: Cell,
    value: i64,
}
struct Draft {
    target: Target,
    text: String,
    error: String,
    focus: bool,
    calculator: Option<Calculator>,
}
struct Calculator {
    expression: String,
    error: String,
    replace: bool,
}
#[derive(Default)]
pub struct Editor {
    draft: Option<Draft>,
    pending: Option<Target>,
    seen: bool,
    blur: bool,
    finish: bool,
    direction: i32,
    cancel: bool,
}
impl Editor {
    pub fn begin(&mut self, ctx: &egui::Context) {
        self.seen = false;
        self.blur = false;
        self.finish = false;
        self.direction = 0;
        self.cancel = false;
        if self
            .draft
            .as_ref()
            .is_some_and(|draft| draft.calculator.is_none())
        {
            ctx.input_mut(|i| {
                self.cancel = i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
                for (key, modifiers, direction) in [
                    (egui::Key::Tab, egui::Modifiers::SHIFT, -1),
                    (egui::Key::Tab, egui::Modifiers::NONE, 1),
                    (egui::Key::Enter, egui::Modifiers::SHIFT, -1),
                    (egui::Key::Enter, egui::Modifiers::NONE, 1),
                    (egui::Key::ArrowDown, egui::Modifiers::NONE, 1),
                    (egui::Key::ArrowUp, egui::Modifiers::NONE, -1),
                ] {
                    if i.consume_key(modifiers, key) {
                        self.finish = true;
                        self.direction = direction;
                    }
                }
            });
        }
    }
    pub fn is_editing(&self, cell: &Cell) -> bool {
        self.draft.as_ref().is_some_and(|d| d.target.cell == *cell)
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        cell: Cell,
        value: i64,
        currency: &str,
        options: CellOptions,
    ) {
        if !ui.is_enabled() {
            let display = self
                .draft
                .as_ref()
                .filter(|d| d.target.cell == cell)
                .map(|d| d.text.clone())
                .unwrap_or_else(|| {
                    if value == 0 {
                        String::new()
                    } else {
                        crate::i18n::number(value, currency)
                    }
                });
            ui.painter().text(
                rect.right_center() - Vec2::new(7., 0.),
                egui::Align2::RIGHT_CENTER,
                display,
                FontId::proportional(12.),
                Color32::GRAY,
            );
            return;
        }
        if !self.is_editing(&cell) {
            let response = ui
                .interact(
                    rect,
                    ui.id().with((&cell.month, &cell.category)),
                    egui::Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::Text);
            if options.row_hovered || response.hovered() {
                ui.painter().rect_filled(
                    rect,
                    0.,
                    if options.row_hovered {
                        crate::palette::CATEGORY_HOVER
                    } else {
                        Color32::from_rgb(232, 247, 252)
                    },
                );
            }
            ui.painter().text(
                rect.right_center() - Vec2::new(7., 0.),
                egui::Align2::RIGHT_CENTER,
                if value == 0 {
                    String::new()
                } else {
                    crate::i18n::number(value, currency)
                },
                FontId::proportional(12.),
                Color32::from_gray(80),
            );
            if response.clicked() {
                self.pending = Some(Target { cell, value });
            }
            return;
        }
        self.seen = true;
        let draft = self.draft.as_mut().unwrap();
        let id = ui.id().with(("budget_input", &cell.month, &cell.category));
        let focus = draft.focus;
        if focus {
            ui.memory_mut(|m| m.request_focus(id));
        }
        ui.painter().rect_filled(rect, 0., Color32::WHITE);
        // The row has already been allocated by the register. A child UI keeps
        // the input out of the parent's cursor/layout, including when it shrinks.
        let mut input_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(1.)));
        input_ui.set_clip_rect(ui.clip_rect().intersect(rect));
        input_ui.spacing_mut().interact_size.y = rect.height() - 2.;
        let output = egui::TextEdit::singleline(&mut draft.text)
            .id(id)
            .font(FontId::proportional(12.))
            .horizontal_align(Align::RIGHT)
            .vertical_align(Align::Center)
            .margin(egui::Margin::symmetric(5, 0))
            .frame(false)
            .lock_focus(true)
            .desired_width(rect.width() - 2.)
            .min_size(rect.size() - Vec2::splat(2.))
            .show(&mut input_ui);
        if focus {
            let mut state = output.state;
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(draft.text.chars().count()),
                )));
            state.store(ui.ctx(), id);
            draft.focus = false;
        }
        if output.response.changed() {
            draft.error.clear();
        }
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.,
            Stroke::new(
                1_f32,
                if draft.error.is_empty() {
                    crate::palette::CELL
                } else {
                    Color32::from_rgb(193, 67, 57)
                },
            ),
            egui::StrokeKind::Inside,
        );
        self.blur = output.response.lost_focus();
        let arrow = Rect::from_min_size(
            egui::pos2(rect.right() + 1., rect.center().y - 8.),
            Vec2::splat(16.),
        );
        let button = ui
            .interact(arrow, id.with("quick_budget"), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let down = button.is_pointer_button_down_on();
        crate::classic::gradient(
            ui.painter(),
            arrow,
            1.,
            Color32::from_gray(if down { 186 } else { 252 }),
            Color32::from_gray(if down { 225 } else { 199 }),
        );
        ui.painter().rect_stroke(
            arrow.shrink(0.5),
            1.,
            Stroke::new(1_f32, Color32::from_gray(117)),
            egui::StrokeKind::Inside,
        );
        let center = arrow.center() + Vec2::new(0., if down { 1. } else { 0. });
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                center + Vec2::new(-3.5, -1.5),
                center + Vec2::new(3.5, -1.5),
                center + Vec2::new(0., 2.5),
            ],
            Color32::from_gray(35),
            Stroke::NONE,
        ));
        let calculating = draft.calculator.is_some();
        let popup_width = if calculating { 252. } else { 435. };
        let popup = egui::Popup::menu(&button)
            .align(egui::RectAlign::BOTTOM_END)
            .gap(1.)
            .width(popup_width)
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1_f32, Color32::from_gray(55)))
                    .shadow(egui::Shadow {
                        offset: [2, 3],
                        blur: 4,
                        spread: 0,
                        color: Color32::from_black_alpha(65),
                    }),
            )
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        let popup_id = popup.get_id();
        popup.show(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            if let Some(calculator) = &mut draft.calculator {
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.add_space(10.);
                    let display = ui.add(egui::TextEdit::singleline(&mut calculator.expression)
                        .font(FontId::proportional(18.))
                        .horizontal_align(Align::RIGHT)
                        .desired_width(232.)
                        .margin(egui::Margin::symmetric(6, 5)));
                    ui.painter().rect_stroke(display.rect, 0., Stroke::new(1_f32, Color32::from_gray(155)), egui::StrokeKind::Inside);
                    if display.changed() { calculator.replace = false; calculator.error.clear(); }
                });
                ui.add_space(8.);
                for row in [
                    ["C", "+/-", "←", "/"],
                    ["7", "8", "9", "×"],
                    ["4", "5", "6", "-"],
                    ["1", "2", "3", "+"],
                    ["0", ".", "=", ""],
                ] {
                    ui.horizontal(|ui| {
                        ui.add_space(10.);
                        for label in row {
                            if label.is_empty() { ui.add_space(58.); continue; }
                            let response = calculator_button(ui, label, Vec2::new(52., 32.));
                            #[cfg(test)]
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("calculator_button", label)), response.rect));
                            if response.clicked() {
                                match label {
                                    "C" => { calculator.expression.clear(); calculator.replace = false; calculator.error.clear(); }
                                    "←" => { calculator.expression.pop(); calculator.replace = false; calculator.error.clear(); }
                                    "+/-" => {
                                        if calculator.expression.starts_with("-(") && calculator.expression.ends_with(')') {
                                            calculator.expression = calculator.expression[2..calculator.expression.len()-1].into();
                                        } else if !calculator.expression.is_empty() {
                                            calculator.expression = format!("-({})", calculator.expression);
                                        }
                                        calculator.replace = false; calculator.error.clear();
                                    }
                                    "=" => match super::budget_calculator::evaluate(&calculator.expression, currency) {
                                        Ok(value) => { calculator.expression = plain(value, currency); calculator.replace = true; calculator.error.clear(); }
                                        Err(error) => calculator.error = error,
                                    },
                                    _ => {
                                        if calculator.replace && (label.chars().next().is_some_and(|c| c.is_ascii_digit()) || label == ".") { calculator.expression.clear(); }
                                        calculator.expression.push_str(if label == "×" { "*" } else { label });
                                        calculator.replace = false; calculator.error.clear();
                                    }
                                }
                            }
                            ui.add_space(6.);
                        }
                    });
                    ui.add_space(6.);
                }
                ui.horizontal(|ui| {
                    ui.add_space(63.);
                    for label in ["OK", "Cancel"] {
                        let response = calculator_button(ui, label, Vec2::new(59., 32.));
                        #[cfg(test)]
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("calculator_button", label)), response.rect));
                        if response.clicked() {
                            if label == "Cancel" { ui.close(); }
                            else {
                                match super::budget_calculator::evaluate(&calculator.expression, currency) {
                                    Ok(value) => { draft.text = plain(value, currency); draft.error.clear(); self.finish = true; ui.close(); }
                                    Err(error) => calculator.error = error,
                                }
                            }
                        }
                        ui.add_space(8.);
                    }
                });
                if !calculator.error.is_empty() { ui.colored_label(Color32::RED, &calculator.error); }
                ui.add_space(10.);
                return;
            }
            let average_label = if options.average_months < 12 {
                format!("Average outflows for last year (only {} months available)", options.average_months)
            } else { "Average outflows for last year".to_string() };
            let entries = [
                ("Amount budgeted last month".to_string(), Ok(options.quick[0])),
                ("Outflows last month".to_string(), Ok(options.quick[1])),
                (average_label, Ok(options.quick[2])),
                ("Total scheduled outflows this month".to_string(), options.scheduled.clone()),
                ("Balance to 0.00".to_string(), Ok(options.quick[3])),
            ];
            let menu_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(popup_width, 6. * 25.));
            let pointer_in_menu = ui.ctx().pointer_hover_pos().is_some_and(|pos| menu_rect.contains(pos));
            for (index, (label, value)) in entries.into_iter().enumerate() {
                let text = match &value {
                    Ok(value) => format!("{label}:  {}", plain(*value, currency)),
                    Err(_) => format!("{label}:  unavailable"),
                };
                let response = quick_menu_row(ui, &text, popup_width, value.is_ok(), index == 0 && !pointer_in_menu, false);
                let response = if let Err(error) = &value { response.on_hover_text(error) }
                else if index == 3 { response.on_hover_text("Foreign-currency amounts use the latest available exchange rate; the final amount may differ.") }
                else { response };
                #[cfg(test)]
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("quick_option", label.as_str())), response.rect));
                if response.clicked() && let Ok(value) = value {
                    draft.text = plain(value, currency); draft.error.clear(); self.finish = true; ui.close();
                }
            }
            let response = quick_menu_row(ui, "Show calculator", popup_width, true, false, true);
            #[cfg(test)]
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("show_calculator"), response.rect));
            if response.clicked() {
                draft.calculator = Some(Calculator { expression: draft.text.clone(), error: String::new(), replace: true });
            }
        });
        if !egui::Popup::is_id_open(ui.ctx(), popup_id)
            && draft.calculator.take().is_some()
            && !self.finish
        {
            draft.focus = true;
        }
        if !draft.error.is_empty() {
            output.response.l_on_hover_text(&draft.error);
        }
        // The quick-budget popup deliberately takes focus away from the text field.
        if button.is_pointer_button_down_on()
            || button.clicked()
            || egui::Popup::is_any_open(ui.ctx())
        {
            self.blur = false;
        }
    }
    pub fn finish(
        &mut self,
        ctx: &egui::Context,
        currency: &str,
        order: &[(Cell, i64)],
        mut save: impl FnMut(&Cell, i64) -> Result<()>,
    ) {
        if self.draft.is_none() && self.pending.is_none() {
            return;
        }
        if self.cancel {
            self.draft = None;
            self.pending = None;
            ctx.request_repaint();
            return;
        }
        let should_finish = self.finish || self.blur || self.pending.is_some() || !self.seen;
        if !should_finish {
            return;
        }
        if let Some(mut draft) = self.draft.take() {
            let parsed = if draft.text.trim().is_empty() {
                Ok(0)
            } else {
                amount(&draft.text, currency)
            };
            match parsed.and_then(|value| {
                if value != draft.target.value {
                    save(&draft.target.cell, value)?;
                }
                Ok(value)
            }) {
                Ok(committed) => {
                    if self.pending.is_none()
                        && self.direction != 0
                        && let Some(index) = order.iter().position(|(c, _)| *c == draft.target.cell)
                    {
                        let index = (index as i32 + self.direction).clamp(0, order.len() as i32 - 1)
                            as usize;
                        let (cell, value) = &order[index];
                        self.pending = Some(Target {
                            cell: cell.clone(),
                            value: if *cell == draft.target.cell {
                                committed
                            } else {
                                *value
                            },
                        });
                    }
                }
                Err(error) => {
                    draft.error = error;
                    draft.focus = true;
                    self.draft = Some(draft);
                    self.pending = None;
                    ctx.request_repaint();
                    return;
                }
            }
        }
        if let Some(target) = self.pending.take() {
            self.draft = Some(Draft {
                text: plain(target.value, currency),
                target,
                error: String::new(),
                focus: true,
                calculator: None,
            });
        }
        ctx.request_repaint();
    }
}
