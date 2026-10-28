use crate::i18n::LocalizedResponse as _;
use budgettinker::model::{Result, amount, plain};
use eframe::egui::{self, Align, Color32, FontId, Rect, Stroke, Vec2};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub month: String,
    pub category: String,
}

#[derive(Default)]
pub struct CellOptions {
    pub quick: [i64; 3],
    pub row_hovered: bool,
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
                            self.editor.show(
                                ui,
                                Rect::from_min_size(
                                    egui::pos2(20., 30. + i as f32 * 30.),
                                    Vec2::new(120., 25.),
                                ),
                                cell.clone(),
                                *value,
                                "EUR",
                                CellOptions::default(),
                            );
                        }
                    });
                    let order = self.values.clone();
                    self.editor.finish(ctx, "EUR", &order, |cell, value| {
                        if self.fail {
                            return Err("disk full".into());
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
        if self.draft.is_some() {
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
        let output = ui
            .scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(1.)), |ui| {
                ui.spacing_mut().interact_size.y = rect.height() - 2.;
                egui::TextEdit::singleline(&mut draft.text)
                    .id(id)
                    .font(FontId::proportional(12.))
                    .horizontal_align(Align::RIGHT)
                    .vertical_align(Align::Center)
                    .margin(egui::Margin::symmetric(5, 0))
                    .frame(false)
                    .lock_focus(true)
                    .desired_width(rect.width() - 2.)
                    .min_size(rect.size() - Vec2::splat(2.))
                    .show(ui)
            })
            .inner;
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
            rect.right_top() + Vec2::new(2., 2.),
            Vec2::new(17., rect.height() - 4.),
        );
        let button = ui.put(arrow, crate::classic::Button::new("▾").small());
        egui::Popup::menu(&button).show(|ui| {
            for (label, value) in [
                "Назначено в прошлом месяце",
                "Расходы прошлого месяца",
                "Довести остаток до нуля",
            ]
            .iter()
            .zip(options.quick)
            {
                if crate::classic::button(ui, format!("{label}: {}", plain(value, currency)))
                    .clicked()
                {
                    draft.text = plain(value, currency);
                    draft.error.clear();
                    self.finish = true;
                    ui.close();
                }
            }
        });
        if !draft.error.is_empty() {
            output.response.l_on_hover_text(&draft.error);
        }
        // The quick-budget popup deliberately takes focus away from the text field.
        if button.clicked() || egui::Popup::is_any_open(ui.ctx()) {
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
            });
        }
        ctx.request_repaint();
    }
}
