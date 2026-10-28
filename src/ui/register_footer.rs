use eframe::egui::{self, Align2, Color32, FontId, Rect, Sense, Stroke, Vec2};

use crate::classic;
use crate::i18n::{self, LocalizedPainter as _};

const INK: Color32 = Color32::from_rgb(65, 69, 71);
const EDGE: Color32 = Color32::from_rgb(135, 145, 149);

fn silver_button(ui: &mut egui::Ui, rect: Rect, label: &str, calendar: bool) -> bool {
    let response = ui.interact(rect, ui.id().with(label), Sense::click());
    classic::gradient(
        ui.painter(),
        rect,
        4.,
        if response.hovered() {
            Color32::WHITE
        } else {
            Color32::from_rgb(253, 253, 253)
        },
        if response.is_pointer_button_down_on() {
            Color32::from_gray(190)
        } else {
            Color32::from_rgb(210, 213, 215)
        },
    );
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        4.,
        Stroke::new(1.0_f32, EDGE),
        egui::StrokeKind::Inside,
    );
    let center = rect.center();
    if calendar {
        let icon = Rect::from_min_size(rect.min + Vec2::new(12., 11.), Vec2::new(28., 26.));
        ui.painter().rect_stroke(
            icon,
            1.,
            Stroke::new(1.5_f32, INK),
            egui::StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [
                icon.left_top() + Vec2::new(0., 7.),
                icon.right_top() + Vec2::new(0., 7.),
            ],
            Stroke::new(1.5_f32, INK),
        );
        ui.painter()
            .hline(icon.x_range(), icon.top() + 4., Stroke::new(1.5_f32, INK));
        for x in [6., 22.] {
            ui.painter().line_segment(
                [icon.min + Vec2::new(x, -4.), icon.min + Vec2::new(x, 4.)],
                Stroke::new(2_f32, INK),
            );
        }
        for x in [7., 14., 21.] {
            ui.painter().line_segment(
                [
                    icon.min + Vec2::new(x, 9.),
                    icon.left_bottom() + Vec2::new(x, -2.),
                ],
                Stroke::new(0.8_f32, INK),
            );
        }
        for y in [12., 17., 22.] {
            ui.painter().line_segment(
                [
                    icon.min + Vec2::new(2., y),
                    icon.right_top() + Vec2::new(-2., y),
                ],
                Stroke::new(0.8_f32, INK),
            );
        }
    }
    let x = if calendar {
        rect.left() + 48.
    } else {
        center.x
    };
    let align = if calendar {
        Align2::LEFT_CENTER
    } else {
        Align2::CENTER_CENTER
    };
    let translated = i18n::tr(ui.ctx(), label);
    let lines: Vec<_> = translated.split('\n').collect();
    for (index, line) in lines.iter().enumerate() {
        ui.painter().l_text(
            egui::pos2(
                x,
                center.y + (index as f32 - (lines.len() as f32 - 1.) / 2.) * 16.,
            ),
            align,
            line,
            FontId::proportional(12.),
            INK,
        );
    }
    response.clicked()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    None,
    Scheduled,
    Reconcile,
    Cleared,
    Uncleared,
}

pub struct Summary<'a> {
    pub currency: &'a str,
    pub cleared: Option<i64>,
    pub working: Option<i64>,
    pub search: Option<(usize, Option<i64>)>,
    pub selected: Option<(usize, Option<i64>)>,
}

