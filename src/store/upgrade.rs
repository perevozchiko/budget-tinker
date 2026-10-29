//! Read-only migration preparation and recoverable SQLite backups.
//! Activation is explicit and preserves a verified recovery copy.
use super::*;
use crate::model::operation_fx::FrozenOperation;
use rusqlite::OptionalExtension;

/// A validated current document plus detached locks; never saved as schema v2.
#[derive(Clone, Debug, serde::Serialize)]
pub struct FrozenBudget {
    pub budget: Budget,
    pub operations: std::collections::BTreeMap<String, FrozenOperation>,
    /// Originals restored by Unmatch/Reject need locks of their own.
    pub matched_originals: std::collections::BTreeMap<String, [FrozenOperation; 2]>,
}

impl FrozenBudget {
    pub fn preflight(budget: &Budget) -> Result<Self> {
        validate(budget)?;
        if budget.version != 1 {
            return Err("Only schema v1 budgets need this upgrade".into());
        }
        let operations = budget
            .transactions
            .iter()
            .map(|t| Ok((t.id.clone(), FrozenOperation::from_current(budget, t)?)))
            .collect::<Result<_>>()?;
        let matched_originals = budget
            .register_meta
            .iter()
            .filter_map(|(id, state)| state.matched.as_ref().map(|pair| (id, pair)))
            .map(|(id, pair)| {
                Ok((
                    id.clone(),
                    [
                        FrozenOperation::from_current(budget, &pair.manual)?,
                        FrozenOperation::from_current(budget, &pair.imported)?,
                    ],
                ))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            budget: budget.clone(),
            operations,
            matched_originals,
        })
    }
    pub fn into_v2(mut self) -> Result<Budget> {
        for t in &mut self.budget.transactions {
            t.operation_fx = Some(
                self.operations
                    .remove(&t.id)
                    .ok_or("Missing prepared operation")?,
            );
        }
        for (id, state) in &mut self.budget.register_meta {
            if let Some(pair) = &mut state.matched {
                let [manual, imported] = self
                    .matched_originals
                    .remove(id)
                    .ok_or("Missing prepared originals")?;
                pair.manual.operation_fx = Some(manual);
                pair.imported.operation_fx = Some(imported);
            }
        }
        for t in self.budget.transactions.iter_mut().chain(
            self.budget
                .register_meta
                .values_mut()
                .filter_map(|state| state.matched.as_mut())
                .flat_map(|pair| [&mut pair.manual, &mut pair.imported]),
        ) {
            for (part, frozen) in t
                .splits
                .iter_mut()
                .zip(&t.operation_fx.as_ref().ok_or("Missing FX")?.splits)
            {
                part.id = frozen.id.clone();
            }
        }
        self.budget.version = 2;
        validate(&self.budget)?;
        Ok(self.budget)
    }
}

