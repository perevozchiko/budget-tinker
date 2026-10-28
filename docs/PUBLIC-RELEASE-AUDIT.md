# BudgetTinker public-release audit — current local result

**Licensing complete:** project-owned material is MIT OR Apache-2.0, with separate third-party
terms retained. The maintainer instructed activation without a separate rights-confirmation
step. macOS technical checks passed; Windows/Linux build/runtime verification is outstanding.
The private repository is [perevozchiko/budget-tinker](https://github.com/perevozchiko/budget-tinker),
with primary branch `master` and an initial source commit. The project is not publicly
released and no GitHub Releases have been published. Tool caches and verification budgets
live only under target/. Earlier migration work was performed before repository creation;
the historical verification below describes that phase.

## Completed release tasks

1. `deny.toml` ignores only RUSTSEC-2026-0192, with owner approval, reason, dependency chain,
   fixed font inputs and review deadline **2027-01-28 UTC**. All other advisories remain
   blocking (`unmaintained = all`, `unsound = all`, yanked = deny). The pinned audit wrapper
   enforces parser/font-stack versions, the only direct depender and the deadline because
   cargo-deny 0.20.2 itself does not support per-ignore expiry/allow fields.
2. The timeframe preset/Done/Cancel/Custom test uses existing bundled egui fonts; it is not
   skipped outside macOS. Missing runtime bold/italic families get bundled fallbacks;
   already installed macOS families are preserved. No OS font binary is copied or added.
3. Native file/folder dialogs are isolated in src/desktop.rs. macOS chooser behavior and
   cancellation remain available; unsupported platforms receive explicit English errors.
   Folder reveal uses open -R / explorer.exe / xdg-open as appropriate. Non-report printing
   returns an explicit unsupported result before creating orphaned files outside macOS.
   Optional rate lookup uses curl on PATH outside macOS. See PLATFORMS.md for exact limits.
4. Actual `.github/workflows/rust.yml` and `.gitignore` exist. CI retains only workflow_dispatch.
   The source ZIP builder includes both hidden paths explicitly, reopens the ZIP and verifies
   every entry/hash. Cargo source candidates exclude builds, data, caches and macOS metadata.
   Root-anchored include patterns and explicit negative rules prevent .DS_Store/AppleDouble leaks.
5. Missing product validation translations, startup errors and raw error-label boundaries
   were corrected. Existing user names/memos, legacy parse strings and stored values are
   not rewritten. No layout, spacing, palette or styling redesign was introduced.
6. Large production and integration files were split by responsibility. UI, model, importer,
   print layout/native helper, widgets, theme and tests have directories and shared fixtures.
   The largest Rust file is 732 lines. See REFACTORING.md for before/after entry sizes.
7. Importer files/API, dialogs, examples, test paths, product-owned IDs and visible captions
   use neutral names. Captions use Legacy Budget and Starting Debt. Historical wire keys,
   debt identifiers and private-budget exclusion patterns remain only for compatibility;
   no old branded module filenames remain in public candidates. Opaque old link IDs remain
   readable; new generated import links use a neutral namespace. No existing budget is migrated.

## Actual verification

| Check | Result |
| --- | --- |
| cargo fmt --all --check | Passed. |
| cargo test --locked --quiet | **249 passed**: 5 library, 98 binary/egui, 146 integration; no failures. |
| cargo clippy --all-targets --locked -- -D warnings | Passed, no warnings. |
| cargo check --examples --locked | Passed. |
| Python release-tool regressions | **4 passed**: deadline, parser version, extra depender and source exclusions. |
| cargo-deny active/full-inventory licenses | Passed; active graph has one expected unused CC0 exception warning; full inventory has no warnings. |
| cargo-deny active/full-inventory advisories | Passed, with the one approved advisory reported as notes; no other findings suppressed. |
| macOS arm64 bundle / plist / ad-hoc signature | Built and verified. Notices are hash-checked and staged by the shared script. No notarization claimed. |
| Legacy SQLite/imports/Undo/Redo | Existing complete suites passed; persisted schema/defaults and calculation bodies retained. |
| Source/ZIP integrity | Hidden files required; no target/dist/cache/private data/macOS metadata in candidates; archive contents/hashes checked. |

Token-based before/after comparison retained all **160 financial function bodies** across
model, register, store, bank, credit, income, splits and reports after identifier-only
normalization. All **250 test declarations** are retained: 249 host tests plus one explicit
non-macOS dialog test. The approved icon files and Cargo.lock match the phase baseline byte-for-byte.
The root license texts were subsequently finalized on maintainer instruction. No dependency upgrade, including eframe/egui,
was performed. Root model serialized type definitions/defaults remain in the same public module.

Detailed local evidence (logs, comparisons, before/after exact file list, source manifests,
synthetic GUI instance and captures) is under target/release-step2 and target/release-audit.
These directories are excluded from public candidates. See VALIDATION.md for live-check
scope, corrected intermediate failures and the actual final checks.

## Current license decision and remaining release checks

- MIT OR Apache-2.0 is active in Cargo.toml and the root license texts; copyright notice:
  2026 BudgetTinker contributors. LICENSE-STATUS.md and assets/LICENSE.md describe scope.
  No separate rights-confirmation step is required by this task.
- The license covers project-owned material and rights the project can grant; dependency
  code/fonts keep their notices. No independent legal clearance, exclusive AI-art rights,
  registered trademark, uniqueness guarantee or endorsement is claimed.
- Complete actual Windows/Linux builds and runtime verification before claiming full
  support. Only macOS/iOS Rust targets are installed; no Linux sysroot/host, Windows SDK/
  linker/runtime, Docker, Podman, Zig, MinGW or PowerShell is available locally. Source
  branches/workflows and target dependency graphs are inspected, not passed native builds.
- Review the temporary font-stack exception by 2027-01-28. No eframe/egui upgrade to 0.34
  is authorized in this task. A changed scope or elapsed deadline fails the audit wrapper.
- Before public release, enable or verify private vulnerability reporting
  and verify its button/notifications as documented in SECURITY.md. No remote setting is changed.

Creating the local ZIP does not authorize this agent to publish. Keep generated app bundles, archives,
SQLite files, tool caches and captured desktop material outside source commits.
