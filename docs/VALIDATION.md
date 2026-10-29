# Validation — public preparation, 2026-10-28

## Multi-currency discovery — 2026-10-29

- Created local `feature/multi-currency` from clean local `master` on owner
  instruction. No commit, push or publication was performed.
- Inspected currency precision, monthly snapshot immutability, transaction and
  split valuation, actual-amount transfers, budget revaluation, schedules,
  SQLite JSON validation and UI restrictions. Added 42 owner questions in
  MULTI-CURRENCY-QUESTIONS.md; proposed defaults remain unapproved. Application
  code, financial behavior, document schema and dependencies are unchanged.
- Model/integration and egui: the existing complete suite passed all **284 Rust
  tests**. Formatting, strict all-target Clippy, example checks, release-tool
  tests and all four dependency audits passed with the existing scoped advisory
  exception. Logs remain under `target/multi-currency-discovery/` and
  `target/release-audit/`.
- Documentation-only work: no bundle rebuild or native GUI check was performed;
  no personal budget was opened.
- Owner follow-up: recorded one budget base currency selected in settings and
  accounts in different currencies including the base currency. Currency lists,
  changing an existing budget's base and valuation policies remain open. Only
  requirements documentation changed; `git diff --check` passed. The checks
  above precede this prose-only follow-up and were not repeated.

## Multi-currency phase 0 and first Phase 1 increment — 2026-10-29

- Copied the owner-provided approved v2 specification verbatim to
  `docs/MULTI-CURRENCY-SPECIFICATION-v2.md`; both files have SHA-256
  `1b45838c1b846b6d0bbe561c848e041d3703ac7e4c1c470092d27fa044252b05`.
  The later owner amendment selects `1 500.00` and overrides original MC-006;
  see MULTI-CURRENCY-AMENDMENTS.md. Linked all six phases and acceptance cases
  in DEVELOPMENT-PLAN; retained
  pre-existing discovery edits/questions as historical context. Audit, migration,
  grouping findings and cache/invalidation design are recorded in
  MULTI-CURRENCY-IMPLEMENTATION.md. No Git mutations or release actions performed.
- **Baseline model/integration and egui:** all **284 Rust tests** passed before
  changing application code. The new benchmark example was then run against the
  original ten-currency model before adding TRY.
- **Final model/integration:** **161 integration tests + 5 library tests** passed.
  Added exact space-grouped parsing/rejection, all 11 currencies, JPY/KWD positive and
  negative precision, amount limits, extreme checked conversion overflow,
  symmetric rounding, all currency-pair signs and all-currency current-document
  JSON equality. CSV/QIF/OFX/QFX regression tests accept valid grouping and reject
  malformed grouping without changing the input budget. All prior importer,
  schedule, persistence, transfer, split and accounting tests passed. These tests
  cover the numeric foundation; they do not claim the future v2 FX schema,
  migration, linked refunds or AC-12 full-feature restore already exist.
- **Final egui/UI and binary unit tests:** **124 passed**, including existing
  report chart/printing/reconciliation event tests updated for ISO text. A new
  transaction-draft test verifies invalid comma input keeps payee/amount fields,
  then space-grouped input posts exactly. Total final Rust result: **290 passed**,
  zero failed. `cargo fmt --all --check`, strict all-target Clippy and
  `cargo check --examples --locked` passed. Initial intermediate failures exposed
  four old symbol/comma-decimal expectations; updated to approved MC-006 rules
  before the successful suite; separator expectations were then updated again
  for the owner amendment and the full suite rerun.
- **Performance:** release example `multi_currency_baseline` measured 100
  accounts, 100,000 synthetic rows and 240 months, initially 10 currencies and
  then all 11. Apple M3 Pro, macOS 26.4.1, Rust 1.99.0, release opt-level 2.
  Original month summary: 4223.601 ms; 20-year Net Worth report: 6479.905 ms.
  Eleven-currency run: 4081.602 / 4782.229 ms respectively. SQLite save/load:
  404.696 / 319.845 ms in the second run. These single runs are bottleneck
  evidence, not a speedup claim or p95 guarantee. Complete timings, provisional
  targets, correction/Undo timing surrogates and unavailable 50-custom-group
  workload are documented in MULTI-CURRENCY-IMPLEMENTATION.md. No performance
  optimization or full new-feature benchmark is claimed yet.
- **Packaging:** `bash scripts/bundle.sh` passed; shared notice staging verified
  327 dependency records. Rebuilt macOS app passed strict deep codesign
  verification and Info.plist lint. No new graphical assets or dependencies;
  no Windows/Linux build or GUI check performed.
- **Live native GUI:** final space-format rebuilt executable PID 91598,
  window 21113, fresh synthetic budget
  `/tmp/budgettinker-space-format-nqjk6v0c/demo.budgettinker`. The startup data
  environment was isolated before generating the demo; `lsof` confirmed that
  database and WAL. Accessibility/CoreGraphics focused/sized the app. Inspected
  window-only captures show space-grouped totals and ISO codes. Entering
  `1 500.00` in Groceries saved exactly **150000** minor units (was 40000),
  confirmed using SQLite read-only. Next-cell input `1,500.00` was rejected,
  retained in the red-bordered editor, and left the full SQLite document
  unchanged. Escape, leaving the editor, and native Command-Z restored the
  **full original document**, verified read-only. Results and driver are under
  `target/multi-currency-phase-0/space-live-*` and `verify-space-live.py`.
  Earlier comma-format captures are intermediate only. No personal budget was
  opened or modified; all screenshots are synthetic and stay under target.
  No human acceptance is claimed.
- **Release tools/dependencies:** all 14 release-tool tests and all four wrapped
  cargo-deny checks passed, with the existing scoped advisory exception and
  deadline guard. `scripts/audit-release.py` reported **825 source candidates,
  327 dependency records, one error**: the already-existing `rust.yml` has push
  triggers for master/version tags, contrary to the audit's manual-only policy.
  Workflow files were not changed or triggered. This remains a source-release
  audit blocker; it is separate from the successful Rust and packaging checks.
  README's private-repository wording versus supplied public-repository claim
  is recorded without changing visibility or assuming a remote visibility check.
- Local evidence: `target/multi-currency-phase-0/` (baseline/final Rust logs,
  performance logs, checks, bundle and synthetic GUI results), plus existing
  `target/release-audit/` dependency results. Final separator checks use the
  `space-format-*` logs; final source audit uses `source-audit-final.log`. Schema stays version 1; Phase 1 is
  **in progress**, with frozen FX and safe backed migration explicitly next.

## Phase 1 detached FX preflight and SQLite backup — 2026-10-29

- Added detached operation-FX preparation and validation, plus current-document
  and full retained-revision preflight. Copied quote/date/source and
  MonthlyDefault provenance; retained existing native/base postings, stable
  transaction-local split IDs, deterministic split equivalents, match originals
  and unchanged scheduled templates. These preparation types are not yet an
  activated v2 schema. The active app continues opening/saving version 1;
  monthly replacement remains disabled.
