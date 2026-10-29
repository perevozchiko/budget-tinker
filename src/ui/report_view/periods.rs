use super::*;

pub(super) fn retain_report_choices(selection: &mut Selection, exists: impl Fn(&str) -> bool) {
    if let Selection::Selected(ids) = selection {
        ids.retain(|id| exists(id));
        if ids.is_empty() {
            *selection = Selection::NoneSelected;
        }
    }
}

pub(super) fn apply_report_period(
    budget: &Budget,
    period: usize,
    filter: &mut reports::Filter,
    today: &str,
) {
    let month = &today[..7];
    let (from, through) = match period {
        0 => (format!("{month}-01"), Budget::month_end(month)),
        1 => {
            let previous = month_shift(month, -1);
            (format!("{previous}-01"), Budget::month_end(&previous))
        }
        2 | 3 => (
            format!(
                "{}-01",
                month_shift(month, if period == 2 { -2 } else { -5 })
            ),
            Budget::month_end(month),
        ),
        4 => (format!("{}-01-01", &month[..4]), Budget::month_end(month)),
        5 => (
            budget
                .transactions
                .iter()
                .map(|t| t.date.as_str())
                .min()
                .unwrap_or(today)
                .to_owned(),
            budget
                .transactions
                .iter()
                .map(|t| t.date.as_str())
                .max()
                .unwrap_or(today)
                .max(today)
                .to_owned(),
        ),
        7 => {
            let year = month[..4].parse::<i32>().unwrap() - 1;
            (format!("{year:04}-01-01"), format!("{year:04}-12-31"))
        }
        _ => return,
    };
    filter.from = format!("{}-01", &from[..7]);
    filter.through = Budget::month_end(&through[..7]);
}

pub(super) fn report_period_caption(filter: &reports::Filter, today: &str) -> String {
    let date = |value: &str| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok();
    let (Some(from), Some(through), Some(today)) =
        (date(&filter.from), date(&filter.through), date(today))
    else {
        return format!("{} — {}", filter.from, filter.through);
    };
    let mut caption = format!(
        "{} — {}",
        from.format("%b %-d, %Y"),
        through.format("%b %-d, %Y")
    );
    let month = today.format("%Y-%m").to_string();
    let first = format!("{month}-01");
    let last = Budget::month_end(&month);
    // Label a selection that includes any part of the current, unfinished month.
    if filter.from <= last && filter.through >= first && today.to_string() < last {
        caption.push_str(&format!(" · {} is a partial month", today.format("%b %Y")));
    }
    caption
}
