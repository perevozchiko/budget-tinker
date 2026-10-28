use crate::classic;
use eframe::egui::{self, Color32, FontId, Rect, Stroke, Vec2};

// Keep the user's text separate from the ISO value: parsing must not move the
// cursor, and an incomplete edit must never save the previous valid date.
#[derive(Default)]
pub struct State {
    shown: String,
    source: String,
    calendar_open: bool,
}

fn parse(text: &str) -> Option<String> {
    let format = match text.as_bytes() {
        [_, _, b'/', _, _, b'/', _, _] => "%m/%d/%y",
        [_, _, b'/', _, _, b'/', _, _, _, _] => "%m/%d/%Y",
        [_, _, _, _, b'-', _, _, b'-', _, _] => "%Y-%m-%d",
        _ => return None,
    };
    chrono::NaiveDate::parse_from_str(text, format)
        .ok()
        .map(|date| date.to_string())
        .filter(|date| budgettinker::model::valid_date(date))
}

impl State {
    pub fn calendar_open(&self) -> bool {
        self.calendar_open
    }

    pub fn text(&self) -> &str {
        &self.shown
    }

    fn sync(&mut self, value: &str) {
        if self.source != value {
            self.shown = crate::register_view::date(value);
            self.source = value.into();
        }
    }

    fn edited(&mut self, value: &mut String) {
        *value = parse(&self.shown).unwrap_or_else(|| self.shown.clone());
        self.source = value.clone();
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        id: egui::Id,
        value: &mut String,
    ) -> egui::Response {
        self.sync(value);
        let field =
            Rect::from_min_max(rect.min, rect.max - Vec2::new(24., 0.)).shrink2(Vec2::splat(2.));
        let response = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.shown)
                .id(id)
                .font(FontId::proportional(12.))
                .hint_text("MM/DD/YY")
                .margin(egui::Margin::symmetric(3, 1)),
        );
        if response.changed() {
            self.edited(value);
        }
        if response.lost_focus() && budgettinker::model::valid_date(value) {
            self.shown = crate::register_view::date(value);
        }
        if !budgettinker::model::valid_date(value) {
            ui.painter().rect_stroke(
                field,
                2.,
                Stroke::new(1_f32, Color32::from_rgb(190, 68, 61)),
                egui::StrokeKind::Inside,
            );
        }
        let response = response.on_hover_text("MM/DD/YY · MM/DD/YYYY · YYYY-MM-DD");
        let calendar = ui.put(
            Rect::from_min_max(
                egui::pos2(rect.right() - 24., rect.top() + 2.),
                rect.max - Vec2::splat(2.),
            ),
            classic::Button::new("▦").small(),
        );
        classic::date_calendar(ui, value, &calendar, Some(rect));
        self.calendar_open =
            egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&calendar));
        self.sync(value);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_complete_real_dates_are_accepted() {
        for (input, expected) in [
            ("02/29/24", "2024-02-29"),
            ("12/31/2026", "2026-12-31"),
            ("2026-01-01", "2026-01-01"),
        ] {
            assert_eq!(parse(input).as_deref(), Some(expected));
        }
        for input in [
            "",
            "02/",
            "02/28/2",
            "02/29/26",
            "04/31/26",
            "13/01/26",
            "2026-02-29",
        ] {
            assert_eq!(parse(input), None, "{input}");
        }
    }

    #[test]
    fn edits_keep_the_buffer_and_invalidate_the_previous_date() {
        let mut state = State::default();
        let mut value = "2026-01-01".to_owned();
        state.sync(&value);
        assert_eq!(state.text(), "01/01/26");
        state.shown = "02/".into();
        state.edited(&mut value);
        state.sync(&value);
        assert_eq!(state.text(), "02/");
        assert!(!budgettinker::model::valid_date(&value));
        state.shown = "02/29/2024".into();
        state.edited(&mut value);
        state.sync(&value);
        assert_eq!(value, "2024-02-29");
        assert_eq!(state.text(), "02/29/2024");
        value = "2026-12-31".into();
        state.sync(&value);
        assert_eq!(state.text(), "12/31/26");
    }
}