- **Model/integration:** **170 passed**, including **9 new upgrade tests**.
  Coverage includes all 11 currencies, quote provenance, repeated preflight,
  detached validation without monthly snapshots, JSON round-trip of individual
  locks, exact whole-budget preservation, zero-net mixed-sign splits, trailing
  zero parts, missing rates/references, duplicate account IDs, tampered base
  amounts, malformed dates/rates, schedule exclusion and hidden match originals.
  The new extreme JPY case exercises an intermediate remainder beyond i64 while
  all four final parts fit; an unrepresentable final part returns an overflow
  error. Normal split calculation now uses an i128 remainder, and current-store
  validation checks representability before accepting posted splits.
- **SQLite integration:** synthetic fresh temporary budgets only. Online backup
  uses the enabled rusqlite `backup` feature, not filesystem copying. A test
  asserts a nonempty live WAL, backs up current JSON, revisions and an additional
  table, checks independent read-only reopen and revision equality, then changes
  the source and proves recovery data remains unchanged. Other tests reject
  missing backup directories, stale preflight and a malformed retained revision,
  preserving the source and history. Backup verification checks integrity and
  exact JSON/history strings before success. No schema replacement, interrupted
  upgrade rollback, native confirmation or Undo reset is claimed in this increment.
- **Egui/binary:** all existing **124 passed**; **5 library tests** also passed.
  Total **299 Rust tests**, zero failed. Formatting, strict all-target Clippy and
  example checks passed. No new GUI flow or UI change, so no bundle rebuild,
  codesign/plist check or live GUI test was performed. No personal budget was
  opened or modified. No performance improvement or new 100k timing is claimed.
- **Release tools/dependencies:** all **14 release-tool tests** and all four
  wrapped dependency checks passed with the existing scoped advisory exception.
  Versions and inventory remain unchanged: the SQLite backup feature introduces
  no new package. Source audit: **828 candidates, 327 dependency records, one
  existing error** for automatic CI triggers; workflow files remain unchanged.
- Evidence is under `target/multi-currency-upgrade/`: `tests.log`, `clippy.log`,
  `examples.log`, `release-tools.log`, `dependencies.log`, `source-audit.log`.
  DEVELOPMENT-PLAN and implementation notes distinguish the completed preparatory
  work from the remaining active schema, exact-base/split overrides, confirmation,
  atomic upgrade/rollback and history-reset work. No Git mutation or publication.

## Quick-budget menu focus fix — 2026-10-28

- The egui event test presses and releases the budget-cell arrow, verifies the popup opens, selects its first item and checks that the resulting allocation is saved once.
- `cargo fmt --all --check`, `cargo test --locked --quiet` (250 tests), strict all-target Clippy, example checks, six release-tool tests and all four dependency audits passed.
- The macOS bundle was rebuilt; its signature and plist verified. The rebuilt executable launched with a freshly generated synthetic budget, confirmed by its open SQLite file. No live menu interaction or visual inspection is claimed.
- The existing menu has three choices; historical averages, scheduled outflows and a calculator remain unimplemented.

## Complete quick-budget menu — 2026-10-28

- Added previous-month allocation and net outflow, average monthly net outflow across up to 12 available complete months, recurring scheduled outflows and the allocation needed for a zero category balance. Calculations use integer minor units; the historical start is the first recorded transaction or allocation. Scheduled foreign-currency values use the latest available rate and are unavailable when no rate exists.
- Model/integration tests cover empty historical months, recurrence, split categories, off-budget accounts and currency-rate fallback. Calculator tests cover arithmetic, rounding and errors. Egui event tests select all five amounts and calculate and save 7 + 8.
- `cargo fmt --all --check`, `cargo test --locked --quiet` (256 tests), strict all-target Clippy and example checks passed. All four dependency audits and six release-tool tests passed. The rebuilt bundle's plist and ad-hoc signature were verified.
- On macOS, the rebuilt bundle opened `/tmp/budgettinker-quick.irlZNO/demo.budgettinker` (fresh synthetic data). The menu and calculator were visually inspected. Selecting zero balance changed the Groceries allocation from 40000 to 7114 minor units; calculator buttons then saved 7 + 8 as 1500, verified with a read-only SQLite query. No personal data was used.

## Quick-budget appearance and action verification — 2026-10-28

- The menu uses right-aligned rows and blue selection, and the calculator uses a bordered display and classic button grid. A rebuilt macOS bundle was visually inspected with the fresh synthetic budget `/tmp/budgettinker-look.aCLYzR/demo.budgettinker`; calculator buttons saved 7 + 8 as 1500 minor units, confirmed with a read-only SQLite query. The display border was added after that visual check.
- Egui tests now obtain the actual menu values from a synthetic budget, click every monetary option, and verify the allocation; the zero-balance choice is additionally checked against the resulting category balance. Missing foreign exchange rates cannot change the budget. Historical tests cover refunds and off-budget transactions; other tests cover scheduled recurrence, splits, rates, calculator sign, backspace and cancellation.
- Final checks: formatting, all 260 Rust tests, strict Clippy, example checks, six release-tool tests and all four dependency audits passed; rebuilt bundle signature and plist verified. The final bundle was visually inspected with a new synthetic budget at `/tmp/budgettinker-final.yg4hdg/demo.budgettinker`, including the calculator display border. After Cancel, selecting zero balance saved 7114 minor units, verified in SQLite read-only.

All application checks ran in the new sibling directory, not the original worktree.

- `cargo fmt --all --check`: passed after formatting renamed imports and strings.
- `cargo test --locked --quiet`: **246 passed**, zero failures: 5 library, 95 binary/egui,
  146 integration tests. Full suite rerun after the final runtime icon replacement.
- `cargo clippy --all-targets --locked -- -D warnings`: passed, no warnings.
- `cargo check --examples --locked`: passed.
- `bash -n scripts/bundle.sh scripts/icon.sh`: passed.
- `bash scripts/bundle.sh`: passed on macOS arm64, including the Swift PDF helper and icon pipeline.
- `codesign --verify --deep --strict dist/BudgetTinker.app`: passed (ad-hoc, no notarization).
- `plutil -lint dist/BudgetTinker.app/Contents/Info.plist`: passed.
- Bundle executable: Mach-O arm64; Info.plist uses `io.github.perevozchiko.budgettinker`.

The extension regression copies a closed legacy `.sqlite3` budget into a `.budgettinker`
file, loads the same full JSON, edits/saves/restores it, verifies the SQLite header and
confirms the original file bytes are unchanged. Existing suites cover atomic transfers,
bank matching, serialization, persistence and Undo/Redo.

## Live automated macOS GUI

Final running PID **50177**, window **15700**, database
`/tmp/budgettinker-release.LPniBP/demo.budgettinker` confirmed by `lsof`.
This was generated by `examples/demo.rs` in a fresh directory; SQLite read-only checks
confirmed `My Budget (Demo)` and 11 synthetic transactions. No real budget was opened.
The final bundle was relaunched after updating the runtime icon PNG.

Window was focused/sized to 1440×900 through Accessibility. A fresh real screenshot was
visually inspected and saved as `screenshots/budgettinker.png`. The title reads
BudgetTinker; the existing English UI and layout are preserved. Dock region captures in
`/tmp` were inspected: final icon is teal and comparable in visible size to adjacent icons.
Private desktop/Dock captures were not placed in the public tree.

Initial padding only in `.icns` did not fix the visible Dock icon: eframe supplied the
unpadded PNG at runtime. The final pipeline pads the shared runtime PNG and derives .icns
from it, resolving that issue. The owner additionally requested the teal sidebar texture;
built-in imagegen changed the background and the pipeline clips/normalizes transparent edges.
See ICON-EDIT.md. No fake application screenshot was generated.

