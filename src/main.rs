mod desktop;
mod i18n;
mod native_menu;
mod preferences;
mod theme;
mod ui;
mod widgets;
use budgettinker::{
    model::Budget,
    store::{Store, default_path},
};
use eframe::egui;
use theme::{classic, icons, palette};
use widgets::{
    budget_editor, category_picker, date_input, payee_picker, register_search, register_timeframe,
    register_view, transaction_editor,
};

fn main() -> eframe::Result {
    let startup_path = preferences::Preferences::startup_path(&default_path());
    let mut store = match Store::open(&startup_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Could not open the database: {}",
                i18n::Language::En.translate(&e)
            );
            std::process::exit(1)
        }
    };
    let loaded = match store.load() {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "Could not read the budget: {}. The source file was not changed.",
                i18n::Language::En.translate(&e)
            );
            std::process::exit(1)
        }
    };
    let first = loaded.is_none();
    let budget = loaded.unwrap_or_else(|| Budget::new("EUR"));
    if first && let Err(e) = store.save(&budget) {
        eprintln!(
            "Could not save the budget: {}",
            i18n::Language::En.translate(&e)
        );
        std::process::exit(1)
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/app-icon.png"))
                    .expect("bundled BudgetTinker icon must be a valid PNG"),
            )
            .with_title(format!("BudgetTinker — {}", budget.name))
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1050.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native(
        "BudgetTinker",
        options,
        Box::new(move |cc| Ok(Box::new(ui::App::new(cc, store, budget, first)))),
    )
}
