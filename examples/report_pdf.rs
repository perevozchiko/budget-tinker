//! Read a synthetic lab JSON without modifying it and export all graphical reports.
//! cargo run --locked --example report_pdf -- /path/to/lab/budget.json /new/output/dir
use budgettinker::{
    model::*,
    report_print,
    reports::{self, Filter, Selection},
    store,
};
use std::{fs, path::PathBuf};
fn main() {
    let mut args = std::env::args().skip(1);
    let source = PathBuf::from(args.next().expect("Pass synthetic budget.json"));
    let output = PathBuf::from(args.next().expect("Pass NEW output directory"));
    let budget = store::import_json(&source).unwrap();
    assert!(
        budget.name.starts_with("Four Months Lab"),
        "Only synthetic lab fixtures are allowed"
    );
    fs::create_dir(&output).expect("Output directory must be new");
    let f = Filter {
        from: "2026-07-01".into(),
        through: "2026-10-31".into(),
        ..Filter::default()
    };
    for kind in 0..5 {
        let mut filter = f.clone();
        if kind == 4 {
            filter.accounts = Selection::All;
        }
        let r = reports::calculate(&budget, kind, &filter).unwrap();
        let metadata = vec![
            format!(
                "Accounts: {}",
                if kind == 4 {
                    "All Accounts"
                } else {
                    "Budget Accounts"
                }
            ),
            "Categories: All Categories".into(),
            "Payees: All Payees".into(),
        ];
        let d = report_print::layout(
            &r,
            &budget.name,
            "Jul 1 – Oct 31, 2026 (Oct 2026 is a partial month)",
            &metadata,
            &budget.base,
            None,
            |v| money(v, &budget.base),
        );
        d.export_pdf(&output.join(format!("report-{kind}.pdf")))
            .unwrap();
        fs::write(
            output.join(format!("report-{kind}.json")),
            serde_json::to_vec_pretty(&d).unwrap(),
        )
        .unwrap();
        println!("{}: {} pages", reports::TITLES[kind], d.pages.len());
    }
}