## Scope and limits

Original source snapshot: **289 files** still match SHA-256.
Original Git status was clean and remains checked separately. No Git write or remote
operation was performed. Build results are generated only in the new copy, ignored by
`.gitignore`; they were not copied from the old project and must not be added to publication.

Windows/Linux were inspected in the manual-only workflow, not built or run here.
Native save-dialog interaction and import/Undo GUI flows were not re-run live during this
branding task; existing egui/integration tests passed. Human GUI acceptance is not claimed.
License ownership and final target-specific dependency-notice review remain open in the
release audit; test success is not legal clearance.


## Release blockers follow-up — 2026-10-28

All work in this phase stayed within BudgetTinker. No command opened the old project;
no application source, fixture, localization, existing artwork or data format changed.
A fresh 76-file baseline matches byte-for-byte after verification. Git history/ownership
claims from the old tree were not re-audited. New code is release/audit/packaging tooling only.

**Rust/macOS checks:** pinned installed Rust/Cargo 1.99.0, `cargo fmt --all --check`,
`cargo test --locked --quiet` (**246 passed**: 5 library, 95 binary/egui, 146 integration),
`cargo clippy --all-targets --locked -- -D warnings` and `cargo check --examples --locked`
all passed. Full tests include the legacy SQLite-to-.budgettinker extension regression,
legacy source format/bank imports, persistence and Undo/Redo. No existing data format was changed.
Bundle rebuilt; Mach-O arm64 executable, plist and ad-hoc signature verified. No new live
GUI editing or manual human acceptance is claimed; the approved image bytes are unchanged.

**Dependencies:** cargo-deny 0.20.2 installed beneath target/. Active graph license check
passes with one expected unused CC0 exception warning. Full 327-crate inventory license
check passes with zero errors/warnings. Both advisory checks exit 1 for the unsuppressed
RUSTSEC-2026-0192 unmaintained ttf-parser 0.25.1 advisory; no other vulnerability/yanked
diagnostic reported. RustSec revision and exact results are in DEPENDENCY-AUDIT.md and
local target/release-audit/dependency-results.json. No dependency version or ignore changed.
Root project licenses remain drafts; cargo-deny's private-workspace skip is not legal clearance.

**Packaging/source candidate checks:**

- 596 notice/status files match SHA-256 between shared staging and macOS bundle; all 327
  package records have collected notices. Embedded font copyright metadata and SQLite
  public-domain notice were manually reviewed and included. No font binary is copied.
- Notice-only tar and ZIP archives were created locally, reopened and compared by file hash.
  These are packaging smoke checks, not Linux/Windows binaries or PowerShell execution.
- Initial ZIP smoke check exposed pre-1980 mtimes inherited from Cargo source archives;
  staging now normalizes only copied files/directories to 2000-01-01. Source bytes/mtimes
  remain unchanged. ZIP smoke check then passed.
- Initial Cargo source-list comparison exposed unanchored `LICENSE-*` matching notices
  under generated dist/. All include patterns are now root-anchored. Final Cargo package
  list contains 698 entries (697 approved source candidates plus generated Cargo.toml.orig),
  with no target/dist/cache/.git, private data or development-only artifacts. The audit
  script now repeats this Cargo comparison rather than relying only on .gitignore.
- Source scanner verifies notice hashes, ownership/grant gate, manual-only trigger,
  secret/large-file/symlink/unsupported-claim checks. No findings in public candidates.
  Literal detector signatures in the scanner itself are treated as code, not home paths
  or ownership assertions. Upstream author addresses remain intentional legal notices.
- Workflow YAML was parsed and retains only workflow_dispatch and the four expected jobs.
  The manual validation job runs the pinned audit and correctly remains blocked by RustSec.

**Foreign-platform limitations:** target dependency graphs inspected (macOS arm64, Windows
x86_64 MSVC, Linux x86_64 GNU). Only macOS/iOS Rust targets are installed; no Windows SDK/
linker/runtime, Linux target/sysroot/host, Docker, Podman, Zig, MinGW or PowerShell is available.
Windows/Linux executables and tests were not built/run; archive legal content was verified
through the shared staging smoke checks only. No claim of platform runtime parity is made.

Root grant activation, exact owner attribution, branding approval/name review and the
maintenance-risk release decision remain explicit. See LICENSE-STATUS.md and the current
PUBLIC-RELEASE-AUDIT.md recommendation: NO-GO until those release gates are resolved.


## Portability, neutral names and structural refactor — 2026-10-28

Current completion results supersede the earlier unsuppressed-advisory failure above.
All commands ran only in BudgetTinker; the original project was not modified.

- Full Rust suite: **249 passed** (5 library, 98 binary/egui, 146 integration), no failures.
  Portable timeframe coverage remains active; palette fallback, process-argument handling
  and validation translation coverage were added. All 250 Rust declarations are retained;
  one additional declared dialog test is cfg(non-macOS) and is not run on this host.
- fmt, strict Clippy and examples passed. Python release-policy tests: **4 passed**.
- cargo-deny 0.20.2: active and full-inventory license/advisory runs all exit 0. Full inventory
  covers 327 registry crates. The active license run has one known unused CC0 exception
  warning. Advisory runs retain notes for the single owner-approved maintenance exception;
  all other advisories remain blocking. Deadline/version/direct-depender guards are tested.
- Financial comparison: 160 function bodies are token-equivalent after renamed identifiers
  and the corresponding named format-variable normalization. Comments/whitespace are ignored;
  numeric and persisted string values are not waived. No financial test body was dropped.
- Cargo.lock, both root license draft texts and all three approved icon files match the
  phase baseline hashes. Root grants remain inactive; the Cargo license line is commented.
- Large modules and integration suites are now topic-based directories. The maximum Rust
  file length is 732 lines. Test fixture include paths and visibility were corrected during
  development; intermediate compile failures from imports/paths/helper placement were fixed
  before the passing completion runs. An introduced nested-if Clippy warning was fixed.
- The Cargo source-list check caught docs/.DS_Store. Explicit negative include rules now
  exclude that metadata, AppleDouble/metadata folders, caches and private budget formats.
  The checked ZIP builder retains .github/workflows/rust.yml and .gitignore without shell globs.

Live macOS scope: the isolated synthetic demo used a confirmed BUDGETTINKER_DATA path
under target/release-step2/live-qyng0sqk. Earlier in this phase PID 58398 / window 16236
showed the rebuilt English Budget/Open screens at 1440×900. Native chooser process 60857 /
window 16264 was identified and focused. The first AX Cancel-button lookup failed and was
not accepted as evidence. Escape after focusing the native chooser closed it; process exit,
return to the budget screen and equality of the complete SQLite JSON hash were verified.
No personal source file was selected, and no live edit/Undo/Redo workflow is claimed here.
Final post-refactor bundle checks and source/archive hashes are recorded in local completion
results; original comparison screenshots or private desktop captures are not distributed.

Windows/Linux remain unbuilt/unrun: only macOS/iOS Rust targets and the macOS SDK are
available; no foreign SDK/linker/sysroot/runtime or Docker/Podman/Zig/MinGW/PowerShell.
Native chooser/PDF/printing restrictions and inherited menu limitations are listed in
PLATFORMS.md. No unsupported platform is presented as verified.

