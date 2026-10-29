use crate::classic;
use chrono::Datelike;
use eframe::egui::{self, Color32, Margin, RichText, Stroke, Vec2};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const PRESETS: [&str; 5] = [
    "This Month",
    "Latest 3 Months",
    "This Year",
    "Last Year",
    "All Dates",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Period {
    ThisMonth,
    LatestThree,
    ThisYear,
    LastYear,
    All,
    Custom(i32, i32),
}

fn month_index(date: chrono::NaiveDate) -> i32 {
    date.year() * 12 + date.month0() as i32
}

impl Period {
    fn bounds(self, today: chrono::NaiveDate) -> Option<(i32, i32)> {
        let month = month_index(today);
        let january = today.year() * 12;
        match self {
            Self::ThisMonth => Some((month, month)),
            Self::LatestThree => Some((month - 2, month)),
            Self::ThisYear => Some((january, january + 11)),
            Self::LastYear => Some((january - 12, january - 1)),
            Self::All => None,
            Self::Custom(from, through) => Some((from, through)),
        }
    }

    fn label(self) -> String {
        match self {
            Self::ThisMonth => PRESETS[0].into(),
            Self::LatestThree => PRESETS[1].into(),
            Self::ThisYear => PRESETS[2].into(),
            Self::LastYear => PRESETS[3].into(),
            Self::All => PRESETS[4].into(),
            Self::Custom(from, through) => {
                let name = |m: i32| {
                    format!(
                        "{} {}",
                        &MONTHS[m.rem_euclid(12) as usize][..3],
                        m.div_euclid(12)
                    )
                };
                if from == through {
                    name(from)
                } else {
                    format!("{} – {}", name(from), name(through))
                }
            }
        }
    }
}

pub struct Timeframe {
    open: bool,
    applied: Period,
    draft: Period,
    from: i32,
    through: i32,
}

impl Default for Timeframe {
    fn default() -> Self {
        let month = month_index(chrono::Local::now().date_naive());
        Self {
            open: false,
            applied: Period::All,
            draft: Period::All,
            from: month,
            through: month,
        }
    }
}

impl Timeframe {
    pub fn includes(&self, date: &str) -> bool {
        self.includes_at(date, chrono::Local::now().date_naive())
    }

    fn includes_at(&self, date: &str, today: chrono::NaiveDate) -> bool {
        let Some((from, through)) = self.applied.bounds(today) else {
            return true;
        };
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .is_ok_and(|d| (from..=through).contains(&month_index(d)))
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let trigger = ui.add(classic::Button::new(format!("{}  ▼", self.applied.label())));
        if trigger.clicked() {
            self.open = !self.open;
            self.draft = self.applied;
            (self.from, self.through) = self
                .applied
                .bounds(chrono::Local::now().date_naive())
                .unwrap_or_else(|| {
                    let m = month_index(chrono::Local::now().date_naive());
                    (m, m)
                });
        }
        let mut changed = false;
        let popup = egui::Popup::menu(&trigger)
            .open_bool(&mut self.open)
            .align(egui::emath::RectAlign::BOTTOM)
            .width(458.)
            .gap(16.)
            .close_behavior(if egui::Popup::is_any_open(ui.ctx()) {
                egui::PopupCloseBehavior::IgnoreClicks
            } else {
                egui::PopupCloseBehavior::CloseOnClickOutside
            })
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_gray(245))
                    .stroke(Stroke::new(1_f32, Color32::from_gray(174)))
                    .corner_radius(4)
                    .inner_margin(Margin::ZERO)
                    .shadow(egui::epaint::Shadow {
                        offset: [1, 5],
                        blur: 10,
                        spread: 0,
                        color: Color32::from_black_alpha(100),
                    }),
            )
            .show(|ui| {
                ui.set_width(458.);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                band(ui, Color32::from_gray(232), 9, |ui| {
                    ui.label(RichText::new("Timeframe").size(18.).color(classic::TEXT));
                });
                divider(ui);
                band(ui, Color32::WHITE, 3, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.;
                        for (index, period) in [
                            Period::ThisMonth,
                            Period::LatestThree,
                            Period::ThisYear,
                            Period::LastYear,
                            Period::All,
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let selected = self.draft == period;
                            let button = egui::Button::new(
                                RichText::new(PRESETS[index])
                                    .font(crate::palette::bold(13.))
                                    .color(if selected {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(0, 145, 207)
                                    }),
                            )
                            .fill(if selected {
                                Color32::from_rgb(65, 181, 222)
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(Stroke::NONE)
                            .corner_radius(12);
                            if ui.add(button).clicked() {
                                self.draft = period;
                                if let Some((from, through)) =
                                    period.bounds(chrono::Local::now().date_naive())
                                {
                                    self.from = from;
                                    self.through = through;
                                }
                            }
                        }
                    });
                });
                divider(ui);
                band(ui, Color32::from_gray(245), 12, |ui| {
                    ui.add_space(9.);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.;
                        ui.label(
                            RichText::new("From:")
                                .font(crate::palette::bold(12.))
                                .color(classic::TEXT),
                        );
                        let from_changed = month_picker(ui, "from", &mut self.from);
                        ui.label(
                            RichText::new("Through:")
                                .font(crate::palette::bold(12.))
                                .color(classic::TEXT),
                        );
                        let through_changed = month_picker(ui, "through", &mut self.through);
                        if from_changed || through_changed {
                            self.draft = Period::Custom(self.from, self.through);
                        }
                    });
                    ui.add_space(9.);
                    if self.from > self.through {
                        ui.colored_label(
                            Color32::from_rgb(180, 45, 35),
                            "From must be before or equal to Through.",
                        );
                    }
                });
                divider(ui);
                band(ui, Color32::from_gray(232), 7, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.;
                        if action_button(
                            ui,
                            "Done",
                            true,
                            self.draft == Period::All || self.from <= self.through,
                        )
                        .clicked()
                        {
                            changed = self.applied != self.draft;
                            self.applied = self.draft;
                            ui.close();
                        }
                        if action_button(ui, "Cancel", false, true).clicked() {
                            ui.close();
                        }
                    });
                });
            });
        if let Some(popup) = popup {
            classic::popup_pointer(
                &ui.ctx().layer_painter(popup.response.layer_id),
                popup.response.rect,
                trigger.rect.center().x,
                trigger.rect.bottom() + 1.,
                Color32::from_gray(232),
                Stroke::new(1_f32, Color32::from_gray(174)),
            );
        }
        changed
    }
}

fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(458., 1.), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1_f32, Color32::from_gray(177)),
    );
}

fn band(ui: &mut egui::Ui, fill: Color32, padding: i8, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(fill)
        .inner_margin(Margin::same(padding))
        .show(ui, |ui| {
            ui.set_width(458. - padding as f32 * 2.);
            // The register toolbar lays out right to left; the popover reads left to right.
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), content);
        });
}

fn silver(p: &egui::Painter, rect: egui::Rect, green: bool, hovered: bool) {
    let top = if green {
        Color32::from_rgb(247, 253, 237)
    } else {
        Color32::from_gray(253)
    };
    let bottom = if green {
        Color32::from_rgb(187, 216, 143)
    } else {
        Color32::from_gray(if hovered { 224 } else { 211 })
    };
    classic::gradient(p, rect, 4., top, bottom);
    p.rect_stroke(
        rect,
        4.,
        Stroke::new(1_f32, Color32::from_gray(149)),
        egui::StrokeKind::Inside,
    );
    p.rect_stroke(
        rect.shrink(1.),
        3.,
        Stroke::new(0.6_f32, Color32::WHITE),
        egui::StrokeKind::Inside,
    );
}

pub(crate) fn action_button(
    ui: &mut egui::Ui,
    label: &str,
    done: bool,
    enabled: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(if done { 69. } else { 76. }, 28.),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let p = ui.painter();
    silver(p, rect, done, response.hovered());
    let c = rect.left_center() + Vec2::new(14., 0.);
    let ink = if done {
        Color32::from_rgb(74, 151, 0)
    } else {
        Color32::from_rgb(221, 41, 29)
    };
    p.circle_filled(c + Vec2::new(0., 0.6), 7.3, ink.gamma_multiply(0.75));
    p.circle_filled(c, 6.8, ink);
    p.circle_stroke(c, 6.2, Stroke::new(0.7_f32, Color32::from_white_alpha(110)));
    let white = Stroke::new(1.7_f32, Color32::WHITE);
    if done {
        p.line_segment([c + Vec2::new(-3., 0.), c + Vec2::new(-0.7, 2.3)], white);
        p.line_segment([c + Vec2::new(-0.7, 2.3), c + Vec2::new(3.5, -2.7)], white);
    } else {
        p.line_segment([c + Vec2::new(-2.4, -2.4), c + Vec2::new(2.4, 2.4)], white);
        p.line_segment([c + Vec2::new(-2.4, 2.4), c + Vec2::new(2.4, -2.4)], white);
    }
    p.text(
        rect.left_center() + Vec2::new(26., 0.),
        egui::Align2::LEFT_CENTER,
        label,
        crate::palette::bold(13.),
        if enabled {
            Color32::from_gray(52)
        } else {
            Color32::from_gray(155)
        },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response
}

fn dropdown(
    ui: &mut egui::Ui,
    id: &str,
    width: f32,
    selected: &str,
    content: impl FnOnce(&mut egui::Ui, bool),
) {
    ui.push_id(id, |ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 27.), egui::Sense::click());
        silver(ui.painter(), rect, false, response.hovered());
        ui.painter().text(
            rect.left_center() + Vec2::new(7., 0.),
            egui::Align2::LEFT_CENTER,
            selected,
            crate::palette::bold(13.),
            Color32::from_gray(52),
        );
        let c = rect.right_center() - Vec2::new(12., 0.);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                c + Vec2::new(-4., -2.),
                c + Vec2::new(4., -2.),
                c + Vec2::new(0., 3.),
            ],
            Color32::from_gray(52),
            Stroke::NONE,
        ));
        egui::Popup::menu(&response).width(width).show(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            egui::ScrollArea::vertical()
                .max_height(220.)
                .show(ui, |ui| content(ui, response.clicked()));
        });
    });
}

