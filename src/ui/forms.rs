use super::*;

const ACCOUNT_TYPES: &[(&str, &str)] = &[
    ("checking", "Текущий счёт"),
    ("savings", "Сберегательный счёт"),
    ("credit", "Кредитная карта"),
    ("cash", "Наличные"),
    ("line_of_credit", "Кредитная линия"),
    ("paypal", "PayPal"),
    ("merchant", "Торговый счёт"),
    ("investment", "Инвестиционный счёт"),
    ("mortgage", "Ипотека"),
    ("asset", "Другой актив (дом, машина и т. п.)"),
    ("liability", "Другой кредит или обязательство"),
];

pub(super) fn account_type(ui: &mut egui::Ui, value: &mut String, width: f32) -> bool {
    let before = value.clone();
    egui::ComboBox::from_id_salt("account_type")
        .width(width)
        .selected_text(
            egui::RichText::new(i18n::tr(
                ui.ctx(),
                ACCOUNT_TYPES
                    .iter()
                    .find(|(id, _)| *id == value)
                    .map(|(_, name)| *name)
                    .unwrap_or("Выберите тип счёта…"),
            ))
            .font(crate::palette::bold(13.)),
        )
        .show_ui(ui, |ui| {
            for (code, label) in ACCOUNT_TYPES {
                ui.l_selectable_value(value, (*code).to_owned(), *label);
            }
        });
    before != *value
}

fn first_field(ui: &mut egui::Ui, value: &mut String, focus: &mut bool, width: f32) {
    first_field_style(ui, value, focus, width, false);
}
fn first_field_style(
    ui: &mut egui::Ui,
    value: &mut String,
    focus: &mut bool,
    width: f32,
    compact: bool,
) {
    let response = ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(width)
            .font(if compact {
                crate::palette::bold(16.)
            } else {
                egui::FontId::proportional(13.)
            })
            .margin(if compact { 4 } else { 7 }),
    );
    if *focus {
        response.request_focus();
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(value.chars().count()),
                )));
            state.store(ui.ctx(), response.id);
        }
        *focus = false;
    }
}

fn action_link(ui: &mut egui::Ui, label: &str, destructive: bool) -> bool {
    let color = if destructive {
        Color32::from_rgb(220, 0, 0)
    } else {
        Color32::from_rgb(0, 0, 238)
    };
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.;
        let (icon, response) = ui.allocate_exact_size(Vec2::splat(14.), egui::Sense::click());
        let (top, bottom) = if destructive {
            (
                Color32::from_rgb(255, 91, 74),
                Color32::from_rgb(205, 20, 13),
            )
        } else {
            (
                Color32::from_rgb(40, 211, 248),
                Color32::from_rgb(0, 136, 186),
            )
        };
        classic::gradient(ui.painter(), icon, 7., top, bottom);
        ui.painter()
            .circle_stroke(icon.center(), 6.5, Stroke::new(0.5_f32, bottom));
        if destructive {
            ui.painter().line_segment(
                [
                    icon.center() - Vec2::new(3.5, 0.),
                    icon.center() + Vec2::new(3.5, 0.),
                ],
                Stroke::new(1.5_f32, Color32::WHITE),
            );
        } else {
            let center = icon.center();
            let points = vec![
                center + Vec2::new(-4., 0.),
                center + Vec2::new(-2., -2.5),
                center + Vec2::new(2., -2.5),
                center + Vec2::new(4., 0.),
                center + Vec2::new(2., 2.5),
                center + Vec2::new(-2., 2.5),
            ];
            ui.painter().add(egui::Shape::closed_line(
                points,
                Stroke::new(0.8_f32, Color32::WHITE),
            ));
            ui.painter().circle_filled(center, 1.5, Color32::WHITE);
        }
        clicked = response.clicked();
        clicked |= ui
            .add(
                egui::Label::new(
                    RichText::new(i18n::tr(ui.ctx(), label))
                        .size(14.)
                        .underline()
                        .color(color),
                )
                .sense(egui::Sense::click()),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked();
    });
    clicked
}

