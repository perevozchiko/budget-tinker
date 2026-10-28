# Project and asset provenance audit — 2026-10-28

## Evidence available in this copy

During migration, this then-unversioned sibling was copied from the local Rust project at source commit 2e4fbb9
in the earlier migration. Git history was intentionally excluded. This follow-up operates
only in BudgetTinker and does not inspect or modify the original project.

The copy contains Rust application modules and integration-test source files, synthetic
bank/import fixtures, example generators, English localization, native build scripts and
the project artwork/screenshot. A fresh baseline of source/fixtures/localization/artwork
hashes is retained only under target/release-audit/baseline.json for this phase.

Source/header/URL scans found no embedded third-party copyright or source-license blocks
in the project-owned modules, generators, translations or Swift helpers. Their claimed
project origin is inherited from the local tree; absence of a notice is not proof of title.
The copy has no contributor history/assignment records, employer release, or signed owner
approval. The earlier migration's author metadata is insufficient to establish rights.
The license decision is recorded in LICENSE-STATUS.md; this provenance inventory is not an independent ownership certification.

## Asset classification

- `src/icons.rs` draws UI symbols in code; no copied proprietary bitmap icon set found.
- `assets/icon-artwork.png`: supplied project emblem, AI-assisted background edit at owner
  request. `assets/app-icon.png` and `.ico` are normalized/padded derivatives; .icns is
  generated during bundling. The source, prompt and workflow are in ICON-EDIT.md.
- `docs/screenshots/budgettinker.png`: real application window with synthetic demo values;
  no generated/fake application UI and no commercial reference screenshot is included.
- `locales/en.json`: existing local English labels; no external localization corpus cited.
- `tests/fixtures/bank/**` and `tests/fixtures/legacy/*.yfull`: small synthetic parser and
  matching scenarios with fabricated records. File/field names retain interoperability.
  The matching expected JSON is financial projections and observations, not app source.
- `docs/third-party/**`: dependency legal notices and declared authors. Those grants remain
  separate. Embedded egui fonts and bundled SQLite are explicitly accounted for.

No proprietary app executable, installer, original-app screenshot, downloaded sample
artwork, account database, bank credential or user budget is part of public candidates.
The source/secret scan is evidence about the current files, not a claim of an independently
certified clean-room development process or a guarantee that no third-party right exists.

## License decision

The maintainer requested active MIT OR Apache-2.0 licensing without a separate rights
confirmation step. The copyright notice is 2026 BudgetTinker contributors, as in the supplied
release-kit template. Existing third-party grants remain separate. AI assistance does not
establish exclusive copyright or guaranteed uniqueness; no trademark clearance is claimed.
See ../LICENSE-STATUS.md and ../assets/LICENSE.md.

## Structural follow-up

Project code is now organized under model/, imports/legacy_budget/, ui/, widgets/ and theme/.
Large integration suites retain their root harness files and topic submodules. No dependency
version or approved image was changed during the structural refactor. Root licenses
were subsequently finalized on maintainer instruction. Financial function bodies were
compared token-by-token against the pre-refactor source after identifier-only normalization;
existing test declarations were counted before and after. See REFACTORING.md and VALIDATION.md.
Branding was removed from product-owned module names, captions, examples and public prose.
Wire-format fields and persisted debt identifiers remain solely to read existing documents.