NO-GO remains for publication pending owner project/branding grants and foreign-platform
validation; the advisory exception is approved only until its review deadline.


Post-refactor live result: final bundle PID **55060**, window **17011**, same confirmed
isolated database. Budget and Open Budget screens were visually inspected; the latter
shows **Import Legacy Budget…**. The complete stored SQLite JSON still matches the original
synthetic document hash. Captures remain in target/release-step2. This confirms the final
bundle launched after module moves; it does not substitute for unperformed foreign builds.

The verified source review ZIP includes both hidden control files and contains no old
branded module filenames, generated build/cache folders, private database or macOS metadata.
Exact created/modified/removed paths are recorded locally in
`target/release-step2/changed-files.json`; historical path names stay outside public candidates.


## License finalization — 2026-10-28

The maintainer directed finalization without a separate rights-confirmation step.
The prior draft-status statements above are historical and superseded by this entry.
Root MIT/Apache texts are active; the MIT/NOTICE copyright line is 2026 BudgetTinker
contributors, using the release-kit attribution. Cargo declares MIT OR Apache-2.0.
Project artwork uses the same terms to the extent the project can grant rights;
upstream notices remain intact. No independent legal clearance or exclusive AI-art
copyright/trademark claim is made. publish = false remains an operational safeguard.

The source auditor now requires the active declaration/texts. cargo-deny checks the root
package as well as all dependencies (private ignore = false). All four dependency audits
pass; the existing scoped maintenance exception remains unchanged. Four release-tool
tests, formatting and source/archive checks pass. Legal material is restaged into the
macOS bundle, its ad-hoc signature is refreshed/verified, and the local source ZIP is rebuilt.
Application source, fixtures, translations, image bytes and Cargo.lock match the baseline
of this licensing-only step. No new live GUI or foreign-platform check is claimed.
No project repository, commit, publication or upload was created.

## Legacy name cleanup — 2026-10-28

- Source text and synthetic fixtures retain legacy serialized values through string fragments and JSON Unicode escapes; the ignore and release-audit rules still exclude the legacy directory format.
- Case-insensitive text and filename searches, excluding build output and Git metadata, found no contiguous legacy brand names.
- `cargo fmt --all --check`, `cargo test --locked --quiet` (249 tests), strict Clippy and example checks passed.
- `scripts/test-release-tools.py` passed 4 tests; `scripts/audit-dependencies.py` passed all four checks.
- No GUI or packaging behavior changed; no live GUI check was performed.

## English source migration — partial, 2026-10-28

Exchange-rate errors now originate in English. Formatting, all 249 Rust tests, strict Clippy, example checks, four release-tool tests and dependency audits passed. The remaining source translation catalogue and Russian literals have not yet been migrated. No live GUI check was performed.

## Post-migration repository cleanup — 2026-10-28

The private Git checkout is on `master`; the initial source commit predates this task.
No application, financial, schema or dependency code was changed. The original project
was not touched. No commits, pushes, releases or visibility changes were made.

- `cargo fmt --all --check`, strict all-target Clippy and `cargo check --examples --locked`
  passed. `cargo test --locked --quiet` passed **249 tests**: 5 library, 98 binary/egui
  and 146 integration. No separate live GUI check was performed for this tooling change.
- `scripts/test-release-tools.py` passed **5 tests**, including a temporary Git fixture,
  archive exclusions, hidden files, reproducibility and rejection of changed source hashes.
- `scripts/audit-dependencies.py` passed active and full-inventory license/advisory checks
  with the existing scoped exception. The active license check retains the known unused
  CC0 exception warning; advisory notes retain the approved maintenance exception.
- Source audit passed in the Git checkout; Cargo's generated `.cargo_vcs_info.json` is
  recognized in its package list but is not a source ZIP candidate. The ZIP builder
  verifies each archived hash and includes the hidden workflow and ignore file.
- macOS ARM64 bundle rebuilt; executable, icon, bundled notices, plist and ad-hoc
  signature verified. No notarization or new live GUI check is claimed.
- Workflow YAML and each job condition were checked locally; only `workflow_dispatch`
  triggers it. No GitHub workflow was run; Windows and Linux builds remain unverified.
- All 798 tracked paths were checked against source exclusions; none matched. Secret and
  inventory checks passed on source candidates. This does not establish legal clearance.


## Calculator precision review fixes — 2026-10-28

- Calculator operands and intermediate results now use reduced, checked i128 fractions. Only the final result is rounded to currency minor units (ties away from zero). Decimal operands are independent of currency precision; no dependency or persisted money representation changed. Expressions exceeding the bounded fraction representation report an error.
- Arithmetic regressions cover `1 / 3 * 3`, `0.01 / 3 * 300`, sums of recurring fractions, negative divisors, `1000 * 1.1` in JPY, `100 * 1.075` in USD, fractional divisors, comma decimals, USD/JPY/KWD rounding, invalid input and i64 boundaries. Egui button tests verify OK writes 100 and 10750 minor units exactly once for the two USD regressions.
- Model/integration and egui: all **263 Rust tests** passed (5 library, 108 binary/egui, 150 integration). Formatting, strict all-target Clippy and example checks passed. Six release-tool tests and all four dependency audits passed with the existing scoped advisory exception.
- Live macOS: rebuilt bundle, verified ad-hoc signature and plist. PID 90957 / window 17437 opened fresh synthetic `/tmp/budgettinker-calculator-9olj27yp/demo.budgettinker`, confirmed by lsof. The calculator and resulting budget were visually inspected. Entering `0.01 / 3 * 300` and pressing OK saved 100 minor units; `100 * 1.075` then saved 10750. Both values were verified through read-only SQLite queries. The demo uses EUR (two decimal places); JPY and KWD were verified in automated tests. Screenshots and logs remain under target/. No personal data was opened.


## Pre-commit validation after release-version pull — 2026-10-28

- Fast-forwarded to `21c2f5b` (`0.1.0-alpha.1`) while preserving the quick-budget/calculator changes.
- All **264 Rust tests** passed (5 library, 109 binary/egui, 150 integration). Formatting, strict all-target Clippy and example checks passed. Seven release-tool tests and all four dependency audits passed with the existing scoped advisory exception.
- Live GUI/bundle evidence above predates this version-only pull; no additional live GUI acceptance is claimed here.

## Account ordering work in progress — 2026-10-29

- BudgetTinker supports dragging within a displayed group through the save/undo path. The Rust suite (264 tests), formatting, strict Clippy and example checks passed. No dedicated reorder test or live interaction verification was completed.
- macOS bundle rebuilt; codesign and plist checks passed. It was launched with fresh synthetic data; reorder and persistence were not checked live.

## Account drag animation — 2026-10-29

- BudgetTinker draws the dragged account as a translucent floating row and animates surrounding rows towards the prospective position.
- Model/integration and egui: existing 264 tests passed before the final pointer-offset adjustment; strict Clippy passed after it. No dedicated animation event test was added.
- Live GUI: inspected a synthetic drag in the rebuilt app and identified an incorrect horizontal offset; the offset was subsequently corrected, but that final correction has not been inspected live. Bundle signature and plist passed after rebuilding. Release-tool tests and dependency audits passed.

## Account drop clipping review fix — 2026-10-29