fn detail_header(ui: &mut egui::Ui, title: &str, width: f32, compact: bool) {
    if !compact {
        chrome::header(ui, title, width);
        return;
    }
    let response = Frame::new()
        .fill(Color32::from_gray(232))
        .inner_margin(Margin::symmetric(20, 11))
        .show(ui, |ui| {
            ui.set_width(width - 40.);
            ui.l_label(RichText::new(title).size(18.).color(INK));
        });
    ui.painter().hline(
        response.response.rect.x_range(),
        response.response.rect.bottom(),
        Stroke::new(1_f32, Color32::from_gray(178)),
    );
}
fn detail_body(ui: &mut egui::Ui, width: f32, compact: bool, contents: impl FnOnce(&mut egui::Ui)) {
    if !compact {
        chrome::body(ui, width, contents);
        return;
    }
    Frame::new()
        .fill(Color32::WHITE)
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(width - 40.);
            ui.visuals_mut().widgets.inactive.bg_stroke =
                Stroke::new(1_f32, Color32::from_gray(170));
            ui.visuals_mut().widgets.active.bg_stroke =
                Stroke::new(1_f32, Color32::from_rgb(0, 174, 230));
            egui::ScrollArea::vertical()
                .id_salt("category_popup_body")
                .max_height((ui.ctx().screen_rect().height() - 160.).max(100.))
                .show(ui, contents);
        });
}
fn detail_footer(
    ui: &mut egui::Ui,
    width: f32,
    compact: bool,
    contents: impl FnOnce(&mut egui::Ui),
) {
    if !compact {
        chrome::footer(ui, width, contents);
        return;
    }
    let response = Frame::new()
        .fill(Color32::from_gray(232))
        .inner_margin(Margin::symmetric(20, 12))
        .show(ui, |ui| {
            ui.set_width(width - 40.);
            ui.spacing_mut().item_spacing.x = 12.;
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), contents);
            });
        });
    ui.painter().hline(
        response.response.rect.x_range(),
        response.response.rect.top(),
        Stroke::new(1_f32, Color32::from_gray(178)),
    );
}
fn category_dialog_button(ui: &mut egui::Ui, label: &str, primary: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(if primary { 52. } else { 60. }, 27.),
        egui::Sense::click(),
    );
    let (top, bottom) = if primary {
        (
            Color32::from_rgb(0, 151, 208),
            Color32::from_rgb(0, 67, 105),
        )
    } else {
        (Color32::from_gray(253), Color32::from_gray(205))
    };
    classic::gradient(
        ui.painter(),
        rect,
        4.,
        if response.is_pointer_button_down_on() {
            bottom
        } else {
            top
        },
        if response.is_pointer_button_down_on() {
            top
        } else {
            bottom
        },
    );
    ui.painter().rect_stroke(
        rect,
        4.,
        Stroke::new(1_f32, classic::BORDER),
        egui::StrokeKind::Inside,
    );
    ui.painter().hline(
        rect.left() + 3. ..=rect.right() - 3.,
        rect.top() + 1.5,
        Stroke::new(0.8_f32, Color32::from_white_alpha(160)),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        crate::palette::bold(14.),
        if primary { Color32::WHITE } else { INK },
    );
    response
}

fn popup_pointer(ctx: &egui::Context, pos: egui::Pos2, color: Color32) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("detail_popup_pointer"),
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(pos.x + 38., pos.y + 1.),
            egui::pos2(pos.x + 64., pos.y + 1.),
            egui::pos2(pos.x + 51., pos.y - 12.),
        ],
        color,
        Stroke::new(1_f32, Color32::from_gray(178)),
    ));
}

