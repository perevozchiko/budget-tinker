//! Category-column controls. Only primary clicks edit; disclosure is a separate target.
use crate::{classic, i18n, palette};
use eframe::egui::{self, Color32, FontId, Rect, Response, Stroke, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupAction {
    None,
    Toggle,
    Edit,
    Add,
    Note,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderAction {
    None,
    Add,
    Expand,
    Collapse,
}

fn add_button(ui: &mut Ui, rect: Rect, id: egui::Id) -> Response {
    let response = ui
        .interact(rect, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let pressed = response.is_pointer_button_down_on();
    classic::gradient(
        ui.painter(),
        rect,
        1.,
        if pressed {
            Color32::from_gray(202)
        } else {
            Color32::WHITE
        },
        if pressed {
            Color32::from_gray(233)
        } else {
            Color32::from_gray(206)
        },
    );
    ui.painter().rect_stroke(
        rect,
        1.,
        Stroke::new(0.8_f32, classic::BORDER),
        egui::StrokeKind::Inside,
    );
    let center = rect.center()
        + if pressed {
            Vec2::splat(0.5)
        } else {
            Vec2::ZERO
        };
    let stroke = Stroke::new(1.8_f32, classic::TEXT);
    ui.painter().line_segment(
        [center - Vec2::new(4., 0.), center + Vec2::new(4., 0.)],
        stroke,
    );
    ui.painter().line_segment(
        [center - Vec2::new(0., 4.), center + Vec2::new(0., 4.)],
        stroke,
    );
    response
}

pub fn header(ui: &mut Ui, rect: Rect) -> HeaderAction {
    let label = i18n::tr(ui.ctx(), "Категории");
    let text = ui
        .painter()
        .layout_no_wrap(label, FontId::proportional(14.), classic::TEXT);
    let center_y = rect.bottom() - 13.;
    let text_width = text.size().x;
    ui.painter().galley(
        egui::pos2(rect.left() + 7., center_y - text.size().y / 2.),
        text,
        classic::TEXT,
    );
    let add_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 7. + text_width + 13., center_y),
        Vec2::splat(16.),
    );
    if add_button(ui, add_rect, ui.id().with("add_category_header")).clicked() {
        return HeaderAction::Add;
    }
    let icon = |offset| {
        Rect::from_center_size(
            egui::pos2(rect.right() - offset, center_y),
            Vec2::new(21., 22.),
        )
    };
    if classic::category_disclosure(ui, icon(34.), true)
        .on_hover_text("Expand all categories")
        .clicked()
    {
        return HeaderAction::Expand;
    }
    if classic::category_disclosure(ui, icon(11.), false)
        .on_hover_text("Collapse all categories")
        .clicked()
    {
        return HeaderAction::Collapse;
    }
    HeaderAction::None
}

pub fn group_row(
    ui: &mut Ui,
    rect: Rect,
    name: &str,
    collapsed: bool,
    has_note: bool,
    hidden: bool,
) -> GroupAction {
    let id = ui.id().with(("master_category", name));
    let hovered = ui.rect_contains_pointer(rect);
    let arrow_rect = Rect::from_min_max(rect.min, egui::pos2(rect.left() + 20., rect.bottom()));
    let arrow = ui.interact(arrow_rect, id.with("disclosure"), egui::Sense::click());
    let center = egui::pos2(rect.left() + 10., rect.center().y);
    let points = if collapsed {
        vec![
            center + Vec2::new(-2., -3.),
            center + Vec2::new(-2., 3.),
            center + Vec2::new(3., 0.),
        ]
    } else {
        vec![
            center + Vec2::new(-3., -2.),
            center + Vec2::new(3., -2.),
            center + Vec2::new(0., 2.),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        classic::TEXT,
        Stroke::NONE,
    ));
    let text =
        ui.painter()
            .layout_no_wrap(i18n::tr(ui.ctx(), name), palette::bold(13.), classic::TEXT);
    let note_x = (rect.left() + 21. + text.size().x).min(rect.right() - 34.);
    let show_note = !hidden && (has_note || hovered);
    let label_rect = Rect::from_min_max(
        egui::pos2(rect.left() + 20., rect.top()),
        egui::pos2(
            if show_note {
                note_x
            } else {
                rect.right() - 23.
            },
            rect.bottom(),
        ),
    );
    let label = ui
        .interact(label_rect, id.with("name"), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let text_pos = egui::pos2(rect.left() + 20., rect.center().y - text.size().y / 2.);
    ui.painter()
        .with_clip_rect(label_rect)
        .galley(text_pos, text.clone(), classic::TEXT);
    if label.hovered() && !hidden {
        ui.painter().hline(
            text_pos.x..=(text_pos.x + text.size().x).min(label_rect.right()),
            text_pos.y + text.size().y - 1.,
            Stroke::new(0.8_f32, classic::TEXT),
        );
    }
    if arrow.clicked() {
        return GroupAction::Toggle;
    }
    if label.clicked() {
        return if hidden {
            GroupAction::Toggle
        } else {
            GroupAction::Edit
        };
    }
    if show_note {
        let icon = Rect::from_center_size(
            egui::pos2(note_x + 6., rect.center().y),
            Vec2::new(11., 14.),
        );
        ui.painter().rect_stroke(
            icon,
            0.,
            Stroke::new(0.8_f32, classic::TEXT),
            egui::StrokeKind::Inside,
        );
        for offset in [4., 7., 10.] {
            ui.painter().hline(
                icon.left() + 2. ..=icon.right() - 2.,
                icon.top() + offset,
                Stroke::new(0.8_f32, classic::TEXT),
            );
        }
        if ui
            .interact(icon, id.with("note"), egui::Sense::click())
            .on_hover_text("Master Category Note")
            .clicked()
        {
            return GroupAction::Note;
        }
    }
    if hovered && !hidden {
        let add = Rect::from_center_size(
            egui::pos2(rect.right() - 11., rect.center().y),
            Vec2::splat(16.),
        );
        if add_button(ui, add, id.with("add"))
            .on_hover_text("Add a sub-category")
            .clicked()
        {
            return GroupAction::Add;
        }
    }
    GroupAction::None
}

/// The hidden summary stays collapsed; restoration happens in this popup.
pub enum UnhideAction {
    Category(String),
    Group(String),
    All,
}

pub fn hidden_row(
    ui: &mut Ui,
    rect: Rect,
    budget: &budgettinker::model::Budget,
) -> Option<UnhideAction> {
    let title = i18n::tr(ui.ctx(), "Скрытые категории");
    ui.painter().text(
        rect.left_center() + Vec2::new(8., 0.),
        egui::Align2::LEFT_CENTER,
        title,
        palette::bold(12.),
        classic::TEXT,
    );
    let button_rect = Rect::from_center_size(
        egui::pos2(rect.right() - 34., rect.center().y),
        Vec2::new(62., 19.),
    );
    let button = ui.put(
        button_rect,
        classic::Button::new(i18n::tr(ui.ctx(), "Показать")).small(),
    );
    let mut action = None;
    let popup = egui::Popup::menu(&button)
        .width(375.)
        .gap(14.)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_gray(128)))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::ZERO)
                .shadow(egui::epaint::Shadow {
                    offset: [1, 6],
                    blur: 12,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(80),
                }),
        )
        .show(|ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            ui.spacing_mut().interact_size.y = 20.;
            egui::Frame::new()
                .fill(egui::Color32::from_gray(232))
                .corner_radius(egui::CornerRadius {
                    nw: 8,
                    ne: 8,
                    sw: 0,
                    se: 0,
                })
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new(i18n::tr(
                            ui.ctx(),
                            "Нажмите на категорию, чтобы показать её",
                        ))
                        .size(14.),
                    );
                });
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(360.)
                .show(ui, |ui| {
                    ui.add_space(5.);
                    let row = |ui: &mut Ui, label: String, indent: f32, strong: bool| {
                        ui.horizontal(|ui| {
                            ui.add_space(indent + 12.);
                            let text = egui::RichText::new(label)
                                .size(14.)
                                .color(egui::Color32::from_rgb(0, 94, 255))
                                .underline();
                            let text = if strong { text.strong() } else { text };
                            ui.link(text).clicked()
                        })
                        .inner
                    };
                    for group in budget.group_names() {
                        let hidden_group = budget
                            .category_groups
                            .iter()
                            .any(|g| g.name == group && g.hidden);
                        let categories: Vec<_> = budget
                            .categories
                            .iter()
                            .filter(|c| c.group == group && c.hidden)
                            .collect();
                        if categories.is_empty() && !hidden_group {
                            continue;
                        }
                        if row(ui, format!("{}:", i18n::tr(ui.ctx(), &group)), 0., true) {
                            action = Some(UnhideAction::Group(group.clone()));
                        }
                        for category in categories {
                            if row(ui, i18n::tr(ui.ctx(), &category.name), 20., false) {
                                action = Some(UnhideAction::Category(category.id.clone()));
                            }
                        }
                    }
                    ui.add_space(1.);
                });
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(29, 0))
                .show(ui, |ui| ui.separator());
            ui.add_space(16.);
            let unhide_all = ui.horizontal(|ui| {
                ui.add_space(12.);
                ui.link(
                    egui::RichText::new(i18n::tr(ui.ctx(), "Показать все"))
                        .size(14.)
                        .strong()
                        .color(egui::Color32::from_rgb(0, 94, 255))
                        .underline(),
                )
            });
            if unhide_all.inner.clicked() {
                action = Some(UnhideAction::All);
            }
            ui.add_space(9.);
            ui.separator();
            egui::Frame::new()
                .fill(egui::Color32::from_gray(232))
                .corner_radius(egui::CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 8,
                    se: 8,
                })
                .show(ui, |ui| ui.add_space(22.));
            if action.is_some() {
                ui.close();
            }
        });
    if let Some(popup) = popup {
        let rect = popup.response.rect;
        let center_x = button
            .rect
            .center()
            .x
            .clamp(rect.left() + 18., rect.right() - 18.);
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            ui.id().with("hidden_categories_pointer"),
        ));
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(center_x - 15., rect.top() + 2.),
                egui::pos2(center_x + 15., rect.top() + 2.),
                egui::pos2(button.rect.center().x, button.rect.bottom() + 1.),
            ],
            egui::Color32::from_gray(232),
            egui::Stroke::new(1.0_f32, egui::Color32::from_gray(128)),
        ));
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.families.insert(
            egui::FontFamily::Name("arial-bold".into()),
            fonts.families[&egui::FontFamily::Proportional].clone(),
        );
        ctx.set_fonts(fonts);
        ctx
    }
    fn draw(ctx: &egui::Context, events: Vec<egui::Event>, hidden: bool) -> GroupAction {
        let mut action = GroupAction::None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(500., 400.))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    action = group_row(
                        ui,
                        Rect::from_min_size(egui::pos2(20., 20.), Vec2::new(184., 23.)),
                        "Monthly Bills",
                        false,
                        false,
                        hidden,
                    );
                });
            },
        );
        action
    }
    fn click(point: egui::Pos2, button: egui::PointerButton, hidden: bool) -> GroupAction {
        let ctx = context();
        draw(&ctx, vec![], hidden);
        draw(&ctx, vec![egui::Event::PointerMoved(point)], hidden);
        for pressed in [true, false] {
            let action = draw(
                &ctx,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                hidden,
            );
            if !pressed {
                return action;
            }
        }
        unreachable!()
    }
    #[test]
    fn one_primary_click_on_master_category_opens_editor() {
        assert_eq!(
            click(egui::pos2(65., 31.), egui::PointerButton::Primary, false),
            GroupAction::Edit
        );
    }
    #[test]
    fn disclosure_only_toggles_and_does_not_open_editor() {
        assert_eq!(
            click(egui::pos2(30., 31.), egui::PointerButton::Primary, false),
            GroupAction::Toggle
        );
    }
    #[test]
    fn secondary_click_does_nothing_on_name_arrow_or_add_button() {
        for x in [30., 65., 193.] {
            assert_eq!(
                click(egui::pos2(x, 31.), egui::PointerButton::Secondary, false),
                GroupAction::None
            );
        }
    }
    #[test]
    fn hover_add_button_adds_to_the_master_category() {
        assert_eq!(
            click(egui::pos2(193., 31.), egui::PointerButton::Primary, false),
            GroupAction::Add
        );
    }
    #[test]
    fn hidden_categories_header_toggles_instead_of_editing_a_fake_group() {
        assert_eq!(
            click(egui::pos2(65., 31.), egui::PointerButton::Primary, true),
            GroupAction::Toggle
        );
    }
}