- Drop targets now require the pointer inside both the account row and the scroll UI clip rectangle, preventing hidden rows from accepting drops over controls below the list.
- Removed three legacy brand mentions from documentation. Case-insensitive searches of all source-tree text (including hidden/ignored files, excluding Git metadata and generated target/dist output) found no remaining contiguous short or expanded brand names; separator variants were checked too.
- Model/integration and egui: all 264 existing Rust tests passed (5 library, 109 binary/egui, 150 integration). No dedicated drag regression event test was added. Formatting, strict all-target Clippy and example checks passed; seven release-tool tests and all four dependency audits passed with the existing scoped exception.
- macOS bundle rebuilt; strict signature and plist checks passed. Rebuilt app PID 81741 / window 18068 was visually inspected with fresh synthetic `/tmp/budgettinker-drop-j5ftdy8_/demo.budgettinker`, confirmed by lsof. Screenshot is under target/drop-fix-window.png. This checks launch/appearance only; scrolling drag behavior and persistence were not exercised live.

## Month quick-budget menu and summary interactions — 2026-10-29

- The month menu previously only copied the preceding month and overwrote filled cells.
  It now offers previous allocations, previous net outflows, averages over up to three
  or twelve complete months, scheduled outflows, zero category balance and zero allocations.
  The first six commands fill only zero-valued allocations (including stored zeros and
  hidden categories); zero allocations replaces every category's amount for that month.
  All values are computed before mutation, so missing scheduled exchange rates cannot
  leave a partially filled month. The existing commit path saves the whole change once.
- Model/integration: new regressions cover the three-month history cap, empty months,
  integer rounding, allocation-only history, invalid month rejection and atomic failure
  on a missing scheduled rate. Income details cover deferred income, split income,
  off-budget exclusion, internal transfers and frozen foreign-currency amounts. Currency
  revaluation is shown separately from categorized income, matching the card's total.
- Egui: actual pointer events open the month popup and select each of its seven entries;
  each test checks allocation results, positive/negative filled-cell preservation, hidden
  categories, SQLite persistence and a single Undo/Redo restoring the complete document.
  Further event tests check missing-rate errors without persistence/Undo changes, global
  summary collapse/expand and saved preferences, income popup opening and exact-month
  Account View filtering (including income received in the previous month).
- All **274 Rust tests** passed: 5 library, 113 binary/egui and 156 integration tests.
  Formatting, strict all-target Clippy and example checks passed. All four dependency
  audits passed with the existing scoped advisory exception. Release-tool tests: **6/7
  passed**; the existing workflow's `push` trigger conflicts with the manual-only policy
  test. This unrelated CI configuration was not changed; no full release-tool pass is
  claimed.
- Clicking Available to Budget collapses all displayed month cards; clicking again
  expands them. Income opens a transaction table with Show in Account View and Close.
- macOS bundle rebuilt; strict ad-hoc signature and plist verification passed. Final
  executable PID **40448**, window **18922**, opened the fresh synthetic budget
  `/tmp/budgettinker-month-final-kra3hywa/demo.budgettinker`, confirmed with lsof.
  Live month-menu checks compare every category allocation through read-only SQLite:
  November totals are 266500 (previous budget), 100752 (previous outflows and both
  available-history averages), 2990 (scheduled), -165748 (zero balance), and 0 (zero
  allocations), in minor units. Each command is undone through the native shortcut;
  the complete stored document must equal the original synthetic document afterwards.
  GUI scripts, results and synthetic screenshots remain under `target/month-menu/`.
- Final bundle appearance: inspected collapsed cards and the income table, then used
  Show in Account View to display the three synthetic income rows totaling 512600 minor
  units. Complete SQLite document equality with the original synthetic budget passed
  after all seven menu commands plus Undo, collapse/expand and income navigation.

## Budget cell editor geometry and arrow — 2026-10-29

- Reproduced the row movement in a new egui regression: entering a budget cell moved
  following rows upward by two pixels at 1050px window width. The input's scope and
  the arrow's `ui.put` both advanced the parent layout despite the row already having
  its space allocated. The input now uses an independent child UI and the arrow uses
  an explicit interaction rectangle; neither changes the parent cursor.
- The editor uses a compact square gray menu button outside the cell, with a black
  downward triangle. The button is 16x16px, vertically centered, with a drawn triangle
  and pressed gradient; it no longer depends on the font's tiny dropdown glyph.
- Egui: the regression compares every category row rectangle before editing, after
  entering the input, after opening quick-budget and after cancellation at both 1050px
  and 1440px widths. All rectangles are identical; cancellation leaves the complete
  SQLite document unchanged with no Undo entry. Existing editor tests continue to
  cover focus, keyboard navigation, menu choices, calculator and persistence failures.
- Model/integration and egui: all **275 Rust tests** passed (5 library, 114 binary/egui,
  156 integration). Formatting, strict all-target Clippy and example checks passed.
  All four dependency audits passed. Release-tool tests retain the previously documented
  **6/7** result because of the existing manual-only CI policy conflict.
- Native GUI: rebuilt macOS bundle, verified strict ad-hoc signature and plist. PID
  **57415**, window **19017**, opened fresh synthetic
  `/tmp/budgettinker-cell-y6qixxd2/demo.budgettinker`, confirmed through lsof.
  Inspected the active Household Goods editor and its menu; screenshot grid-line
  coordinates below the active row stayed identical before/after editing. Opening
  the editor did not change SQLite. Selecting Balance to 0.00 for Groceries then saved
  **7114** minor units, checked through a read-only connection; native Undo restored
  the complete original document. Synthetic captures/logs remain under `target/cell-editor/`.
- After committing the editor fix, merged fresh `origin/master` at **96b4e0d** without
  conflicts. Its release-test policy update supersedes the earlier CI-test failure:
  `scripts/test-release-tools.py` now passes **all 8 tests**. The incoming master change
  only updates those policy tests; no application or dependency code changed in the merge.

## Documentation maintenance — 2026-10-29

- Product notes describe BudgetTinker's controls and behavior directly. Asset descriptions
  retain their sources and license terms; dependency notices are unchanged.
- Documentation-only changes: all 275 Rust tests, formatting, strict Clippy, example
  checks, eight release-tool tests and all four dependency audits passed.
- Rebuilt the macOS bundle with the updated attribution. Packaged attribution matches
  the source; strict ad-hoc signature and plist checks passed. No additional live GUI
  check was performed for this documentation-only update.

## Reports filter panels — 2026-10-29

- Replaced report filter combo menus with anchored classic panels: title band, pointer,
  blue selection links, compact silver/blue checkboxes and pinned Cancel/Done buttons.
  All five reports share the controls. Net Worth disables categories/payees and starts
  with All Accounts; other reports start with Budget Accounts. Each report now retains
  its own period and selections in budget-local preferences, with defaults for older files.
- Timeframe offers This Month, Latest 3 Months, This Year, Last Year and All Dates.
  Quick presets apply immediately and close the panel. Custom ranges use month/year
  selectors and require Done; Cancel, Escape and outside dismissal discard drafts.
  Relative/all-date presets include complete calendar months. Last Year is disabled
  when the budget has no history before this year. Empty multi-selections disable Done.
  Category groups support partial selection, hidden categories have an aggregate checkbox,
  and accounts are grouped into budget, off-budget and closed accounts.
- Model/integration: existing financial and persistence suites pass; period regressions
  cover previous-year boundaries, three-month rollover and leap-year month endings.
  No transaction, monetary calculation or budget-document schema changed.
