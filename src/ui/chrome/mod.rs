use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Screen {
    Preferences,
    Properties,
    New,
    Open,
}
#[derive(Clone)]
pub(super) enum Overlay {
    Export,
    Fresh,
    Wifi,
    Payees,
    About,
    Diagnostics,
    Technical,
    Help(String),
    SavedVersion,
    LoadVersion(i64),
}
#[derive(Default)]
pub(super) struct Chrome {
    pub screen: Option<Screen>,
    pub overlay: Option<Overlay>,
    pub name: String,
    pub currency: String,
    pub error: String,
    pub payee: String,
    pub renamed_payee: String,
    pub rename_pattern: String,
    pub versions_dirty: bool,
    pub print_preview: bool,
    pub print_zoom: f32,
    pub report_print_job: Option<std::sync::mpsc::Receiver<Result<()>>>,
}

pub(super) fn modal(_ctx: &egui::Context, id: &str) -> egui::Modal {
    egui::Modal::new(egui::Id::new(id))
        .backdrop_color(Color32::from_rgba_unmultiplied(31, 63, 76, 160))
        .frame(
            Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1_f32, Color32::from_gray(178)))
                .corner_radius(3)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 10],
                    blur: 24,
                    spread: 0,
                    color: Color32::from_black_alpha(110),
                }),
        )
}
pub(super) fn header(ui: &mut egui::Ui, title: &str, width: f32) {
    ui.set_width(width);
    Frame::new()
        .fill(Color32::from_gray(232))
        .inner_margin(Margin::symmetric(18, 9))
        .show(ui, |ui| {
            ui.set_width(width - 36.);
            ui.l_label(RichText::new(title).size(16.).color(INK));
        });
    let rect = ui.min_rect();
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1_f32, Color32::from_gray(195)),
    );
}
fn account_header(ui: &mut egui::Ui, width: f32) {
    ui.set_width(width);
    Frame::new()
        .fill(Color32::from_gray(232))
        .inner_margin(Margin::symmetric(20, 11))
        .show(ui, |ui| {
            ui.set_width(width - 40.);
            ui.l_label(RichText::new("Создать новый счёт").size(18.).color(INK));
        });
    let rect = ui.min_rect();
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1_f32, Color32::from_gray(195)),
    );
}

fn account_radio(ui: &mut egui::Ui, check: &mut bool, chosen: &mut bool, value: bool, text: &str) {
    let translated = i18n::tr(ui.ctx(), text);
    let (title, description) = translated.split_once(" — ").unwrap_or((&translated, ""));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.;
        if ui.radio(*chosen && *check == value, "").clicked() {
            *check = value;
            *chosen = true;
        }
        ui.add_space(4.);
        let title_response = ui.add(
            egui::Label::new(RichText::new(title).font(crate::palette::bold(13.)))
                .sense(egui::Sense::click()),
        );
        let description_response =
            ui.add(egui::Label::new(format!(" - {description}")).sense(egui::Sense::click()));
        if title_response.clicked() || description_response.clicked() {
            *check = value;
            *chosen = true;
        }
    });
}
pub(super) fn footer(ui: &mut egui::Ui, width: f32, contents: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(Color32::from_gray(232))
        .inner_margin(Margin::symmetric(18, 10))
        .show(ui, |ui| {
            ui.set_width(width - 36.);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), contents);
            });
        });
}

pub(super) fn body(ui: &mut egui::Ui, width: f32, contents: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .inner_margin(Margin::symmetric(28, 24))
        .show(ui, |ui| {
            ui.set_width(width - 56.);
            ui.visuals_mut().widgets.inactive.bg_stroke =
                Stroke::new(1_f32, Color32::from_gray(170));
            let height = (ui.ctx().screen_rect().height() - 180.).max(120.);
            egui::ScrollArea::vertical()
                .id_salt("modal_body")
                .max_height(height)
                .show(ui, contents);
        });
}
fn section(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    let response = Frame::new()
        .inner_margin(Margin {
            left: 12,
            right: 0,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.l_label(
                RichText::new(title)
                    .font(crate::palette::bold(17.))
                    .color(Color32::WHITE),
            );
            ui.add_space(4.);
            contents(ui);
        });
    ui.painter().vline(
        response.response.rect.left(),
        response.response.rect.y_range(),
        Stroke::new(1.5_f32, Color32::from_rgb(118, 210, 234)),
    );
    ui.add_space(28.);
}

mod account_form;
mod budgets;
mod commands;
mod dialogs;
mod payees;
mod printing;
mod screens;

#[cfg(test)]
mod tests;
