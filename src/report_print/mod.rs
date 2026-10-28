//! A landscape A4 display list shared by the native PDF renderer and egui preview.
//! Financial values come from Report; floating point is used only for geometry.
use crate::{
    model::Result,
    reports::{Report, SpendingScope, TITLES},
};
use eframe::egui::{self, Color32};

pub const WIDTH: f32 = 842.;
pub const HEIGHT: f32 = 595.;
const LEFT: f32 = 32.;
const RIGHT: f32 = 810.;
const BOTTOM: f32 = 555.;
const INK: [f32; 3] = [0.16, 0.20, 0.24];
const PALE: [f32; 3] = [0.92, 0.94, 0.95];

#[derive(Clone, Debug, serde::Serialize)]
pub struct Item {
    kind: &'static str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    size: f32,
    text: String,
    color: [f32; 3],
    points: Vec<[f32; 2]>,
}
impl Default for Item {
    fn default() -> Self {
        Self {
            kind: "text",
            x: 0.,
            y: 0.,
            w: 0.,
            h: 0.,
            size: 10.,
            text: String::new(),
            color: INK,
            points: vec![],
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Document {
    pub pages: Vec<Vec<Item>>,
}

struct Layout {
    document: Document,
    title: String,
    caption: String,
    y: f32,
}

// Keep complete user labels; wide Unicode glyphs get a full em of space.
fn wrap(text: &str, width: f32, size: f32) -> Vec<String> {
    let mut lines = vec![];
    for paragraph in text.split('\n') {
        let chars: Vec<_> = paragraph.chars().collect();
        if chars.is_empty() {
            lines.push(String::new());
        }
        let mut start = 0;
        while start < chars.len() {
            let mut end = start;
            let mut used = 0.;
            let mut space = None;
            while end < chars.len() {
                let advance = size * if chars[end].is_ascii() { 0.65 } else { 1.0 };
                if end > start && used + advance > width {
                    break;
                }
                used += advance;
                end += 1;
                if chars[end - 1].is_whitespace()
                    && chars[start..end].iter().any(|c| !c.is_whitespace())
                {
                    space = Some(end);
                }
            }
            if end < chars.len()
                && let Some(boundary) = space
            {
                end = boundary;
            }
            lines.push(chars[start..end].iter().collect());
            start = end;
        }
    }
    lines
}

impl Layout {
    fn page(&mut self) {
        self.document.pages.push(vec![]);
        self.text(LEFT, 25., 778., 18., &self.title.clone());
        self.text(LEFT, 51., 778., 10., &self.caption.clone());
        self.y = 80.;
    }
    fn items(&mut self) -> &mut Vec<Item> {
        self.document.pages.last_mut().unwrap()
    }
    fn text(&mut self, x: f32, y: f32, w: f32, size: f32, text: &str) {
        // Fit single-line numeric cells without losing integer precision or digits.
        let size = size.min(w / (text.chars().count().max(1) as f32 * 0.65));
        self.items().push(Item {
            x,
            y,
            w,
            h: size * 1.5,
            size,
            text: text.into(),
            ..Default::default()
        });
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 3]) {
        self.items().push(Item {
            kind: "rect",
            x,
            y,
            w,
            h,
            color,
            ..Default::default()
        });
    }
    fn path(&mut self, kind: &'static str, points: Vec<[f32; 2]>, color: [f32; 3]) {
        self.items().push(Item {
            kind,
            points,
            color,
            ..Default::default()
        });
    }
    fn paragraph(&mut self, text: &str) {
        for line in wrap(text, RIGHT - LEFT, 10.) {
            if self.y + 16. > BOTTOM {
                self.page();
            }
            self.text(LEFT, self.y, RIGHT - LEFT, 10., &line);
            self.y += 16.;
        }
    }
    fn table(&mut self, headers: Vec<String>, widths: Vec<f32>, rows: Vec<Vec<String>>) {
        let header_height = 24.;
        let header = |layout: &mut Self| {
            layout.rect(LEFT, layout.y, RIGHT - LEFT, header_height, PALE);
            let mut x = LEFT + 4.;
            for (label, width) in headers.iter().zip(&widths) {
                layout.text(x, layout.y + 5., width - 8., 10., label);
                x += width;
            }
            layout.y += header_height;
        };
        if self.y + 50. > BOTTOM {
            self.page();
        }
        header(self);
        for (index, row) in rows.iter().enumerate() {
            let cells: Vec<_> = row
                .iter()
                .zip(&widths)
                .enumerate()
                .map(|(column, (s, w))| {
                    if column == row.len() - 1
                        || (headers[0] == "Date" && column == 0)
                        || (headers[0] != "Date" && column > 0)
                    {
                        vec![s.clone()]
                    } else {
                        wrap(s, w - 26., 10.)
                    }
                })
                .collect();
            let count = cells.iter().map(Vec::len).max().unwrap_or(1);
            // Very long memo/label cells continue onto the next page without dropping text.
            for line in 0..count {
                if self.y + 17. > BOTTOM {
                    self.page();
                    header(self);
                }
                if index % 2 == 0 {
                    self.rect(LEFT, self.y, RIGHT - LEFT, 17., [0.97; 3]);
                }
                let swatch = headers[0] == "Group" && row[0] != "Total Spending";
                if swatch && line == 0 {
                    self.rect(LEFT + 4., self.y + 3., 8., 8., rgb(group_color(&row[0])));
                }
                let mut x = LEFT + if swatch { 18. } else { 4. };
                for (column, (cell, width)) in cells.iter().zip(&widths).enumerate() {
                    if let Some(text) = cell.get(line) {
                        self.text(x, self.y + 2., width - 8., 10., text);
                    }
                    x += width;
                    if swatch && column == 0 {
                        x -= 14.;
                    }
                }
                self.y += 17.;
            }
        }
        self.y += 12.;
    }
}

pub struct Detail<'a> {
    pub scope: &'a SpendingScope,
    pub month: Option<&'a str>,
    pub label: &'a str,
}

pub fn layout(
    report: &Report,
    budget: &str,
    caption: &str,
    metadata: &[String],
    currency: &str,
    detail: Option<Detail<'_>>,
    money: impl Fn(i64) -> String,
) -> Document {
    let mut l = Layout {
        document: Document { pages: vec![] },
        title: TITLES[report.kind].into(),
        caption: format!("{caption}  |  {currency}"),
        y: 0.,
    };
    l.page();
    l.paragraph(&format!("Budget: {budget}"));
    for line in metadata {
        l.paragraph(line);
    }
    l.y += 10.;
    if let Some(detail) = detail {
        l.paragraph(&format!(
            "Detail: {}{}",
            detail.label,
            detail.month.map(|m| format!(" / {m}")).unwrap_or_default()
        ));
        let entries = report.spending_details(detail.scope, detail.month);
        let mut rows: Vec<_> = entries
            .iter()
            .map(|e| {
                vec![
                    e.date.clone(),
                    e.account.clone(),
                    e.payee.clone(),
                    e.category_label.clone(),
                    e.memo.clone(),
                    money(e.value),
                ]
            })
            .collect();
        rows.push(vec![
            "Total".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
            money(entries.iter().map(|e| e.value).sum()),
        ]);
        l.table(
            ["Date", "Account", "Payee", "Category", "Memo", "Spending"]
                .map(String::from)
                .to_vec(),
            vec![85., 115., 140., 140., 185., 113.],
            rows,
        );
    } else if report.kind == 3 {
        for start in (0..report.months.len()).step_by(6) {
            let end = (start + 6).min(report.months.len());
            if start > 0 {
                l.page();
            }
            l.paragraph(&format!(
                "Months {}–{} of {} (Average / Total use the full selected period)",
                start + 1,
                end,
                report.months.len()
            ));
            let mut headers = vec!["Category / Source".into()];
            headers.extend(report.months[start..end].iter().map(|m| m.month.clone()));
            headers.extend(["Average".into(), "Total".into()]);
            let width = (778. - 210.) / (end - start + 2) as f32;
            let mut widths = vec![210.];
            widths.extend(vec![width; end - start + 2]);
            let rows = report
                .matrix
                .iter()
                .flat_map(|r| {
                    std::iter::once((false, r)).chain(r.children.iter().map(|c| (true, c)))
                })
                .map(|(child, r)| {
                    let mut row = vec![format!("{}{}", if child { "  " } else { "" }, r.label)];
                    row.extend(
                        r.values[start..end]
                            .iter()
                            .copied()
                            .chain([r.average(), r.total()])
                            .map(&money),
                    );
                    row
                })
                .collect();
            l.table(headers, widths, rows);
        }
    } else if report.kind <= 1 {
        if l.y + 235. > BOTTOM {
            l.page();
        }
        let rows = report.spending_rows(None, None);
        let y = l.y;
        let total: f64 = rows
            .iter()
            .filter(|r| r.value > 0)
            .map(|r| r.value as f64)
            .sum();
        let mut angle = -std::f64::consts::FRAC_PI_2;
        for row in &rows {
            if row.value <= 0 || total <= 0. {
                continue;
            }
            let sweep = row.value as f64 / total * std::f64::consts::TAU;
            let steps = (sweep * 40.).ceil().max(2.) as usize;
            let mut points = vec![[180., y + 108.]];
            for step in 0..=steps {
                let a = angle + sweep * step as f64 / steps as f64;
                points.push([
                    180. + a.cos() as f32 * 100.,
                    y + 108. + a.sin() as f32 * 100.,
                ]);
            }
            l.path("polygon", points, spending_color(&row.scope));
            angle += sweep;
        }
        l.text(
            320.,
            y + 35.,
            450.,
            14.,
            &format!("Total Spending: {}", money(report.total)),
        );
        l.text(
            320.,
            y + 65.,
            450.,
            10.,
            "Legend and exact amounts follow below.",
        );
        if total == 0. {
            l.text(80., y + 100., 220., 12., "No positive spending");
        }
        if rows.iter().any(|r| r.value < 0) {
            l.text(
                320.,
                y + 90.,
                450.,
                10.,
                "Refunds are listed separately; pie shows positive spending.",
            );
        }
        l.y += 230.;
        // Keep all payees (including Other members) visible in the printed legend.
        let mut legend = vec![];
        for row in rows {
            legend.push((row.label, row.value, spending_color(&row.scope)));
            if matches!(row.scope, SpendingScope::Other(_)) {
                for child in report.spending_rows(Some(&row.scope), None) {
                    legend.push((
                        format!("  {}", child.label),
                        child.value,
                        spending_color(&row.scope),
                    ));
                }
            }
        }
        for (label, value, color) in legend {
            let lines = wrap(&label, 590., 10.);
            for (i, line) in lines.iter().enumerate() {
                if l.y + 18. > BOTTOM {
                    l.page();
                }
                if i == 0 {
                    l.rect(LEFT, l.y + 2., 10., 10., color);
                    l.text(660., l.y, 145., 10., &money(value));
                }
                l.text(50., l.y, 600., 10., line);
                l.y += 18.;
            }
        }
    } else {
        for start in (0..report.months.len()).step_by(12) {
            let end = (start + 12).min(report.months.len());
            if start > 0 || l.y + 245. > BOTTOM {
                l.page();
            }
            chart(&mut l, report, start, end, &money);
            l.y += 245.;
            if report.kind == 4 {
                l.table(
                    ["Month", "Assets", "Debts", "Net Worth"]
                        .map(String::from)
                        .to_vec(),
                    vec![178., 200., 200., 200.],
                    report.months[start..end]
                        .iter()
                        .map(|m| {
                            vec![
                                m.month.clone(),
                                money(m.assets),
                                money(m.debts),
                                money(m.net_worth),
                            ]
                        })
                        .collect(),
                );
            } else {
                // Six monthly columns per table keep values readable.
                for sub in (start..end).step_by(6) {
                    let last = (sub + 6).min(end);
                    let mut titles = vec!["Group".into()];
                    titles.extend(report.months[sub..last].iter().map(|m| m.month.clone()));
                    let mut widths = vec![210.];
                    widths.extend(vec![568. / (last - sub) as f32; last - sub]);
                    let mut rows: Vec<_> = report
                        .spending_series
                        .iter()
                        .map(|s| {
                            let mut row = vec![s.label.clone()];
                            row.extend(s.values[sub..last].iter().map(|v| money(*v)));
                            row
                        })
                        .collect();
                    let mut total = vec!["Total Spending".into()];
                    total.extend(report.months[sub..last].iter().map(|m| money(m.expense)));
                    rows.push(total);
                    l.table(titles, widths, rows);
                }
            }
        }
    }
    let count = l.document.pages.len();
    for (i, page) in l.document.pages.iter_mut().enumerate() {
        page.push(Item {
            x: LEFT,
            y: 570.,
            w: 778.,
            h: 15.,
            text: format!("BudgetTinker  |  Page {} of {count}", i + 1),
            size: 9.,
            ..Default::default()
        });
    }
    l.document
}

pub fn group_color(label: &str) -> Color32 {
    let hash = label.bytes().fold(2166136261_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16777619)
    });
    egui::ecolor::Hsva::new((hash % 360) as f32 / 360., 0.60, 0.78, 1.).into()
}
fn rgb(color: Color32) -> [f32; 3] {
    [
        color.r() as f32 / 255.,
        color.g() as f32 / 255.,
        color.b() as f32 / 255.,
    ]
}
fn spending_color(scope: &SpendingScope) -> [f32; 3] {
    rgb(match scope {
        SpendingScope::Other(_) => Color32::from_rgb(135, 145, 150),
        SpendingScope::Group(name) => group_color(name),
        SpendingScope::Category(id) => group_color(&format!("category:{id:?}")),
        SpendingScope::Payee(name) => group_color(&format!("payee:{name}")),
    })
}

