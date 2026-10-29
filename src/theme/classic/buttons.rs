use super::*;

#[derive(Clone, Copy)]
pub enum Glyph {
    Add,
    Transfer,
    Import,
    Check,
}
pub struct Button {
    label: WidgetText,
    small: bool,
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    icon: Option<Glyph>,
    selected: bool,
    primary: bool,
}
impl Button {
    pub fn new(label: impl Into<WidgetText>) -> Self {
        Self {
            label: label.into(),
            small: false,
            fill: None,
            stroke: None,
            icon: None,
            selected: false,
            primary: false,
        }
    }
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }
    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }
}
impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let color = if self.primary && ui.is_enabled() {
            Color32::WHITE
        } else if ui.is_enabled() {
            TEXT
        } else {
            Color32::from_gray(151)
        };
        let galley = crate::i18n::widget(ui, self.label.color(color)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            FontId::proportional(if self.small { 12. } else { 13. }),
        );
        let height = if self.small { 20. } else { 26. };
        let icon_width = if self.icon.is_some() { height - 2. } else { 0. };
        let size = Vec2::new((galley.size().x + 16. + icon_width).max(height), height);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), galley.text())
        });
        if ui.is_rect_visible(rect) {
            let pressed = response.is_pointer_button_down_on() || self.selected;
            let (top, bottom) = if self.primary && ui.is_enabled() {
                if pressed {
                    (
                        Color32::from_rgb(0, 82, 119),
                        Color32::from_rgb(0, 151, 204),
                    )
                } else if response.hovered() {
                    (
                        Color32::from_rgb(0, 176, 235),
                        Color32::from_rgb(0, 103, 151),
                    )
                } else {
                    (
                        Color32::from_rgb(0, 157, 215),
                        Color32::from_rgb(0, 82, 127),
                    )
                }
            } else if pressed {
                (Color32::from_gray(180), Color32::from_gray(228))
            } else if response.hovered() && ui.is_enabled() {
                (Color32::WHITE, Color32::from_gray(219))
            } else {
                (Color32::from_gray(252), Color32::from_gray(202))
            };
            if let Some(fill) = self.fill {
                ui.painter().rect_filled(rect, 3., fill);
            } else {
                gradient(ui.painter(), rect, 3., top, bottom);
            }
            ui.painter().rect_stroke(
                rect.shrink(0.5),
                3.,
                self.stroke.unwrap_or(Stroke::new(1_f32, BORDER)),
                egui::StrokeKind::Inside,
            );
            if self.fill.is_none() {
                ui.painter().line_segment(
                    [
                        rect.left_top() + Vec2::new(3., 1.5),
                        rect.right_top() + Vec2::new(-3., 1.5),
                    ],
                    Stroke::new(
                        1_f32,
                        Color32::from_white_alpha(if pressed { 70 } else { 210 }),
                    ),
                );
            }
            if response.has_focus() {
                ui.painter().rect_stroke(
                    rect.expand(1.),
                    4.,
                    Stroke::new(1_f32, Color32::from_rgb(0, 153, 204)),
                    egui::StrokeKind::Outside,
                );
            }
            let offset = if pressed { Vec2::splat(1.) } else { Vec2::ZERO };
            if let Some(icon) = self.icon {
                let tile = Rect::from_min_size(
                    rect.min + Vec2::splat(1.),
                    Vec2::new(icon_width, height - 2.),
                );
                match icon {
                    Glyph::Import => {}
                    _ => gradient(
                        ui.painter(),
                        tile,
                        2.,
                        Color32::from_rgb(93, 179, 26),
                        Color32::from_rgb(0, 126, 0),
                    ),
                }
                let p = ui.painter();
                let c = if matches!(icon, Glyph::Import) {
                    TEXT
                } else {
                    Color32::WHITE
                };
                let center = tile.center() + offset;
                match icon {
                    Glyph::Add => {
                        p.line_segment(
                            [center - Vec2::new(5., 0.), center + Vec2::new(5., 0.)],
                            Stroke::new(2_f32, c),
                        );
                        p.line_segment(
                            [center - Vec2::new(0., 5.), center + Vec2::new(0., 5.)],
                            Stroke::new(2_f32, c),
                        );
                    }
                    Glyph::Transfer => {
                        crate::icons::draw(
                            p,
                            Rect::from_center_size(center, Vec2::splat(17.)),
                            3,
                            c,
                        );
                    }
                    Glyph::Import => {
                        p.line_segment(
                            [center - Vec2::new(0., 7.), center + Vec2::new(0., 4.)],
                            Stroke::new(2_f32, c),
                        );
                        p.add(egui::Shape::line(
                            vec![
                                center + Vec2::new(-4., 1.),
                                center + Vec2::new(0., 5.),
                                center + Vec2::new(4., 1.),
                            ],
                            Stroke::new(2_f32, c),
                        ));
                    }
                    Glyph::Check => {
                        p.add(egui::Shape::line(
                            vec![
                                center + Vec2::new(-5., 0.),
                                center + Vec2::new(-1., 4.),
                                center + Vec2::new(6., -4.),
                            ],
                            Stroke::new(2_f32, c),
                        ));
                    }
                }
            }
            let pos = egui::pos2(
                rect.left() + icon_width + (rect.width() - icon_width - galley.size().x) / 2.,
                rect.center().y - galley.size().y / 2.,
            ) + offset;
            ui.painter().galley(pos, galley, color);
        }
        response
    }
}
pub fn button(ui: &mut Ui, label: impl Into<WidgetText>) -> Response {
    ui.add(Button::new(label))
}
pub fn small_button(ui: &mut Ui, label: impl Into<WidgetText>) -> Response {
    ui.add(Button::new(label).small())
}