- Egui: pointer events exercise custom ranges, confirmation/cancellation, invalid-range
  rejection, immediate presets and account selection in every report. Spending reports
  additionally apply category/payee selections; Net Worth checks disabled filters and
  a filtered net-worth total. Existing category events now cover the hidden-category
  aggregate. Tests verify separate tab settings, restart persistence, unchanged complete
  SQLite documents and no budget Undo entries from filter changes.
- Required checks passed: formatting, **279 Rust tests** (5 library, 118 binary/egui,
  156 integration), strict all-target Clippy and example checks. Release-tool tests:
  **11 passed**. All four dependency audits passed with the existing scoped advisory
  exception. The macOS bundle was rebuilt; strict ad-hoc codesign and plist checks passed.
- Native macOS: final rebuilt executable PID **58381**, window **20405**, opened fresh
  synthetic `/tmp/budgettinker-reports-final-uwa_qzd3/demo.budgettinker`, confirmed with
  lsof. Visually inspected all four filter panels and all five report screens. Native
  actions selected the off-budget account group, four Monthly Bills categories and
  Coffee Shop, then applied This Year. Each other report applied Latest 3 Months;
  returning to the first report restored its distinct filters. Assertions read the
  saved budget-local preferences after each action. A read-only SQLite connection
  verified the entire document still equals its original synthetic baseline.
  Custom-range confirmation and invalid-range behavior were checked in egui, not
  claimed as completed native interaction checks. Synthetic screenshots, the native
  action script/results and build/audit logs remain under `target/reports-filters/`.
- Reference application inspection used its already-open synthetic laboratory budget;
  reference captures remain private under target/. No personal budget was opened,
  no reference assets were added to source, and no publication or commit was performed.

## Report filter hover correction — 2026-10-29

- Enabled Timeframe, Categories, Payees and Accounts links now show the blue pill with
  white text and arrow immediately on hover. Open panels keep the same highlight;
  disabled Net Worth filters remain gray.
- Model/integration and existing egui checks: all **279 Rust tests** passed. Formatting,
  strict Clippy, example checks, 11 release-tool tests and all four dependency audits
  passed. This visual correction adds no new test that duplicates its implementation.
- Rebuilt the macOS bundle and verified strict codesign and plist. PID **74832**, window
  **20552**, opened fresh synthetic `/tmp/budgettinker-report-hover-h753zaz4/demo.budgettinker`,
  confirmed with lsof. CoreGraphics pointer movement without clicking produced the
  highlight on all four controls in Income v. Expense; moving over disabled Categories
  in Net Worth produced no highlight. Synthetic screenshots were visually inspected
  and remain under `target/reports-filters/hover-*.png`. A read-only SQLite comparison
  confirmed the complete budget document is unchanged.

## Account edit popup and sidebar highlights — 2026-10-29

- Work is on `fix/account-edit-dialog`, created from local `master` with a clean working tree. No commit, push or publication was performed.
- Right-click opens a 374px account popup below the corresponding row without a dimmed backdrop. Name and Note use bordered inset fields; Type is aligned with them. Adjust Balance keeps the parent draft, budget membership changes in the draft, and the footer offers Close/Reopen Account, Cancel and a green Done button. Existing account validation, persistence and Undo/Redo remain in use.
- Hovering or editing an account uses a lighter blue (RGB 0/112/151), distinct from the selected register account (RGB 0/72/109). The edit highlight persists after the pointer leaves the row, including while the balance-adjustment child dialog is open.
- Model/integration: the complete suite passed **280 Rust tests** (5 library, 119 binary/egui, 156 integration). Existing financial and persistence suites pass. No money, transfer or stored schema implementation changed.
- Egui: five new tests exercise real pointer events for hover, right-click opening, both simultaneous highlight colors, popup/footer geometry, budget toggling, Cancel, Done, Close/Reopen and opening the balance-adjustment child. They verify draft preservation, rejecting nonzero account closure without data/Undo changes, saving name/type/note/membership in one change, SQLite persistence and complete-document Undo/Redo. Escape from the child returns to the unsaved account draft.
- Formatting, strict all-target Clippy and example checks passed. All four dependency audits passed with the existing scoped advisory exception; all **11 release-tool tests** passed.
- Native GUI: the macOS ARM64 bundle was rebuilt; strict ad-hoc signature and Info.plist checks passed. Final PID **49541**, window **19807**, opened fresh synthetic `/tmp/budgettinker-account-colors-3039gcut/demo.budgettinker`, confirmed by lsof. Selected Checking in the register, then right-clicked Cash. Visually inspected the anchored popup, field borders and distinct dark Checking/lighter Cash highlights. A read-only SQLite comparison confirmed the complete stored document was unchanged after navigation and opening the popup. Final synthetic capture: `target/account-dialog/final-colors.png`; logs and test captures stay under target/.
- Live edit/save/Undo and balance-adjustment acceptance are not claimed; account mutation and Undo/Redo were verified through egui events and SQLite above. An earlier scripted native edit sequence did not reach its expected save and was not accepted as evidence. No external application screenshot or personal budget was added to the source tree.

## Documentation wording cleanup — 2026-10-29

- Icon-edit notes describe the requested colors, texture and geometry directly. The palette module documents its color space. Project asset provenance, artwork terms, dependency attribution and upstream license texts are retained.
- Product documentation was searched for external-product names and visual-development wording. Import compatibility details and data-safety instructions remain intact.
- The account-popup implementation and checks recorded above are included in the same branch. This follow-up changes prose and a Rust module comment; no additional UI or data behavior changed.

- Final follow-up checks: all 280 Rust tests, formatting, strict Clippy, examples, all four dependency audits and 11 release-tool tests passed. The bundle was rebuilt after the documentation cleanup; strict signature and plist checks passed. Fresh launch PID **60772** uses `/tmp/budgettinker-fresh-8vd5lcse/demo.budgettinker`; no additional editing workflow was exercised during this launch.

## Sidebar selection priority — 2026-10-29

- Selected register accounts now retain RGB 0/72/109 while hovered or edited. Other hovered/edited rows use the slightly lighter RGB 0/92/128. This supersedes the earlier RGB 0/112/151 hover color and fixes hover/edit incorrectly taking priority over selection.
- Model/integration: all **280 Rust tests** passed (5 library, 119 binary/egui, 156 integration); no financial or persistence implementation changed. Formatting, strict all-target Clippy and example checks passed. All four dependency audits and all 11 release-tool tests passed.
- Egui: extended the sidebar event regression to move onto the selected row, onto another row, and away from the rows. It verifies that selection stays dark, other-row hover differs, the edited row stays highlighted, and opening the selected account editor preserves its dark selection while clearing the previous row's hover.
- Native GUI: rebuilt bundle; strict signature and plist checks passed. PID **66454**, window **19879**, uses fresh synthetic `/tmp/budgettinker-highlight-4r_0udcz/demo.budgettinker`. Visually inspected the final popup and measured rendered screenshot pixels: Checking stays 0/72/109 while hovered; moving to Cash changes only Cash to 0/92/128; opening Cash's popup preserves both colors. Complete-document SQLite comparison through a read-only connection remained equal after these interactions. Captures and exact pixel results are under `target/account-highlight-priority/`. No live account mutation was exercised.