pub(crate) fn month_picker(ui: &mut egui::Ui, id: &str, value: &mut i32) -> bool {
    let old = *value;
    let mut month = value.rem_euclid(12);
    let mut year = value.div_euclid(12);
    dropdown(
        ui,
        &format!("{id}_month"),
        99.,
        MONTHS[month as usize],
        |ui, _opening| {
            for (index, name) in MONTHS.iter().enumerate() {
                if ui
                    .selectable_value(&mut month, index as i32, *name)
                    .clicked()
                {
                    ui.close();
                }
            }
        },
    );
    dropdown(
        ui,
        &format!("{id}_year"),
        59.,
        &year.to_string(),
        |ui, opening| {
            let this_year = chrono::Local::now().year();
            for candidate in year.min(1900)..=year.max(this_year + 100) {
                let response = ui.selectable_value(&mut year, candidate, candidate.to_string());
                if opening && candidate == year {
                    response.scroll_to_me(Some(egui::Align::Center));
                }
                if response.clicked() {
                    ui.close();
                }
            }
        },
    );
    *value = year * 12 + month;
    old != *value
}

#[cfg(test)]
mod tests {
    use super::*;
    fn date(s: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn presets_cross_years_and_include_whole_months() {
        let now = date("2026-01-15");
        let mut filter = Timeframe {
            applied: Period::LatestThree,
            ..Default::default()
        };
        assert!(filter.includes_at("2025-11-01", now));
        assert!(filter.includes_at("2026-01-31", now));
        assert!(!filter.includes_at("2025-10-31", now));
        assert!(!filter.includes_at("2026-02-01", now));
        filter.applied = Period::ThisYear;
        assert!(filter.includes_at("2026-12-31", now));
        assert!(!filter.includes_at("2025-12-31", now));
        filter.applied = Period::LastYear;
        assert!(filter.includes_at("2025-01-01", now));
        assert!(filter.includes_at("2025-12-31", now));
        assert!(!filter.includes_at("2026-01-01", now));
    }

    #[test]
    fn draft_does_not_change_filter_and_custom_includes_end_of_leap_month() {
        let now = date("2026-10-27");
        let mut filter = Timeframe {
            draft: Period::Custom(2024 * 12 + 1, 2024 * 12 + 1),
            ..Default::default()
        };
        assert!(filter.includes_at("2026-10-27", now));
        filter.applied = filter.draft;
        assert!(filter.includes_at("2024-02-01", now));
        assert!(filter.includes_at("2024-02-29", now));
        assert!(!filter.includes_at("2024-03-01", now));
        assert!(!filter.includes_at("2024-01-31", now));
    }
    fn render(
        ctx: &egui::Context,
        filter: &mut Timeframe,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1000., 600.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    filter.show(ui);
                });
            },
        )
    }

    fn click_text(ctx: &egui::Context, filter: &mut Timeframe, label: &str) {
        let output = render(ctx, filter, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == label
                {
                    return Some(text.pos + text.galley.size() * 0.5);
                }
                None
            })
            .unwrap_or_else(|| panic!("Missing control: {label}"));
        render(
            ctx,
            filter,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        render(
            ctx,
            filter,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        render(ctx, filter, vec![]);
    }

    #[test]
    fn popup_presets_wait_for_done_and_cancel_discards_edits() {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        // Exercise the same named font family with egui's bundled fonts on every OS.
        fonts.families.insert(
            egui::FontFamily::Name("arial-bold".into()),
            fonts.families[&egui::FontFamily::Proportional].clone(),
        );
        ctx.set_fonts(fonts);
        let mut filter = Timeframe::default();
        click_text(&ctx, &mut filter, "All Dates  ▼");
        click_text(&ctx, &mut filter, "Last Year");
        assert_eq!(filter.applied, Period::All);
        assert_eq!(filter.draft, Period::LastYear);
        click_text(&ctx, &mut filter, "Cancel");
        assert_eq!(filter.applied, Period::All);
        click_text(&ctx, &mut filter, "All Dates  ▼");
        assert_eq!(filter.draft, Period::All);
        click_text(&ctx, &mut filter, "This Year");
        click_text(&ctx, &mut filter, "Done");
        assert_eq!(filter.applied, Period::ThisYear);
        click_text(&ctx, &mut filter, "This Year  ▼");
        click_text(&ctx, &mut filter, "January");
        click_text(&ctx, &mut filter, "May");
        assert!(matches!(filter.draft, Period::Custom(_, _)));
        click_text(&ctx, &mut filter, "Done");
        assert!(matches!(filter.applied, Period::Custom(_, _)));
    }
}