impl App {
    pub(super) fn detail_form(&mut self, ctx: &egui::Context, mut d: Dialog) {
        if matches!(d.kind, DialogKind::Note(_)) {
            self.note_form(ctx, d);
            return;
        }
        let compact = matches!(
            d.kind,
            DialogKind::Category
                | DialogKind::NewGroup
                | DialogKind::EditCategory(_)
                | DialogKind::EditGroup(_)
                | DialogKind::Overspending { .. }
        );
        let width = match d.kind {
            DialogKind::Reconcile(_) => 425.,
            DialogKind::AdjustBalance(_) => 545.,
            DialogKind::EditGroup(_) => 425.,
            DialogKind::Overspending { .. } => 470.,
            _ if compact => 375.,
            _ => 490.,
        };
        let title = match &d.kind {
            DialogKind::AccountSettings(_) => "Свойства счёта",
            DialogKind::AdjustBalance(a) => {
                let name = &self.b.account(a).expect("existing account").name;
                &format!("Adjust balance for '{name}'")
            }
            DialogKind::Reconcile(_) => "Сверка счёта",
            DialogKind::Category => "Add new category",
            DialogKind::NewGroup => "Add new master category",
            DialogKind::EditCategory(_) => "Изменить категорию",
            DialogKind::EditGroup(_) => "Изменить группу",
            DialogKind::Overspending { .. } => "Настройки перерасхода",
            DialogKind::Note(_) => "Заметка",
            _ => unreachable!(),
        };
        let mut save = false;
        let mut cancel = false;
        let mut visibility_action = false;
        let mut next = None;
        let mut pointer = None;
        let mut modal = chrome::modal(ctx, "dialog");
        if compact {
            modal = modal.backdrop_color(Color32::TRANSPARENT);
            if let Some(anchor) = d.anchor {
                let screen = ctx.screen_rect();
                let x = anchor
                    .x
                    .min(screen.right() - width - 12.)
                    .max(screen.left() + 8.);
                let y = if anchor.y + 230. > screen.bottom() {
                    anchor.y - 260.
                } else {
                    anchor.y
                }
                .max(screen.top() + 8.);
                if y >= anchor.y {
                    pointer = Some(egui::pos2(x, y));
                }
                modal = modal.area(egui::Modal::default_area(egui::Id::new("dialog")).anchor(
                    egui::Align2::LEFT_TOP,
                    Vec2::new(x - screen.left(), y - screen.top()),
                ));
            }
        }
        let response = modal.show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.;
            detail_header(ui, title, width, compact);
            detail_body(ui, width, compact, |ui| {
                ui.spacing_mut().item_spacing.y = if compact { 6. } else { 9. };
                match &d.kind {
                    DialogKind::AccountSettings(a) => {
                        let account = self.b.account(a).expect("existing account");
                        ui.l_label(RichText::new("Название:").strong());
                        first_field(ui, &mut d.fields[0], &mut d.focus, width - 56.);
                        ui.l_label("Заметка:");
                        ui.add(egui::TextEdit::multiline(&mut d.fields[2]).desired_width(width - 56.).desired_rows(3));
                        ui.l_label("Тип:");
                        account_type(ui, &mut d.fields[1], 330.);
                        ui.add_space(4.);
                        ui.separator();
                        ui.l_checkbox(&mut d.second, "Включать в бюджет");
                        if d.second != account.on_budget {
                            ui.l_label(RichText::new("Изменение учёта счёта пересчитает бюджет и расходы по всей его истории.").size(12.).color(RED));
                        }
                        let balance = self.b.balance(a, None, false);
                        ui.horizontal(|ui| {
                            ui.l_label("Текущий остаток:");
                            ui.label(RichText::new(i18n::money(balance, &account.currency)).strong());
                        });
                        if ui.add_enabled(!account.closed, classic::Button::new("Скорректировать остаток…")).clicked() {
                            next = Some(DialogKind::AdjustBalance(a.clone()));
                        }
                        ui.separator();
                        ui.add_enabled_ui(account.closed || balance == 0, |ui| {
                            ui.l_checkbox(&mut d.check, "Закрыть счёт с нулевым остатком");
                        });
                        if !account.closed && balance != 0 {
                            ui.l_label(RichText::new("Для закрытия счёта его остаток должен быть нулевым").size(11.).color(Color32::GRAY));
                        }
                    }
                    DialogKind::Reconcile(a) => {
                        let account = self.b.account(a).expect("existing account");
                        ui.l_label("Enter the balance from your account website or paper statement to begin reconciling.");
                        ui.separator();
                        ui.l_label(RichText::new("What date do you want to reconcile against?").strong());
                        let (earlier, later) = d.fields.split_at_mut(8);
                        classic::account_date_field(ui, &mut earlier[1], &mut later[0]);
                        ui.l_label(RichText::new("What is your account balance on that date?").strong());
                        first_field(ui, &mut d.fields[0], &mut d.focus, width - 56.);
                        if valid_date(&d.fields[1]) {
                            let balance = self.b.balance(a, Some(&d.fields[1]), true);
                            ui.horizontal_wrapped(|ui| {
                                ui.l_label(RichText::new(format!("Your cleared balance in BudgetTinker is {}.", i18n::money(balance, &account.currency))).italics().color(Color32::GRAY));
                                if ui.link("Use this amount.").clicked() { d.fields[0] = plain(balance, &account.currency); }
                            });
                        }
                    }
                    DialogKind::AdjustBalance(a) => {
                        let account = self.b.account(a).expect("existing account");
                        ui.l_label(format!("Enter a new working balance for {} and BudgetTinker will enter an adjustment transaction so everything balances.", account.name));
                        ui.separator();
                        ui.l_label(RichText::new("New Working Balance:").strong());
                        first_field(ui, &mut d.fields[0], &mut d.focus, width - 56.);
                        ui.l_label(RichText::new(format!("The current working balance is {}", i18n::money(self.b.balance(a, None, false), &account.currency))).italics().color(Color32::GRAY));
                    }
                    DialogKind::Category | DialogKind::EditCategory(_) => {
                        let debt = matches!(&d.kind, DialogKind::EditCategory(id) if budgettinker::credit::debt_account(id).is_some());
                        if debt {
                            ui.label(RichText::new(&d.fields[1]).strong());
                            ui.label("Old debt from before this budget. Budget money here to reduce its negative balance. Record payments as transfers, and new purchases in their spending categories. Rename the account to rename this category.");
                        } else {
                            first_field_style(ui, &mut d.fields[1], &mut d.focus, width - 40., true);
                        }
                        if matches!(d.kind, DialogKind::EditCategory(_)) {
                            ui.add_space(10.);
                            if action_link(ui, if d.second { "Показать эту категорию" } else { "Скрыть эту категорию" }, false) { visibility_action = true; }
                            if !debt && action_link(ui, if d.check { "Отменить удаление категории" } else { "Удалить эту категорию" }, true) { d.check = !d.check; }
                            if d.check { ui.l_label(RichText::new("Удаление доступно, если нет операций, планов и ненулевых назначений.").size(12.).color(RED)); }
                        }
                    }
                    DialogKind::NewGroup => { first_field_style(ui, &mut d.fields[0], &mut d.focus, width - 40., true); }
                    DialogKind::EditGroup(group) => {
                        first_field_style(ui, &mut d.fields[0], &mut d.focus, width - 40., true);
                        ui.add_space(10.);
                        if action_link(ui, if d.second { "Показать эту группу" } else { "Скрыть эту группу" }, false) {
                            visibility_action = true;
                        }
                        if action_link(ui, if d.check { "Отменить удаление группы" } else { "Удалить эту группу" }, true) {
                            d.check = !d.check;
                        }
                        if d.check {
                            ui.l_label(RichText::new(format!("Категорий в группе: {}. Удаление доступно, если они не используются.", self.b.categories.iter().filter(|c| c.group == *group).count())).size(11.).color(RED));
                        }
                    }
                    DialogKind::Overspending { month, category } => {
                        let name = self.b.category_name(&Some(category.clone()));
                        ui.l_label(RichText::new(format!("{} · {}", i18n::tr(ctx, &name), month_name(ctx, month))).strong());
                        ui.l_label("Как учитывать перерасход по этой категории?");
                        ui.radio_value(&mut d.check, false, i18n::tr(ctx, "Вычесть из доступного бюджета следующего месяца"));
                        ui.radio_value(&mut d.check, true, i18n::tr(ctx, "Вычесть из остатка категории следующего месяца →"));
                        ui.l_label(RichText::new("Правило действует с выбранного месяца до следующего изменения.").size(11.).color(Color32::GRAY));
                    }
                    DialogKind::Note(key) => {
                        let subject = if let Some(id) = key.strip_prefix("category:") {
                            self.b.categories.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default()
                        } else if let Some(month) = key.strip_prefix("month:") {
                            month_name(ctx, month)
                        } else {
                            key.strip_prefix("group:").unwrap_or(key).to_owned()
                        };
                        ui.l_label(RichText::new(subject).font(crate::palette::bold(16.)).color(GREEN));
                        let input = ui.add(egui::TextEdit::multiline(&mut d.fields[0]).desired_width(width - 56.).desired_rows(8));
                        if d.focus { input.request_focus(); d.focus = false; }
                    }
                    _ => unreachable!(),
                }
                if !d.error.is_empty() { ui.l_label(RichText::new(&d.error).color(RED)); }
            });
            detail_footer(ui, width, compact, |ui| {
                let enabled = match &d.kind {
                    DialogKind::Reconcile(a) => valid_date(&d.fields[1]) && d.fields[1] <= today() && chrono::NaiveDate::parse_from_str(&d.fields[8], "%m/%d/%Y").is_ok_and(|date| date.format("%Y-%m-%d").to_string() == d.fields[1]) && amount(&d.fields[0], &self.b.account(a).unwrap().currency).is_ok(),
                    DialogKind::AdjustBalance(a) => valid_date(&d.fields[1]) && d.fields[1] <= today() && amount(&d.fields[0], &self.b.account(a).unwrap().currency).is_ok(),
                    DialogKind::Category | DialogKind::EditCategory(_) => !d.fields[0].trim().is_empty() && !d.fields[1].trim().is_empty(),
                    DialogKind::EditGroup(_) | DialogKind::NewGroup => !d.fields[0].trim().is_empty(),
                    DialogKind::AccountSettings(_) => !d.fields[0].trim().is_empty(),
                    _ => true,
                };
                let label = match d.kind { DialogKind::Reconcile(_) => "Begin Reconciliation", DialogKind::AdjustBalance(_) => "Adjust Balance", DialogKind::EditCategory(_) | DialogKind::EditGroup(_) => "Готово", _ => "Сохранить" };
                if compact {
                    if ui.add_enabled_ui(enabled, |ui| category_dialog_button(ui, "Done", true)).inner.clicked() { save = true; }
                    if category_dialog_button(ui, "Cancel", false).clicked() { cancel = true; }
                } else {
                    if ui.add_enabled(enabled, classic::Button::new(label).primary()).clicked() { save = true; }
                    if classic::button(ui, "Отмена").clicked() { cancel = true; }
                }
            });
        });
        if let Some(pos) = pointer {
            popup_pointer(ctx, pos, Color32::from_gray(232));
        }
        if visibility_action {
            match &d.kind {
                DialogKind::EditCategory(id) => {
                    let hidden = self
                        .b
                        .categories
                        .iter()
                        .find(|category| category.id == *id)
                        .is_some_and(|category| !category.hidden);
                    self.change(|budget| {
                        let category = budget
                            .categories
                            .iter_mut()
                            .find(|category| category.id == *id)
                            .ok_or("Категория не найдена")?;
                        category.hidden = hidden;
                        Ok(())
                    });
                }
                DialogKind::EditGroup(group) => {
                    let hidden = !d.second;
                    self.change(|budget| budget.set_group_hidden(group, hidden));
                }
                _ => {}
            }
            self.dialog = d.parent.map(|parent| *parent);
            return;
        }
        if cancel || response.should_close() {
            self.dialog = d.parent.map(|parent| *parent);
            return;
        }
        if let Some(next) = next {
            self.open(next);
            self.dialog.as_mut().expect("new dialog").parent = Some(Box::new(d));
            return;
        }
        if save {
            match self.save_dialog(&d) {
                Ok(()) => {
                    self.dialog = d.parent.map(|parent| *parent);
                    return;
                }
                Err(error) => d.error = error,
            }
        }
        self.dialog = Some(d);
    }

    fn note_form(&mut self, ctx: &egui::Context, mut d: Dialog) {
        let DialogKind::Note(key) = &d.kind else {
            unreachable!()
        };
        let subject = if let Some(id) = key.strip_prefix("category:") {
            self.b
                .categories
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.name.clone())
                .unwrap_or_default()
        } else if let Some(month) = key.strip_prefix("month:") {
            month_name(ctx, month)
        } else {
            key.strip_prefix("group:").unwrap_or(key).to_owned()
        };
        let mut save = false;
        let mut cancel = false;
        let mut delete = false;
        let mut pointer = None;
        let mut modal = chrome::modal(ctx, "note_dialog")
            .backdrop_color(Color32::TRANSPARENT)
            .frame(
                Frame::new()
                    .fill(Color32::from_gray(235))
                    .stroke(Stroke::new(1_f32, Color32::from_gray(173)))
                    .corner_radius(3)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 10],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(110),
                    }),
            );
        if let Some(anchor) = d.anchor {
            let screen = ctx.screen_rect();
            let x = anchor.x.min(screen.right() - 387.).max(screen.left() + 8.);
            let y = if anchor.y + 220. > screen.bottom() {
                anchor.y - 260.
            } else {
                anchor.y
            }
            .max(screen.top() + 8.);
            if y >= anchor.y {
                pointer = Some(egui::pos2(x, y));
            }
            modal = modal.area(
                egui::Modal::default_area(egui::Id::new("note_dialog")).anchor(
                    egui::Align2::LEFT_TOP,
                    Vec2::new(x - screen.left(), y - screen.top()),
                ),
            );
        }
        let response = modal.show(ctx, |ui| {
            ui.set_width(375.);
            ui.spacing_mut().item_spacing.y = 9.;
            Frame::new()
                .inner_margin(Margin::symmetric(20, 12))
                .show(ui, |ui| {
                    ui.set_width(335.);
                    ui.label(RichText::new(i18n::tr(ctx, &subject)).size(17.).color(INK));
                    let input = ui.add(
                        egui::TextEdit::multiline(&mut d.fields[0])
                            .desired_width(335.)
                            .desired_rows(6)
                            .background_color(Color32::WHITE),
                    );
                    if d.focus {
                        input.request_focus();
                        d.focus = false;
                    }
                    if !d.error.is_empty() {
                        ui.l_label(RichText::new(&d.error).color(RED));
                    }
                    ui.add_space(2.);
                    ui.horizontal(|ui| {
                        if action_link(ui, "Удалить заметку", true) {
                            delete = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    classic::Button::new("Готово")
                                        .icon(classic::Glyph::Check)
                                        .success(),
                                )
                                .clicked()
                            {
                                save = true;
                            }
                            if ui
                                .add(classic::Button::new("Отмена").icon(classic::Glyph::Cancel))
                                .clicked()
                            {
                                cancel = true;
                            }
                        });
                    });
                });
        });
        if let Some(pos) = pointer {
            popup_pointer(ctx, pos, Color32::from_gray(235));
        }
        if cancel || response.should_close() {
            self.dialog = None;
            return;
        }
        if delete {
            d.fields[0].clear();
            save = true;
        }
        if save {
            match self.save_dialog(&d) {
                Ok(()) => {
                    self.dialog = None;
                    return;
                }
                Err(error) => d.error = error,
            }
        }
        self.dialog = Some(d);
    }
}
