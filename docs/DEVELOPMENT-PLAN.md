# Development plan

## Current repository status

The private repository `perevozchiko/budget-tinker` has an initial source commit on
`master`. It is not publicly released and has no published GitHub Releases. macOS ARM64
is verified; Windows and Linux builds and runtime support remain unverified.

## Public preparation — 2026-10-28 (historical)

- [x] Create an unversioned sibling copy without old history, builds or private reference material.
- [x] Rename product, binary, bundle, support paths, environment variable and icon.
- [x] Use `.budgettinker` for native budget defaults and save dialogs; retain SQLite format and legacy opening.
- [x] Remove original-application documentation and screenshots from the public copy.
- [x] Verify 246 tests, strict Clippy, examples, macOS bundle and live synthetic GUI.
- [x] Adjust Dock icon size and match the sidebar background at the owner’s request.
- [x] Finalize root MIT OR Apache-2.0 licenses, Cargo declaration and project artwork terms.
- [x] Audit all 327 dependency licenses, embedded fonts/native components and packaged notices.
- [x] Add pinned cargo-deny, shared notice staging and source-candidate verification.
- [x] Activate MIT OR Apache-2.0 and project asset terms on maintainer instruction, without a separate confirmation step.
- [x] Owner-approved RUSTSEC-2026-0192 exception with fixed font-stack scope and UTC deadline 2027-01-28; all other advisories still block.
- [x] Keep timeframe event coverage portable; add named-font fallback without changing installed macOS fonts.
- [x] Audit native operations, report unsupported Windows/Linux dialogs/printing, and fix OS command paths.
- [x] Include hidden CI/ignore files in verified source ZIPs; exclude builds, data and macOS metadata.
- [x] Translate uncovered product validation/error boundaries while preserving stored/user text.
- [ ] Review the temporary font-stack exception by 2027-01-28; no egui/eframe upgrade authorized yet.
- [ ] Review the release audit and verify Windows/Linux builds using manual CI.

- [x] Remove contiguous legacy brand names from source text while retaining legacy format compatibility.

## Source organization follow-up

- [x] Neutral importer files/modules, product captions and public documentation.
- [x] Split application/model/report/editor/chrome/control monoliths by responsibility.
- [x] Move shared widgets/theme and topic-based unit/integration tests into directories.
- [x] Retain all test declarations and check financial bodies after identifier normalization.

## English source migration

- [x] Convert exchange-rate error messages to English at their source.
- [ ] Replace remaining Russian source strings and retire the translation catalogue without altering stored user data.

## Next behavioral work (separate from branding)

- Explicit Uncategorized state for imported bank inflows, with old-document compatibility.
- Performance checks at 3,000–30,000 transactions and large payee/category lists.
- Cross-platform native file dialogs and platform helper support.

Public preparation preserves the existing interface, calculations and serialized budget schema.
See PUBLIC-RELEASE-AUDIT.md for verification and release blockers.

## Post-migration repository maintenance

- [x] Permit Git checkouts in source audits while excluding Git metadata from review ZIPs.
- [x] Add manual CI platform selection and update private repository status documentation.
- [ ] Run and test Windows/Linux platform builds before claiming platform support.
