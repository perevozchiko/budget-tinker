# Agent guide: BudgetTinker

Native Rust (edition 2024), eframe/egui 0.32.3 and SQLite desktop budgeting app.
Keep product UI in English; communicate with the owner in Russian unless asked otherwise.

Read docs/DEVELOPMENT-PLAN.md and docs/VALIDATION.md before working.
Preserve unrelated edits; do not initialize Git, commit, push or publish without an explicit request.
Do not spawn additional agents unless explicitly requested by the user.

## Structure

- src/model/: serialized data types, money, accounts, transactions, categories, totals and schedules.
- src/register.rs: validated register commands, matching and recurrence.
- src/store.rs: SQLite validation, persistence, JSON and native default paths.
- src/ui/: application state, Undo/Redo, sidebar, accounts, budget views and dialogs.
- src/ui/report_view/: report filters, preferences, charts, matrices and grouped event tests.
- src/ui/chrome/: screen/dialog chrome, menu commands, budget switching, printing and payees.
- src/ui/scheduled_panel/: schedule commands, view and lifecycle tests.
- src/widgets/: shared category/payee/date inputs, register widgets and budget editing.
- src/widgets/transaction_editor/: draft building, row rendering and topic-based tests.
- src/theme/: unchanged colors, code-drawn icons and classic controls separated by role.
- src/imports/legacy_budget/: read-only source loading, decoding, matching and transfer mapping.
- src/reports.rs, src/report_print/: calculations, portable print layout and native renderer.
- src/desktop.rs, src/native_menu.rs, src/preferences.rs: platform operations and settings.
- tests/: financial/persistence integration suites with synthetic legacy/bank fixtures.
- examples/: synthetic generators and read-only importer examples.
- scripts/: bundling, notice staging, dependency/source audits and verified source ZIP packaging.

## Data safety

Normal startup can open personal data. Always use a fresh temporary directory for GUI checks:

```sh
export BUDGETTINKER_DATA="$(mktemp -d /tmp/budgettinker-demo.XXXXXX)/demo.budgettinker"
cargo run --locked --example demo
cargo run --locked
```

Preferences beside the database can select a recent budget; do not reuse an old demo directory.
Never modify personal budgets for experiments. Read live SQLite using read-only connections;
use SQLite backup rather than copying a database while WAL is active.
Paired transfers must change atomically; money must retain integer precision.
Use existing validation/commands/persistence/Undo patterns and serde defaults for new fields.
Native GUI checks use macOS Accessibility/CoreGraphics, not browser automation.
Verify data effects as well as screenshots and keep all screenshots synthetic.

## Required checks

```sh
cargo fmt --all --check
cargo test --locked --quiet
cargo clippy --all-targets --locked -- -D warnings
cargo check --examples --locked
```

For GUI or packaging changes, rebuild with `bash scripts/bundle.sh`, verify codesign and plist,
and inspect the rebuilt app. Report model/integration, egui and live GUI checks separately.
Update development plan and append concrete evidence to validation docs.
New assets require provenance in ATTRIBUTION.md. Original commercial software, screenshots,
reference dumps, personal data and secrets must never be added to this public copy.

## Release readiness

Keep the active MIT OR Apache-2.0 declaration and LICENSE-STATUS.md consistent. Do not edit
upstream license texts to make checks pass or suppress RustSec findings silently.
Use scripts/audit-release.py for source candidates and inventory integrity. Use
scripts/stage-notices.py for all platform packages; it verifies notice hashes first.
See docs/DEPENDENCY-AUDIT.md for cargo-deny commands, exceptions and current findings.
Only workflow_dispatch may trigger the existing CI. Tool/cache logs remain under target/.

The only approved advisory exception is RUSTSEC-2026-0192 for the pinned font stack,
requiring review by 2027-01-28 UTC. Always run scripts/audit-dependencies.py: cargo-deny
0.20.2 alone cannot enforce the deadline or direct-depender/version guard. Run
scripts/test-release-tools.py too. Use scripts/package-source.py for a local review ZIP;
it preserves .github/.gitignore and verifies every archive entry. Creating a local archive does not authorize publication or Git operations.