## Rounded popup pointers and dialog buttons — 2026-10-29

- Account, category/group, note, hidden-category and register timeframe popups now share pointer rendering. The rounded tip and shoulders join the popup's actual top border; a filled strip covers that border only at the attachment, and the outer contour remains open at the base. Pointers use the popup's own layer and matching header/border colors. A popup placed above its trigger does not draw an inverted top pointer.
- Category/group/note dialogs share 60×27 Cancel/Done buttons with consistent spacing, 13px text, disabled styling and keyboard-focus outlines. Notes use plain silver Cancel and blue Done, replacing the colored icon sections. Category/group input and link text sizes were reduced. Removed the now-unused general-button success mode and Cancel glyph; other button behavior and account-specific Done styling are preserved.
- Model/integration: all **280 Rust tests** passed (5 library, 119 binary/egui, 156 integration). No financial calculation, validation or storage implementation changed. Egui: the existing event suites passed, including account popup geometry, actions, drafts, persistence and Undo/Redo, plus register timeframe interactions. No new graphical regression test is claimed for this visual change.
- Formatting, strict all-target Clippy and examples passed. All four dependency audits passed with the existing scoped advisory exception; all 11 release-tool tests passed. The macOS ARM64 bundle was rebuilt; strict ad-hoc signature and Info.plist checks passed.
- Native GUI: final PID **4935**, window **20037**, uses fresh synthetic `/tmp/budgettinker-popup-final-2r4hjdme/demo.budgettinker`, confirmed by lsof. Visually inspected account, category, group, note, timeframe and hidden-category popups. Rounded joins have no horizontal base stripe; category/group/note buttons align and use the same controls. Captures are under `target/popup-join-final/`.
- Read-only SQLite comparison confirmed that opening/canceling the first four dialogs and navigating to the register left the complete document unchanged. To inspect the hidden-category popup, hid Rent/Mortgage through the actual UI, opened Unhide, then used native Undo; a second comparison confirmed restoration of the complete original document. Finished with the revised note dialog open. No personal data or supplied screenshots were added to the source tree.

## Merge conflict resolution — 2026-10-29

- Resolved the conflicts in DEVELOPMENT-PLAN.md and this log by retaining both report-filter and account/popup entries. A line-subsequence comparison against index stages 2 and 3 verified that every line from both sides remains in its original order. No application or workflow code was edited during resolution.
- Model/integration and egui: all **284 Rust tests** passed (5 library, 123 binary/egui, 156 integration). Formatting, strict all-target Clippy and example checks passed. All **14 release-tool tests** and all four dependency audits passed with the existing scoped advisory exception.
- No new bundle rebuild or native GUI check was performed for this documentation conflict resolution. Earlier branch-specific GUI evidence above remains historical; no combined native GUI verification is claimed. No commit, push or publication was performed.

## Phase 1 explicit v2 upgrade and active operation FX — 2026-10-29

- Added explicit Budget Properties → Upgrade currency data confirmation. Cancel
  and Escape preserve document/history and create no recovery directory. Success
  activates v2 only after verified online backup and atomic replacement of the
  current JSON plus every retained revision. Startup/new-budget creation still
  writes v1 until that file is explicitly upgraded. Ordinary save rejects schema
  transitions and v2 base-currency changes. Existing v1 legacy replacement/Undo
  remains compatible; v1 replacement into v2 is rejected pending Phase 5.
- V2 operations and hidden matched originals persist independently validated FX
  locks. Readers use copied quotes/base sums, including stable split identities
  carried through the editor. New posting/starting-balance/transfer/schedule and
  split-peer paths create or retain locks. Exact-base/split-base command validation
  is atomic and preserves overrides during metadata edits; the manual FX editor
  controls and monthly-rate editing remain outstanding. The properties base
  selector is creation-only; its format example uses space grouping and ISO code.
- **Model/integration:** **176 integration + 5 library tests passed**. New coverage
  verifies activation/reopen, JSON equality, every retained version, preserved
  balances/category activity, exact backup contents, copied FX independence from
  changed/removed monthly quotes, hidden originals/Unmatch, exact-base/split
  overrides, preserved split identity after deletion, new accounts/transfers and
  schedule posting. Corrupt locks, stale preflight, backup failure, downgrade and
  base changes are rejected. SQLite triggers inject both ABORT and ignored
  revision updates after the document update; all document/history writes roll
  back together, recovery copies remain usable, and reopening sees v1.
- **Egui/UI and binary:** **126 tests passed**, including two new pointer-event
  upgrade tests. They enter through properties, cancel, press Escape, confirm,
  verify exact recovery data and SQLite state, clear both populated Undo/Redo
  stacks, and exercise ordinary commit/Undo/Redo after activation. A stale source
  leaves in-memory data and both history stacks unchanged. Total final Rust
  result: **307 passed**, zero failed. The first full run exposed five v1 legacy
  replacement regressions from an over-broad storage base lock; the guard was
  restricted to v2, preserving v1 replacement, and the full suite rerun successfully.
- Required formatting, strict all-target Clippy and example checks passed on the
  final code. All **14 release-tool tests** and all four wrapped dependency audits
  passed, retaining only the approved font-stack exception and its deadline guard.
  Source audit reports **829 source candidates / 327 dependency records / 1
  error**: the pre-existing non-manual CI push triggers. No workflow was changed
  or triggered; no full source-release audit pass is claimed.
- **Packaging:** rebuilt macOS ARM64 bundle with the final code; notice staging
  verified all 327 records. Strict deep codesign verification and Info.plist lint
  passed. No new assets or dependencies, no Windows/Linux validation.
- **Live native GUI:** LaunchServices-launched PID **35055**, window **21261**,
  fresh isolated synthetic budget
  `/var/folders/zw/f8bg791d14z0yj4hhkq566ch0000gn/T/budgettinker-upgrade-live-rgqzrrzl/demo.budgettinker`.
  Added synthetic USD split and zero-net mixed-sign rows plus two retained
  revisions before launch. Accessibility/CoreGraphics entered properties and the
  confirmation, canceled with complete SQLite equality and no backup, reopened,
  then confirmed upgrade of **13 operations / 2 revisions**. Read-only SQLite
  verified every original field and every retained ID/timestamp after stripping
  only v2 lock/identity metadata; the recovery copy equals the complete original
  current document and history. Frozen split bases are `[-1, -1, -1]` and
  `[-5, 3, 2]` minor units respectively. Inspected properties, confirmation and
  success captures; the success dialog identifies the usable recovery file.
  Native Command-Z did not revert the schema or change the upgraded document.
  Undo-stack population/reset itself was verified in egui above.
- **Live reopen:** restarted the same rebuilt bundle on that isolated upgraded
  file: PID **38345**, window **21285**. Inspected the reopened budget; read-only
  full-document equality with the upgraded result passed. `lsof` confirms only the
  intended synthetic database/WAL. Live mutation of quotes/exact-base entries and
  simulated storage failures are model/integration evidence, not claimed native
  interaction coverage. No personal budget was opened or modified; no human
  acceptance, commit, push or publication was performed.
- Evidence remains under `target/multi-currency-activation/`: final `*-verified`
  and `*-final` Rust/build logs, dependency/release/source audits, native scripts,
  read-only comparisons, synthetic screenshots and process/database metadata.
  Phase 1 remains in progress: creation/account currency controls, inline missing
  rates, explicit lookup acceptance and monthly valuation lifecycle are next.


