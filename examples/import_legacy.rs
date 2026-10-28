//! Read-only preview, or import into a NEW SQLite file. Never modifies the source.
//! cargo run --locked --example import_legacy -- /path/LegacyBudgetFolder [new.budgettinker]
use budgettinker::{legacy_budget, model::Result, store::Store};
use std::path::Path;
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(1..=2).contains(&args.len()) {
        return Err("Usage: import_legacy SOURCE_FOLDER [NEW.budgettinker]".into());
    }
    let source = Path::new(&args[0]);
    let p = legacy_budget::preview(source)?;
    println!(
        "{} ({})\n{} accounts, {} categories, {} transactions, {} scheduled rows\nSnapshot: {}",
        p.budget.name,
        p.budget.base,
        p.budget.accounts.len(),
        p.budget.categories.len(),
        p.budget.transactions.len(),
        p.budget.scheduled.len(),
        p.snapshot.display()
    );
    for w in &p.warnings {
        println!("Review: {w}");
    }
    if let Some(dest) = args.get(1) {
        let path = Path::new(dest);
        if path.exists() {
            return Err("Destination already exists. Choose a new file.".into());
        }
        // Resolve an existing parent before allowing a destination near the source.
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent = parent.canonicalize().map_err(|e| e.to_string())?;
        let origin = source.canonicalize().map_err(|e| e.to_string())?;
        if parent.starts_with(&origin) {
            return Err("Destination must be outside the source budget.".into());
        }
        p.verify_source(source)?;
        let mut store = Store::open(path)?;
        store.save(&p.budget)?;
        println!("Imported into {}", path.display());
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