fn chart(
    l: &mut Layout,
    report: &Report,
    start: usize,
    end: usize,
    money: &impl Fn(i64) -> String,
) {
    let top = l.y + 12.;
    let bottom = l.y + 192.;
    let (mut min, mut max) = (0_i64, 0_i64);
    for i in start..end {
        if report.kind == 4 {
            min = min.min(report.months[i].debts);
            max = max.max(report.months[i].assets);
        } else {
            let positive: i64 = report
                .spending_series
                .iter()
                .map(|s| s.values[i].max(0))
                .sum();
            let negative: i64 = report
                .spending_series
                .iter()
                .map(|s| s.values[i].min(0))
                .sum();
            max = max.max(positive);
            min = min.min(negative);
        }
    }
    let span = (max as f64 - min as f64).max(1.);
    let y = |value: f64| bottom - ((value - min as f64) / span) as f32 * (bottom - top);
    let zero = y(0.);
    l.path("line", vec![[132., zero], [800., zero]], INK);
    l.text(LEFT, top, 96., 9., &money(max));
    l.text(LEFT, bottom - 14., 96., 9., &money(min));
    let step = 650. / (end - start) as f32;
    let mut net = vec![];
    for i in start..end {
        let x = 140. + (i - start) as f32 * step;
        let width = (step * 0.65).min(45.);
        if report.kind == 4 {
            for (value, color) in [
                (report.months[i].assets, [0.25, 0.62, 0.68]),
                (report.months[i].debts, [0.85, 0.35, 0.35]),
            ] {
                let end_y = y(value as f64);
                l.rect(x, end_y.min(zero), width, (end_y - zero).abs(), color);
            }
            net.push([x + width / 2., y(report.months[i].net_worth as f64)]);
        } else {
            let (mut positive, mut negative) = (0_i64, 0_i64);
            for s in &report.spending_series {
                let v = s.values[i];
                let stack = if v >= 0 { &mut positive } else { &mut negative };
                let a = y(*stack as f64);
                *stack += v;
                let b = y(*stack as f64);
                l.rect(
                    x,
                    a.min(b),
                    width,
                    (a - b).abs(),
                    rgb(group_color(&s.label)),
                );
            }
        }
        l.text(x - 4., bottom + 7., step, 9., &report.months[i].month);
    }
    if report.kind == 4 {
        if net.len() == 1 {
            l.rect(net[0][0] - 2., net[0][1] - 2., 4., 4., INK);
        } else {
            l.path("line", net, INK);
        }
        for (i, (label, color)) in [
            ("Assets", [0.25, 0.62, 0.68]),
            ("Debts", [0.85, 0.35, 0.35]),
            ("Net Worth", INK),
        ]
        .iter()
        .enumerate()
        {
            let x = 150. + i as f32 * 175.;
            l.rect(x, bottom + 30., 10., 10., *color);
            l.text(x + 16., bottom + 28., 150., 10., label);
        }
    } else {
        l.text(
            140.,
            bottom + 29.,
            650.,
            10.,
            "Group colors match Spending By Category. Exact monthly amounts follow.",
        );
        // A compact color key precedes the amounts, and can continue on its own pages.
        // The full group labels appear in the table rather than being truncated here.
    }
}

mod native;

#[cfg(test)]
mod tests;
