//! Read-only legacy snapshot import. A preview owns the validated candidate and
//! the exact source bytes; application must verify them again before committing.
use crate::{
    model::*,
    register::{ImportState, MatchBackup, Repeat},
    store,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Preview {
    pub budget: Budget,
    pub snapshot: PathBuf,
    pub warnings: Vec<String>,
    source: PathBuf,
    files: BTreeMap<PathBuf, Vec<u8>>,
}
impl Preview {
    pub fn verify_source(&self, source: &Path) -> Result<()> {
        if source != self.source || load_files(source)?.1 != self.files {
            return Err("The source changed after preview. Run Check and Preview again.".into());
        }
        Ok(())
    }
}

pub fn preview(source: &Path) -> Result<Preview> {
    let (snapshots, files) = load_files(source)?;
    let clocks: Vec<_> = snapshots
        .iter()
        .map(|(_, v)| knowledge(text(&v["fileMetaData"], "currentKnowledge")?))
        .collect::<Result<_>>()?;
    let mut required = Vec::new();
    for (p, bytes) in &files {
        let v: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        match p.extension().and_then(|s| s.to_str()) {
            Some("ydiff") => required.push(knowledge(text(&v, "endVersion")?)?),
            Some("ydevice") => {
                if let Some(s) = optional(&v, "knowledge")? {
                    required.push(knowledge(s)?);
                }
            }
            _ => {}
        }
    }
    let eligible: Vec<_> = clocks
        .iter()
        .enumerate()
        .filter(|(_, clock)| {
            clocks
                .iter()
                .chain(&required)
                .all(|other| covers(clock, other))
        })
        .map(|(i, _)| i)
        .collect();
    let i = *eligible.first().ok_or("The full snapshot does not include all known changes. Open, synchronize and close the budget in the source application, then retry. Journals are never silently ignored.")?;
    // Equal version vectors must contain the same data; do not guess by mtime.
    if eligible.iter().any(|j| snapshots[*j].1 != snapshots[i].1) {
        return Err("Conflicting full snapshots with identical versions. Resolve synchronization in the source application first.".into());
    }
    let (path, data) = &snapshots[i];
    let name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported Legacy Budget")
        .split('~')
        .next()
        .unwrap_or("Imported Legacy Budget");
    let (budget, mut warnings) = convert(data, name)?;
    if source.is_file() {
        warnings.push("Detached snapshot: package-wide device history could not be checked. Prefer the complete budget folder.".into());
    }
    warnings.push("Window settings and bank connections are not imported. Payee memo/amount autofill and complex rename rules require review.".into());
    Ok(Preview {
        budget,
        snapshot: path.clone(),
        warnings,
        source: source.into(),
        files,
    })
}

mod source;
use source::*;

mod amounts;
use amounts::money;

mod decode;
use decode::{convert, ordinary_category, sorted};

mod transactions;
use transactions::*;

mod matches;
use matches::*;

mod transfers;
use transfers::*;