/// Balance cells are search shortcuts, just like the original register footer.
pub fn show(ui: &mut egui::Ui, rect: Rect, totals: Summary<'_>, reconcile: bool) -> Action {
    let painter = ui.painter();
    painter.hline(rect.x_range(), rect.top(), Stroke::new(1.0_f32, EDGE));
    painter.hline(
        rect.x_range(),
        rect.top() + 1.,
        Stroke::new(1.0_f32, Color32::WHITE),
    );
    let button_h = 51.;
    let y = rect.bottom() - button_h - 6.;
    let schedule_rect =
        Rect::from_min_size(egui::pos2(rect.left() + 5., y), Vec2::new(151., button_h));
    let reconcile_rect =
        Rect::from_min_size(egui::pos2(rect.right() - 103., y), Vec2::new(98., button_h));
    let summary_w = (rect.width() * 0.40)
        .clamp(390., 465.)
        .min((rect.width() - 280.).max(100.));
    let summary_right = if reconcile {
        reconcile_rect.left() - 5.
    } else {
        rect.right() - 5.
    };
    let summary = Rect::from_min_size(
        egui::pos2(summary_right - summary_w, y),
        Vec2::new(summary_w, button_h),
    );
    classic::gradient(
        painter,
        summary,
        4.,
        Color32::WHITE,
        Color32::from_rgb(235, 236, 237),
    );
    painter.rect_stroke(
        summary.shrink(0.5),
        4.,
        Stroke::new(1.0_f32, EDGE),
        egui::StrokeKind::Inside,
    );
    let searching = totals.search.is_some();
    let labels = if let Some((count, _)) = totals.search {
        [
            totals
                .selected
                .map(|(n, _)| format!("Selected Total ({n})"))
                .unwrap_or_default(),
            format!("Search Total ({count})"),
            "Working Balance".into(),
        ]
    } else {
        [
            "Cleared Balance".into(),
            "Uncleared Transactions".into(),
            "Working Balance".into(),
        ]
    };
    let amounts = if let Some((_, amount)) = totals.search {
        [
            totals.selected.and_then(|(_, amount)| amount),
            amount,
            totals.working,
        ]
    } else {
        [
            totals.cleared,
            totals.working.zip(totals.cleared).map(|(w, c)| w - c),
            totals.working,
        ]
    };
    let tooltips = [
        "Total of cleared and reconciled transactions. Click to display these transactions.",
        "Total of uncleared transactions (these haven't yet cleared your bank). Click to display these transactions.",
        "This is the actual money you have available in this account based on both Cleared and Uncleared transactions.",
    ];
    let ratios = [0.31, 0.36, 0.33];
    let mut left = summary.left();
    let mut action = Action::None;
    for (i, ratio) in ratios.iter().enumerate() {
        let width = summary.width() * ratio;
        let cell = Rect::from_min_size(
            egui::pos2(left, summary.top()),
            Vec2::new(width, summary.height()),
        );
        let clickable = !searching && i < 2;
        let response = ui.interact(
            cell.shrink2(Vec2::new(3., 4.)),
            ui.id().with(("balance_summary", i)),
            if clickable {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        if clickable && response.hovered() {
            painter.rect_filled(response.rect, 3., Color32::from_gray(222));
        }
        if clickable && response.clicked() {
            action = if i == 0 {
                Action::Cleared
            } else {
                Action::Uncleared
            };
        }
        if clickable {
            response
                .clone()
                .on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        response.on_hover_text(if searching && i < 2 {
            if i == 0 {
                "Total of all selected transactions"
            } else {
                "This is the total of all transactions found in the current search"
            }
        } else {
            tooltips[i]
        });
        painter.text(
            egui::pos2(cell.center().x, cell.top() + 14.),
            Align2::CENTER_CENTER,
            &labels[i],
            FontId::proportional(10.),
            INK,
        );
        if !labels[i].is_empty() {
            painter.text(
                egui::pos2(cell.center().x, cell.top() + 33.),
                Align2::CENTER_CENTER,
                amounts[i]
                    .map(|v| i18n::money(v, totals.currency))
                    .unwrap_or_else(|| "—".into()),
                FontId::proportional(15.),
                if amounts[i].is_some_and(|v| v < 0) {
                    Color32::from_rgb(215, 45, 35)
                } else {
                    INK
                },
            );
        }
        left += width;
    }
    for (x, sign) in [
        (summary.left() + summary.width() * 0.31, "+"),
        (summary.left() + summary.width() * 0.67, "="),
    ] {
        if searching {
            painter.vline(
                x,
                summary.top() + 7. ..=summary.bottom() - 7.,
                Stroke::new(0.7_f32, EDGE),
            );
        } else {
            painter.text(
                egui::pos2(x, summary.top() + 34.),
                Align2::CENTER_CENTER,
                sign,
                FontId::proportional(18.),
                INK,
            );
        }
    }
    if silver_button(ui, schedule_rect, "Плановые\nоперации", true) {
        action = Action::Scheduled;
    }
    if reconcile && silver_button(ui, reconcile_rect, "Сверить\nсчёт", false) {
        action = Action::Reconcile;
    }
    action
}
