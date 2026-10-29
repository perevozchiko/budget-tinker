use eframe::egui::{self, Align, Color32, FontId, Pos2, Response, Ui, WidgetText};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Ru,
}
impl Language {
    pub fn install(self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("language"), self));
    }
    pub fn current(ctx: &egui::Context) -> Self {
        ctx.data(|d| d.get_temp(egui::Id::new("language")).unwrap_or_default())
    }
    pub fn translate(self, source: &str) -> String {
        if self == Self::Ru {
            return source.into();
        }
        if source == budgettinker::credit::INITIAL_DEBT_GROUP {
            return "Starting Debt".into();
        }
        let dictionary = DICTIONARY.get_or_init(|| {
            serde_json::from_str(include_str!("../locales/en.json"))
                .expect("valid English catalogue")
        });
        if let Some(text) = dictionary.get(source) {
            return text.clone();
        }
        if !source
            .chars()
            .any(|c| ('А'..='я').contains(&c) || c == 'ё' || c == 'Ё')
        {
            return source.into();
        }
        for (pattern, english) in dictionary.iter().filter(|(p, _)| p.contains('{')) {
            if let Some(values) = capture(pattern, source) {
                let mut result = english.clone();
                for (key, value) in values {
                    result = result.replacen(&key, &self.translate(&value), 1);
                }
                return result;
            }
        }
        source.into()
    }
}
static DICTIONARY: OnceLock<BTreeMap<String, String>> = OnceLock::new();
fn capture(pattern: &str, source: &str) -> Option<Vec<(String, String)>> {
    let mut pattern = pattern;
    let mut source = source;
    let mut values = vec![];
    while let Some(open) = pattern.find('{') {
        source = source.strip_prefix(&pattern[..open])?;
        let close = pattern[open..].find('}')? + open;
        let token = &pattern[open..=close];
        let tail = &pattern[close + 1..];
        let literal_end = tail.find('{').unwrap_or(tail.len());
        let literal = &tail[..literal_end];
        let end = if literal.is_empty() {
            source.len()
        } else {
            source.find(literal)?
        };
        values.push((token.into(), source[..end].into()));
        source = &source[end..];
        pattern = tail;
    }
    (source == pattern).then_some(values)
}
pub fn tr(ctx: &egui::Context, text: impl AsRef<str>) -> String {
    Language::current(ctx).translate(text.as_ref())
}
pub fn tr_language(language: Language, text: &str) -> String {
    language.translate(text)
}
pub fn widget(ui: &Ui, text: impl Into<WidgetText>) -> WidgetText {
    let text = text.into();
    let translated = tr(ui.ctx(), text.text());
    if translated == text.text() {
        return text;
    }
    match text {
        WidgetText::RichText(rich) => {
            let job = WidgetText::RichText(rich).into_layout_job(
                ui.style(),
                egui::FontSelection::Default,
                Align::Center,
            );
            let mut job = Arc::unwrap_or_clone(job);
            job.text = translated;
            for section in &mut job.sections {
                section.byte_range = 0..job.text.len();
            }
            WidgetText::LayoutJob(Arc::new(job))
        }
        _ => translated.into(),
    }
}
pub trait LocalizedUi {
    fn l_label(&mut self, text: impl Into<WidgetText>) -> Response;
    fn l_strong(&mut self, text: impl Into<WidgetText>) -> Response;
    fn l_checkbox(&mut self, value: &mut bool, text: impl Into<WidgetText>) -> Response;
    fn l_selectable_value<T: PartialEq>(
        &mut self,
        current: &mut T,
        value: T,
        text: impl Into<WidgetText>,
    ) -> Response;
}
impl LocalizedUi for Ui {
    fn l_label(&mut self, text: impl Into<WidgetText>) -> Response {
        self.label(widget(self, text))
    }
    fn l_strong(&mut self, text: impl Into<WidgetText>) -> Response {
        self.label(widget(self, text).strong())
    }
    fn l_checkbox(&mut self, value: &mut bool, text: impl Into<WidgetText>) -> Response {
        self.checkbox(value, widget(self, text))
    }
    fn l_selectable_value<T: PartialEq>(
        &mut self,
        current: &mut T,
        value: T,
        text: impl Into<WidgetText>,
    ) -> Response {
        self.selectable_value(current, value, widget(self, text))
    }
}
pub trait LocalizedPainter {
    fn l_text(
        &self,
        pos: Pos2,
        anchor: egui::Align2,
        text: impl ToString,
        font: FontId,
        color: Color32,
    ) -> egui::Rect;
}
impl LocalizedPainter for egui::Painter {
    fn l_text(
        &self,
        pos: Pos2,
        anchor: egui::Align2,
        text: impl ToString,
        font: FontId,
        color: Color32,
    ) -> egui::Rect {
        self.text(pos, anchor, tr(self.ctx(), text.to_string()), font, color)
    }
}
pub trait LocalizedResponse {
    fn l_on_hover_text(self, text: impl Into<WidgetText>) -> Self;
}
impl LocalizedResponse for Response {
    fn l_on_hover_text(self, text: impl Into<WidgetText>) -> Self {
        let text = text.into();
        let translated = tr(&self.ctx, text.text());
        self.on_hover_text(translated)
    }
}
pub fn number(value: i64, currency: &str) -> String {
    budgettinker::model::grouped(value, currency)
}
pub fn money(value: i64, currency: &str) -> String {
    budgettinker::model::money(value, currency)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switches_labels_and_templates() {
        assert_eq!(Language::En.translate("Бюджет"), "Budget");
        assert_eq!(Language::Ru.translate("Бюджет"), "Бюджет");
        assert_eq!(
            Language::En.translate("Все суммы в EUR"),
            "All amounts in EUR"
        );
        assert_eq!(
            Language::En.translate("Допустимо знаков после запятой: 2"),
            "Maximum decimal places: 2"
        );
    }
    #[test]
    fn preserves_unknown_account_names_and_currency_values() {
        assert_eq!(Language::En.translate("Мой личный счёт"), "Мой личный счёт");
        assert_eq!(Language::En.translate("test USD"), "test USD");
    }
    #[test]
    fn translates_nested_validation_errors() {
        assert_eq!(
            Language::En.translate("Строка 2: Счёт не найден"),
            "Row 2: Account not found"
        );
    }
    #[test]
    fn release_validation_errors_render_in_english_without_rewriting_user_text() {
        for (source, expected) in [
            (
                "Для будущего месяца введите плановый курс вручную",
                "Enter a planned exchange rate manually for future months",
            ),
            (
                "Сервис вернул некорректный курс",
                "The service returned an invalid exchange rate",
            ),
            (
                "Редактирование доступно для одного плана",
                "Select a single scheduled transaction to edit",
            ),
            (
                "Сумма операции превышает лимит",
                "The transaction amount exceeds the limit",
            ),
            (
                "Не удалось получить курс. Можно ввести его вручную. timeout",
                "Could not fetch the exchange rate. You can enter it manually. timeout",
            ),
            (
                "Строка 3: Сумма операции превышает лимит",
                "Row 3: The transaction amount exceeds the limit",
            ),
        ] {
            assert_eq!(Language::En.translate(source), expected);
        }
        assert_eq!(
            Language::En.translate("Моя категория пользователя"),
            "Моя категория пользователя"
        );
    }

    #[test]
    fn formatting_never_changes_stored_minor_units() {
        assert_eq!(number(123456789, "EUR"), "1 234 567.89");
        assert_eq!(money(-123456, "USD"), "-1 234.56 USD");
        assert_eq!(number(123456, "JPY"), "123 456");
    }
}
