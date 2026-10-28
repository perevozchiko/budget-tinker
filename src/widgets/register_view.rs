use crate::classic;
use budgettinker::model::{Budget, Transaction, TransactionFlag};
use eframe::egui::{self, Align2, Color32, FontId, Rect, Sense, Stroke, Vec2};

// Keep the editor's established field indexes while placing Flag before Account.
pub const ORDER: [usize; 10] = [0, 9, 1, 2, 3, 4, 5, 6, 7, 8];

pub fn date(value: &str) -> String {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|date| date.format("%m/%d/%y").to_string())
        .unwrap_or_else(|_| value.into())
}

pub fn flag(p: &egui::Painter, rect: Rect, flag: Option<TransactionFlag>) {
    let c = rect.center();
    let r = Rect::from_center_size(c, Vec2::new(15., 9.));
    let color = match flag {
        Some(TransactionFlag::Red) => Color32::from_rgb(226, 40, 30),
        Some(TransactionFlag::Orange) => Color32::from_rgb(255, 133, 0),
        Some(TransactionFlag::Yellow) => Color32::from_rgb(241, 218, 0),
        Some(TransactionFlag::Green) => Color32::from_rgb(136, 190, 14),
        Some(TransactionFlag::Blue) => Color32::from_rgb(0, 144, 199),
        Some(TransactionFlag::Purple) => Color32::from_rgb(154, 133, 184),
        None => Color32::from_gray(220),
    };
    p.add(egui::Shape::convex_polygon(
        vec![
            r.left_top(),
            r.right_top(),
            r.right_center() - Vec2::new(4., 0.),
            r.right_bottom(),
            r.left_bottom(),
        ],
        color,
        Stroke::new(0.7_f32, color.gamma_multiply(0.8)),
    ));
}

pub fn flag_menu(ui: &mut egui::Ui, count: usize) -> Option<Option<TransactionFlag>> {
    ui.spacing_mut().item_spacing.y = 0.;
    ui.set_min_width(152.);
    ui.label(format!(
        "{count} Selected transaction{}:",
        if count == 1 { "" } else { "s" }
    ));
    let mut result = None;
    for value in TransactionFlag::ALL.into_iter().map(Some).chain([None]) {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(152., 24.), Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, 0., Color32::from_rgb(207, 231, 246));
        }
        flag(
            ui.painter(),
            Rect::from_min_size(rect.min, Vec2::new(23., 24.)),
            value,
        );
        ui.painter().text(
            rect.left_center() + Vec2::new(28., 0.),
            Align2::LEFT_CENTER,
            value
                .as_ref()
                .map(TransactionFlag::label)
                .unwrap_or("Remove Flag"),
            FontId::proportional(12.),
            classic::TEXT,
        );
        if response.clicked() {
            result = Some(value);
            ui.close();
        }
    }
    result
}

pub fn reconciled_buttons(ui: &mut egui::Ui, hide: &mut bool, searching: bool) -> bool {
    let before = *hide;
    ui.add_enabled_ui(!searching, |ui| {
        ui.spacing_mut().item_spacing.x = 0.;
        for hidden in [true, false] {
            let (rect, response) = ui.allocate_exact_size(Vec2::new(25., 25.), Sense::click());
            let selected = hidden == *hide;
            classic::gradient(
                ui.painter(),
                rect,
                3.,
                if selected {
                    Color32::from_gray(207)
                } else {
                    Color32::from_gray(251)
                },
                if selected {
                    Color32::from_gray(182)
                } else {
                    Color32::from_gray(222)
                },
            );
            ui.painter().rect_stroke(
                rect,
                3.,
                Stroke::new(0.8_f32, Color32::from_gray(145)),
                egui::StrokeKind::Inside,
            );
            classic::reconciled_lock(ui.painter(), rect.center(), false);
            if hidden {
                let slash = [
                    rect.left_bottom() + Vec2::new(4., -4.),
                    rect.right_top() + Vec2::new(-4., 4.),
                ];
                ui.painter()
                    .line_segment(slash, Stroke::new(4_f32, Color32::WHITE));
                ui.painter()
                    .line_segment(slash, Stroke::new(1.8_f32, Color32::from_gray(65)));
            }
            if response.clicked() {
                *hide = hidden;
            }
            response.on_hover_text(if searching {
                "Both reconciled and unreconciled transactions are shown while searching"
            } else if hidden {
                "Hide reconciled transactions"
            } else {
                "Show reconciled transactions"
            });
        }
    });
    *hide != before
}

pub fn sort(b: &Budget, rows: &mut [Transaction], column: usize, ascending: bool) {
    rows.sort_by(|a, z| {
        let order = match column {
            1 => b
                .account(&a.account)
                .unwrap()
                .name
                .to_lowercase()
                .cmp(&b.account(&z.account).unwrap().name.to_lowercase()),
            3 => a.payee.to_lowercase().cmp(&z.payee.to_lowercase()),
            4 => b
                .transaction_category_name(a)
                .to_lowercase()
                .cmp(&b.transaction_category_name(z).to_lowercase()),
            5 => a.memo.to_lowercase().cmp(&z.memo.to_lowercase()),
            6 => (-a.base_amount.min(0)).cmp(&(-z.base_amount.min(0))),
            7 => a.base_amount.max(0).cmp(&z.base_amount.max(0)),
            8 => (a.cleared, a.reconciled).cmp(&(z.cleared, z.reconciled)),
            9 => a.flag.cmp(&z.flag),
            _ => a.date.cmp(&z.date),
        };
        if ascending { order } else { order.reverse() }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_match_original_without_changing_storage_dates() {
        assert_eq!(date("2026-10-25"), "10/25/26");
        assert_eq!(date("2028-02-29"), "02/29/28");
        assert_eq!(date("invalid"), "invalid");
    }
    #[test]
    fn header_sorting_uses_values_and_keeps_equal_rows_stable() {
        let mut b = Budget::new("EUR");
        b.add_account("Cash", "EUR", true, 100, "2026-01-01")
            .unwrap();
        let mut first = b.transactions[0].clone();
        first.id = "first".into();
        first.payee = "Zulu".into();
        first.flag = Some(TransactionFlag::Blue);
        let mut second = first.clone();
        second.id = "second".into();
        second.payee = "alpha".into();
        second.flag = Some(TransactionFlag::Red);
        let mut third = second.clone();
        third.id = "third".into();
        let mut rows = vec![first, second, third];
        sort(&b, &mut rows, 3, true);
        assert_eq!(
            rows.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["second", "third", "first"]
        );
        sort(&b, &mut rows, 9, false);
        assert_eq!(
            rows.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["first", "second", "third"]
        );
        assert_eq!(b.transactions[0].date, "2026-01-01");
    }
}
