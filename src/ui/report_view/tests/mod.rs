use super::*;

struct ReportFixture {
    app: App,
    root: std::path::PathBuf,
    ctx: egui::Context,
}

impl Drop for ReportFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn report_fixture() -> ReportFixture {
    let root = std::env::temp_dir().join(format!("report-settings-{}", id()));
    let mut budget = Budget::new("USD");
    let checking = budget
        .add_account("Checking", "USD", true, 10000, "2026-07-01")
        .unwrap();
    budget
        .add_account("Brokerage", "USD", false, 50000, "2026-08-01")
        .unwrap();
    let mut draft = RegisterDraft::new(&budget, Some(&checking), None).unwrap();
    draft.date = "2026-10-20".into();
    draft.payee = "Shop".into();
    draft.category = budget.categories[0].id.clone();
    draft.outflow = "10".into();
    // Use the model's existing transaction builder/validation path.
    budget
        .put_transaction(draft.build(&budget).unwrap())
        .unwrap();
    let mut store = Store::open(&root.join("first.sqlite3")).unwrap();
    store.save(&budget).unwrap();
    let ctx = egui::Context::default();
    let app = App::new(
        &eframe::CreationContext::_new_kittest(ctx.clone()),
        store,
        budget,
        false,
    );
    ReportFixture { app, root, ctx }
}

fn choices() -> Vec<(&'static str, String, bool)> {
    vec![
        ("checking", "Checking".into(), true),
        ("savings", "Savings".into(), true),
        ("tracking", "Tracking".into(), false),
    ]
}

fn text_shapes(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Pos2)> {
    fn visit(shape: &egui::Shape, result: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            egui::Shape::Text(text) => result.push((text.galley.job.text.clone(), text.pos)),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    visit(shape, result);
                }
            }
            _ => {}
        }
    }
    let mut result = vec![];
    for shape in shapes {
        visit(&shape.shape, &mut result);
    }
    result
}

mod charts;
mod filters;
mod matrix;
mod periods;
mod persistence;
mod printing;
