use super::*;
use chrono::Datelike;

#[derive(Clone)]
struct Draft {
    open: bool,
    last_frame: u64,
    period: usize,
    from: i32,
    through: i32,
    filter: reports::Filter,
}

fn month_index(date: &str) -> i32 {
    let date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());
    date.year() * 12 + date.month0() as i32
}

fn month_date(month: i32) -> String {
    format!(
        "{:04}-{:02}-01",
        month.div_euclid(12),
        month.rem_euclid(12) + 1
    )
}

pub(super) fn timeframe(
    ui: &mut egui::Ui,
    budget: &Budget,
    period: &mut usize,
    filter: &mut reports::Filter,
) {
    let id = ui.make_persistent_id("report_timeframe_draft");
    let frame_nr = ui.ctx().cumulative_frame_nr();
    let fresh = || Draft {
        open: false,
        last_frame: frame_nr,
        period: *period,
        from: month_index(&filter.from),
        through: month_index(&filter.through),
        filter: filter.clone(),
    };
    let mut draft = ui
        .data_mut(|d| d.get_temp::<Draft>(id))
        .unwrap_or_else(fresh);
    if draft.last_frame + 1 < ui.ctx().cumulative_frame_nr() {
        draft = fresh();
    }
    draft.last_frame = ui.ctx().cumulative_frame_nr();
    let label = if *period == 6 {
        if filter.from[..7] == filter.through[..7] {
            report_month(&filter.from[..7])
        } else {
            format!(
                "{} – {}",
                report_month(&filter.from[..7]),
                report_month(&filter.through[..7])
            )
        }
    } else {
        PERIODS[*period].to_owned()
    };
    let trigger = popover::trigger(ui, "report_period", &label, draft.open);
    if trigger.clicked() {
        let open = !draft.open;
        draft = fresh();
        draft.open = open;
    }
    let nested = egui::Popup::is_any_open(ui.ctx());
    let popup = egui::Popup::menu(&trigger)
        .open_bool(&mut draft.open)
        .align(egui::RectAlign::BOTTOM)
        .width(458.)
        .gap(16.)
        .close_behavior(if nested {
            egui::PopupCloseBehavior::IgnoreClicks
        } else {
            egui::PopupCloseBehavior::CloseOnClickOutside
        })
        .frame(popover::frame())
        .show(|ui| {
            popover::header(ui, 458., "Timeframe");
            popover::band(ui, 458., Color32::WHITE, 3, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.;
                    for index in [0, 2, 4, 7, 5] {
                        let enabled = index != 7
                            || budget
                                .transactions
                                .iter()
                                .any(|t| t.date[..4] < today()[..4]);
                        if ui
                            .add_enabled_ui(enabled, |ui| {
                                popover::link(ui, PERIODS[index], draft.period == index)
                            })
                            .inner
                            .clicked()
                        {
                            draft.period = index;
                            apply_report_period(budget, index, &mut draft.filter, &today());
                            draft.from = month_index(&draft.filter.from);
                            draft.through = month_index(&draft.filter.through);
                            *period = draft.period;
                            filter.from = draft.filter.from.clone();
                            filter.through = draft.filter.through.clone();
                            ui.close();
                        }
                    }
                });
            });
            popover::divider(ui, 458.);
            popover::band(ui, 458., Color32::from_gray(245), 12, |ui| {
                ui.add_space(9.);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.;
                    ui.label(RichText::new("From:").font(crate::palette::bold(12.)));
                    let from =
                        crate::register_timeframe::month_picker(ui, "report_from", &mut draft.from);
                    ui.label(RichText::new("Through:").font(crate::palette::bold(12.)));
                    let through = crate::register_timeframe::month_picker(
                        ui,
                        "report_through",
                        &mut draft.through,
                    );
                    if from || through {
                        draft.period = 6;
                        draft.filter.from = month_date(draft.from);
                        draft.filter.through = Budget::month_end(&month_date(draft.through)[..7]);
                    }
                });
                ui.add_space(9.);
                if let Err(error) = draft.filter.months() {
                    ui.colored_label(RED, error);
                }
            });
            if popover::footer(ui, 458., draft.filter.months().is_ok()) {
                *period = draft.period;
                filter.from = draft.filter.from.clone();
                filter.through = draft.filter.through.clone();
            }
        });
    if let Some(popup) = popup {
        popover::pointer(ui, &trigger, popup.response.rect);
    }
    ui.data_mut(|d| {
        if draft.open {
            d.insert_temp(id, draft);
        } else {
            d.remove::<Draft>(id);
        }
    });
}
