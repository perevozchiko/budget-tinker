use budgettinker::{
    legacy_budget,
    model::*,
    register::Repeat,
    store::{self, Store},
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

struct Fixture {
    root: PathBuf,
    full: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir()
            .join(format!("legacy-import-{}", id()))
            .join("Synthetic.legacybudget");
        let full = root.join("data/device/Budget.yfull");
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(
            root.join("Budget.ymeta"),
            r#"{"formatVersion":"2","relativeDataFolderName":"data"}"#,
        )
        .unwrap();
        fs::write(&full, include_bytes!("fixtures/legacy/Budget.yfull")).unwrap();
        Self { root, full }
    }
    fn edit(&self, f: impl FnOnce(&mut Value)) {
        let mut v = serde_json::from_slice(&fs::read(&self.full).unwrap()).unwrap();
        f(&mut v);
        fs::write(&self.full, serde_json::to_vec(&v).unwrap()).unwrap();
    }
    fn matched() -> Self {
        let f = Self::new();
        fs::write(&f.full, include_bytes!("fixtures/legacy/Matched.yfull")).unwrap();
        f
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(self.root.parent().unwrap()).unwrap();
    }
}

fn boundary_transfer(f: &Fixture, value: i64, category: &str, peer_category: Option<&str>) {
    f.edit(|v| {
        v["accounts"][2]["hidden"] = json!(false);
        v["transactions"][6]["targetAccountId"] = json!("asset");
        v["transactions"][6]["payeeId"] = Value::Null;
        v["transactions"][6]["amount"] = json!(value);
        v["transactions"][6]["categoryId"] = json!(category);
        v["transactions"][7]["accountId"] = json!("asset");
        v["transactions"][7]["amount"] = json!(-value);
        v["transactions"][7]["categoryId"] = json!(peer_category);
    });
}

#[path = "legacy_import/matching.rs"]
mod matching;
#[path = "legacy_import/schedules.rs"]
mod schedules;
#[path = "legacy_import/snapshots.rs"]
mod snapshots;
#[path = "legacy_import/splits.rs"]
mod splits;
#[path = "legacy_import/transfers.rs"]
mod transfers;
