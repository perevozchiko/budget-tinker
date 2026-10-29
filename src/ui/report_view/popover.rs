use super::*;

pub(super) const BLUE: Color32 = Color32::from_rgb(0, 145, 207);

pub(super) fn trigger(ui: &mut egui::Ui, id: &str, label: &str, open: bool) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), crate::palette::bold(13.), BLUE);
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 19., 21.), egui::Sense::hover());
    let response = ui.interact(rect, ui.make_persistent_id(id), egui::Sense::click());
    let highlighted = ui.is_enabled() && (open || response.hovered());
    if highlighted {
        ui.painter().rect_filled(rect, 11., BLUE);
    }
    ui.painter().galley_with_override_text_color(
        rect.left_center() + Vec2::new(5., -galley.size().y / 2.),
        galley,
        if !ui.is_enabled() {
            Color32::GRAY
        } else if highlighted {
            Color32::WHITE
        } else {
            BLUE
        },
    );
    if ui.is_enabled() {
        let c = rect.right_center() - Vec2::new(7., 0.);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                c + Vec2::new(-3., -1.5),
                c + Vec2::new(3., -1.5),
                c + Vec2::new(0., 2.5),
            ],
            if highlighted { Color32::WHITE } else { BLUE },
            Stroke::NONE,
        ));
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}

pub(super) fn frame() -> Frame {
    Frame::new()
        .fill(Color32::from_gray(245))
        .stroke(Stroke::new(1_f32, Color32::from_gray(174)))
        .corner_radius(4)
        .inner_margin(egui::Margin::ZERO)
        .shadow(egui::epaint::Shadow {
            offset: [1, 5],
            blur: 10,
            spread: 0,
            color: Color32::from_black_alpha(100),
        })
}

pub(super) fn band(
    ui: &mut egui::Ui,
    width: f32,
    fill: Color32,
    padding: i8,
    content: impl FnOnce(&mut egui::Ui),
) {
    Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::same(padding))
        .show(ui, |ui| {
            ui.set_width(width - 2. * f32::from(padding));
            ui.with_layout(Layout::top_down(Align::Min), content);
        });
}

pub(super) fn divider(ui: &mut egui::Ui, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1_f32, Color32::from_gray(177)),
    );
}

pub(super) fn header(ui: &mut egui::Ui, width: f32, title: &str) {
    ui.set_width(width);
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    ui.spacing_mut().indent = 12.;
    ui.spacing_mut().interact_size.y = 19.;
    ui.visuals_mut().indent_has_left_vline = false;
    ui.spacing_mut().icon_width = 13.;
    ui.spacing_mut().icon_width_inner = 9.;
    ui.spacing_mut().icon_spacing = 6.;
    ui.visuals_mut().selection.bg_fill = Color32::from_rgb(149, 214, 238);
    ui.visuals_mut().selection.stroke.color = Color32::from_gray(35);
    band(ui, width, Color32::from_gray(232), 7, |ui| {
        ui.label(RichText::new(title).size(16.).color(classic::TEXT));
    });
    divider(ui, width);
}

pub(super) fn link(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).font(crate::palette::bold(13.)).color(
            if !ui.is_enabled() {
                Color32::GRAY
            } else if selected {
                Color32::WHITE
            } else {
                BLUE
            },
        ))
        .fill(if selected {
            Color32::from_rgb(65, 181, 222)
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .corner_radius(11.),
    )
}

pub(super) fn checkbox(
    ui: &mut egui::Ui,
    checked: &mut bool,
    label: RichText,
    partial: bool,
) -> egui::Response {
    let response = ui.add(egui::Checkbox::new(checked, label).indeterminate(partial));
    let rect = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 6.5, response.rect.center().y),
        Vec2::splat(13.),
    );
    let blue = (*checked || partial) && ui.is_enabled();
    classic::gradient(
        ui.painter(),
        rect,
        1.,
        Color32::from_gray(251),
        if blue {
            Color32::from_rgb(146, 210, 233)
        } else {
            Color32::from_gray(200)
        },
    );
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        1.,
        Stroke::new(
            1_f32,
            if response.has_focus() {
                BLUE
            } else {
                Color32::from_gray(134)
            },
        ),
        egui::StrokeKind::Inside,
    );
    let c = rect.center();
    let ink = Stroke::new(
        1.7_f32,
        if ui.is_enabled() {
            Color32::from_gray(35)
        } else {
            Color32::GRAY
        },
    );
    if partial {
        ui.painter()
            .line_segment([c - Vec2::new(3.5, 0.), c + Vec2::new(3.5, 0.)], ink);
    } else if *checked {
        ui.painter().add(egui::Shape::line(
            vec![
                c + Vec2::new(-4., 0.),
                c + Vec2::new(-1., 3.),
                c + Vec2::new(4., -4.),
            ],
            ink,
        ));
    }
    response
}

/// Returns true only on Done. Closing the panel otherwise leaves the applied filter intact.
pub(super) fn footer(ui: &mut egui::Ui, width: f32, valid: bool) -> bool {
    let mut apply = false;
    divider(ui, width);
    band(ui, width, Color32::from_gray(232), 6, |ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.;
            if crate::register_timeframe::action_button(ui, "Done", true, valid).clicked() {
                apply = true;
                ui.close();
            }
            if crate::register_timeframe::action_button(ui, "Cancel", false, true).clicked() {
                ui.close();
            }
        });
    });
    apply
}

pub(super) fn pointer(ui: &egui::Ui, trigger: &egui::Response, rect: egui::Rect) {
    let below = rect.center().y >= trigger.rect.center().y;
    let y = if below { rect.top() } else { rect.bottom() };
    let x = trigger
        .rect
        .center()
        .x
        .clamp(rect.left() + 18., rect.right() - 18.);
    let tip = egui::pos2(x, y + if below { -16. } else { 16. });
    let a = egui::pos2(x - 16., y + if below { 1. } else { -1. });
    let b = egui::pos2(x + 16., a.y);
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        trigger.id.with("pointer"),
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![a, tip, b],
        Color32::from_gray(232),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::line(
        vec![a, tip, b],
        Stroke::new(1_f32, Color32::from_gray(174)),
    ));
}
