use super::*;

pub fn register_menu_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(242, 249, 250))
        .stroke(Stroke::new(0.5_f32, Color32::from_rgb(158, 179, 184)))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(5, 6))
        .shadow(egui::epaint::Shadow {
            offset: [0, 7],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(45),
        })
}

pub fn register_menu_style(ui: &mut Ui) {
    ui.set_min_width(184.);
    ui.spacing_mut().item_spacing.y = 0.;
    ui.spacing_mut().button_padding = Vec2::new(11., 5.);
    ui.spacing_mut().interact_size.y = 24.;
    let visuals = ui.visuals_mut();
    visuals.override_text_color = None;
    visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(35, 48, 50);
    visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(210, 221, 223);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(35, 48, 50);
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    let blue = Color32::from_rgb(82, 151, 249);
    for style in [
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        style.weak_bg_fill = blue;
        style.bg_fill = blue;
        style.bg_stroke = Stroke::NONE;
        style.fg_stroke.color = Color32::WHITE;
        style.corner_radius = 5.into();
        style.expansion = 0.;
    }
}