## Phase 1 operation FX editor — 2026-10-29

- Added v2 ordinary foreign-operation FX fields: canonical base-per-account quote,
  quote date, exact signed base amount and exact signed split base parts. Current
  monthly or latest earlier quotes may be explicitly selected; older suggestions
  are labeled Stale. Manual edits retain Manual provenance; posting does not
  create or edit monthly valuation quotes. Exact-base-only input derives a
  reference using integer arithmetic while preserving the entered base amount.
- **Model/integration:** **178 integration + 5 library tests passed**. Two new
  integration tests cover no-monthly-quote posting, preserving confirmed FX through
  repeated command validation, native balance, exact split sums, malformed rates,
  unchanged input on rejection, metadata edits, SQLite save/load and full-document
  validation. Existing migration, historical revision, transfer, import, schedule
  and split regressions pass.
- **Egui/UI and binary:** **130 tests passed**, four more than before this increment.
  Pointer events enable FX, type an invalid comma rate and select exact base mode;
  the draft retains invalid text/payee and subsequently posts valid fixed-point FX.
  Split tests reject a one-cent mismatch and preserve IDs/base entries on reopen.
  An earlier suggestion is visibly labeled Stale, stays unselected until explicit
  selection and does not allow silent posting. Another UI-state test verifies
  complete-document SQLite commit/Undo/Redo and injected save failure preserving
  the in-memory document, persisted document and both history stacks.
  Total Rust result: **313 passed**, zero failed.
- Required formatting, strict all-target Clippy and example checks passed.
  Initial compile checks exposed a new helper visibility/import error; these were
  fixed before the successful suite. Strict Clippy then identified two collapsible
  conditions; both were corrected without changing behavior. All **14 release-tool
  tests** and all four wrapped dependency audits passed, preserving the scoped
  font-stack exception and deadline guard. No new assets or dependencies.
- **Packaging:** rebuilt final macOS ARM64 bundle, staged all 327 notice records,
  verified strict deep codesign and Info.plist lint. Windows/Linux remain untested.
- **Live native GUI:** final rebuilt bundle, initially PID **57839**, window
  **21331**, opened freshly generated synthetic v2 fixture
  `/var/folders/zw/f8bg791d14z0yj4hhkq566ch0000gn/T/budgettinker-fx-editor-lbuv0f4p/demo.budgettinker`.
  Accessibility/CoreGraphics selected the empty RSD account and opened a new row.
  Missing monthly quote blocked Save without changing the complete SQLite
  document. Entering `0,00855` in FX showed an error and retained the draft.
  Replacing it with `0.00855000` and entering exact `-10.09 EUR` saved actual
  `-1 180.00 RSD` as **-118000 native / -1009 base minor units**, with the copied
  manual rate **855000**, exact-base flag and payee intact. Read-only SQLite
  verified all other document fields remained identical and no monthly quote was
  created. Native Undo restored the complete original document; Redo restored
  the complete posted document. Inspected missing/invalid/confirmed/saved captures.
- **Live reopen:** restarted the same final bundle, PID **60980**, window **21354**;
  read-only SQLite integrity check and complete equality to the posted result
  passed. Inspected the reopened budget: missing monthly valuation is unavailable,
  while the native RSD balance persists. `lsof` confirms the isolated database/WAL.
  Native split editing, exact-base-only input, stale selection and save failures
  are automated evidence above, not claimed native interaction coverage.
- Evidence: `target/multi-currency-editor/` contains tests/checks, dependency and
  release-tool logs, bundle/signature/plist logs, native drivers, synthetic captures,
  read-only comparisons and process/database metadata. No personal budgets were
  opened or modified; no Git mutation, workflow trigger or publication occurred.
  Phase 1 remains in progress; monthly lifecycle/corrections, currency creation
  controls, reciprocal quote entry and explicit optional lookup acceptance remain.
- Final source audit: **832 source candidates / 327 dependency records / 1 error**,
  the existing CI push triggers conflicting with the manual-only policy. Workflow
  files were not changed or triggered; a full source-release audit pass is not claimed.
  Final evidence uses `tests-verified.log`, `clippy-final.log`, `fmt-final.log`,
  `examples.log`, `bundle-final.log`, `codesign-final.log` and `plist-final.log`.

## Phase 1 monthly FX lifecycle command foundation — 2026-10-29

- Added v2 current-month quote replacement, explicit finalization after month
  end, Incomplete/Provisional/Finalized valuation status and independently
  persisted lifecycle/audit metadata with serde defaults. Existing upgraded
  months remain unfinalized until explicitly finalized. Past-month entry or
  replacement uses an opaque correction proposal with a reason and prior/new
  quote metadata. A changed document invalidates the proposal; posting locks,
  native balances and category flows remain untouched.
- Correction proposals show before/after all-account Net Worth, on-budget stock,
  Available and frozen-flow residual revaluation for the selected and following
  months. Missing comparison quotes remain unavailable. Separate account previews
  may use an earlier quote with its month/date and Estimated flag; neither those
  previews nor missing quotes become authoritative totals. Zero exposure requires
  no quote. Net Worth accumulation now checks the combined i128 total before
  returning i64; an overflowing correction proposal fails without mutation.
- **Model/integration:** **185 integration + 5 library tests passed**. Seven new
  integration tests cover exact selected/adjacent-period residuals, frozen posting
  and category equality, current-month edits, explicit finalization, missing and
  earlier/future quote boundaries, unavailable previews, zero exposure, stale
  approval, legacy guards/defaults, invalid quotes/audit/finalization timestamps,
  preserving legacy empty source text, SQLite reopen, separate off-budget debt
  and on-budget asset effects, and combined valuation overflow rejection.
- **Egui/UI and binary:** **131 tests passed**. The new App-state test exercises
  correction and finalization as individual commits, complete-document SQLite
  Undo/Redo (including audit and lifecycle state), and a trigger-injected save
  failure preserving memory, stored document and both history stacks. This is
  App-state coverage, not pointer-event coverage of new monthly controls. Total
  final Rust result: **321 passed**, zero failed.
  The final future-quote exclusion assertion was then checked by rerunning all
  seven monthly lifecycle integration tests; formatting, Clippy and examples
  passed again (`targeted-final.log`).
- Formatting, strict all-target Clippy and example checks passed. All **14
  release-tool tests** and all four wrapped dependency audits passed with the
  existing approved font-stack exception/deadline. The source audit reports
  **834 source candidates / 327 dependency records / 1 error**: existing CI push
  triggers violate the manual-only policy. No workflow was changed or triggered.
- **Native GUI/packaging:** not run for this model/command/storage increment;
  product widgets and package configuration are unchanged. No new assets or
  dependencies. Monthly editor/confirmation controls, report status badges,
  lookup acceptance, transit exposure and the full Phase 4 classification/bridge
  presentation remain outstanding. No live acceptance or completion of
  AC-09/AC-16 is claimed; the existing Income display is not replaced here.
- Evidence: `target/monthly-fx-lifecycle/tests-final.log`, `fmt-final.log`,
  `clippy-final.log`, `examples-final.log`, `dependencies.log`, `release-tools.log`
  and `source-audit.log`. Tests use fresh synthetic temporary stores. No personal
  budget was opened or modified; no Git mutation, publication or release occurred.