#[derive(Clone, Debug)]
pub struct UpgradePreview {
    pub current: FrozenBudget,
    pub revisions: Vec<(i64, FrozenBudget)>,
    source: SourceDocuments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceDocuments {
    current: String,
    revisions: Vec<(i64, String, String)>,
}

fn documents(conn: &Connection) -> Result<SourceDocuments> {
    let current = conn
        .query_row("SELECT json FROM document WHERE id=1", [], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("No budget to upgrade")?;
    let mut query = conn
        .prepare("SELECT id, created_at, json FROM revisions ORDER BY id")
        .map_err(|e| e.to_string())?;
    let revisions = query
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(SourceDocuments { current, revisions })
}

fn prepare(json: &str) -> Result<FrozenBudget> {
    let budget = serde_json::from_str(json).map_err(|e| e.to_string())?;
    FrozenBudget::preflight(&budget)
}

impl Store {
    /// One consistent read snapshot, including every retained revision. A bad
    /// revision blocks upgrade rather than being discarded or misinterpreted.
    pub fn preflight_upgrade(&self) -> Result<UpgradePreview> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let source = documents(&tx)?;
        let current = prepare(&source.current)?;
        let revisions = source
            .revisions
            .iter()
            .map(|(id, _, json)| {
                prepare(json)
                    .map(|candidate| (*id, candidate))
                    .map_err(|e| format!("Cannot upgrade revision {id}: {e}"))
            })
            .collect::<Result<_>>()?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(UpgradePreview {
            current,
            revisions,
            source,
        })
    }

    /// Back up the exact preflighted source through SQLite's online backup API,
    /// including committed WAL pages, revisions and any additional tables.
    /// This does not activate a migration or replace the live document.
    pub fn backup_for_upgrade(&self, preview: &UpgradePreview, parent: &Path) -> Result<PathBuf> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        if documents(&tx)? != preview.source {
            return Err("Budget changed after upgrade preflight; review it again".into());
        }
        // Reserve a private, unique directory; never overwrite an existing backup.
        let directory = parent.join(format!("budgettinker-pre-upgrade-{}", id()));
        fs::create_dir(&directory).map_err(|e| format!("Cannot create upgrade backup: {e}"))?;
        let path = directory.join("original.budgettinker");
        let result = (|| {
            self.conn
                .backup(rusqlite::MAIN_DB, &path, None)
                .map_err(|e| e.to_string())?;
            let backup =
                Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(|e| e.to_string())?;
            let integrity: String = backup
                .query_row("PRAGMA integrity_check", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            if integrity != "ok" || documents(&backup)? != preview.source {
                return Err("Upgrade backup verification failed".into());
            }
            fs::File::open(&path)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            #[cfg(unix)]
            fs::File::open(&directory)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            #[cfg(unix)]
            fs::File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(path.clone())
        })();
        if result.is_err() {
            // Keep a potentially complete recovery copy on any verification or
            // durability error; callers receive its location for investigation.
            return result.map_err(|e: String| {
                format!(
                    "{e}. Backup candidate: {}. Budget was not upgraded.",
                    path.display()
                )
            });
        }
        result
    }
}

impl Store {
    /// Call only after explicit user confirmation. Rebuild from the private
    /// preflight source, never from mutable public preview fields. SQLite rolls
    /// back document and revision writes together on every failure.
    pub fn upgrade(
        &mut self,
        preview: &UpgradePreview,
        parent: &Path,
    ) -> Result<(Budget, PathBuf)> {
        let current = prepare(&preview.source.current)?.into_v2()?;
        let json = serde_json::to_string(&current).map_err(|e| e.to_string())?;
        let revisions = preview
            .source
            .revisions
            .iter()
            .map(|(id, _, json)| {
                let next = prepare(json)?.into_v2()?;
                Ok((
                    *id,
                    serde_json::to_string(&next).map_err(|e| e.to_string())?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let backup = self.backup_for_upgrade(preview, parent)?;
        let result = (|| {
            let tx = self
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| e.to_string())?;
            if documents(&tx)? != preview.source {
                return Err("Budget changed after upgrade preflight; review it again".into());
            }
            tx.execute("UPDATE document SET json=?1 WHERE id=1", [&json])
                .map_err(|e| e.to_string())?;
            for (id, json) in &revisions {
                tx.execute(
                    "UPDATE revisions SET json=?1 WHERE id=?2",
                    params![json, id],
                )
                .map_err(|e| e.to_string())?;
            }
            // Read through the same validation contract before committing.
            let written = documents(&tx)?;
            let expected = SourceDocuments {
                current: json.clone(),
                revisions: preview
                    .source
                    .revisions
                    .iter()
                    .zip(&revisions)
                    .map(|((id, timestamp, _), (_, json))| (*id, timestamp.clone(), json.clone()))
                    .collect(),
            };
            if written != expected {
                return Err("Upgrade write verification failed".into());
            }
            let restored: Budget =
                serde_json::from_str(&written.current).map_err(|e| e.to_string())?;
            validate(&restored)?;
            for (_, _, json) in written.revisions {
                let restored = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                validate(&restored)?;
            }
            tx.commit().map_err(|e| e.to_string())?;
            Ok(())
        })();
        result.map_err(|e: String| {
            format!(
                "{e}. Upgrade was not applied. Recovery copy: {}",
                backup.display()
            )
        })?;
        Ok((current, backup))
    }
}
