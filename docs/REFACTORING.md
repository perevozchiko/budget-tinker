# Source organization and compatibility

The refactor groups code by responsibility instead of splitting it into numbered fragments.
No dependency upgrade, interface redesign or financial algorithm rewrite is included.

| Component | Earlier size | Current entry | Entry size |
| --- | ---: | --- | ---: |
| Application | 3,620 lines | src/ui/mod.rs | 313 |
| Report UI and tests | 2,810 | src/ui/report_view/mod.rs | 83 |
| Transaction editor and tests | 1,951 | src/widgets/transaction_editor/mod.rs | 125 |
| Model | 1,516 | src/model/mod.rs | 383 |
| Scheduled panel and tests | 1,224 | src/ui/scheduled_panel/mod.rs | 122 |
| Application chrome | 1,149 | src/ui/chrome/mod.rs | 170 |
| Legacy importer | 1,002 | src/imports/legacy_budget/mod.rs | 105 |
| Shared classic controls | 984 | src/theme/classic/mod.rs | 53 |

- ui/: application state/Undo, sidebar, accounts, budget navigation/summary/table, and
  dialog preparation/application/rendering. No new egui layout scopes were introduced.
- ui/report_view/: filter controls/selectors, preferences, periods, spending, trends,
  net worth, matrix and print/export document construction; tests grouped by topic.
- widgets/transaction_editor/: draft validation/building, rendering and date/payee/category/
  split/schedule/entry tests. Other shared inputs and register widgets live under widgets/.
- theme/: colors, symbols, buttons, dates, badges, navigation, menus and sidebar painting.
  Existing colors, geometry, typography and approved artwork are retained.
- model/: unchanged public serialized types plus initialization, accounts, valuation,
  transactions, categories, allocations, totals, reconciliation and schedules.
- imports/legacy_budget/: source validation, decimal conversion, schema decoding,
  transactions, bank-original matching and transfer pairing.
- report_print/: portable layout and the existing native helper boundary.
- tests/accounting/, tests/reports/, tests/legacy_import/: topic submodules, with shared
  fixtures in the existing root integration harnesses. Every test declaration is retained.

The largest Rust source file is now 732 lines, including when integration-test files are
counted. The public model type location and serialized field names/defaults remain stable.
Internal debt helper names and product-owned import link namespaces are neutral. Old opaque
link IDs are still read as ordinary IDs; existing documents are not migrated or rewritten.

Product-owned file/module names and visible labels no longer use the source application's
brand. Captions use BudgetTinker, Legacy Budget and Starting Debt. Some wire-format keys,
persisted debt group/prefix values, matching fixtures and private-file exclusion patterns
necessarily retain historical identifiers for compatibility. Those are not product labels;
they are not obfuscated or silently renamed. The UI translates the historic debt group to
Starting Debt. The import still reads original folder metadata/full snapshots regardless of
the enclosing folder name. Synthetic fixture folders use a neutral suffix.

Before/after verification tokenizes financial function bodies, ignores comments/whitespace,
and normalizes only renamed identifiers/format-variable names. All 160 bodies in model,
register, store, bank, credit, income, splits and reports match; no numeric literal or string
value used by financial calculations is waived. All 250 Rust test declarations are retained
(249 execute on macOS; one unsupported-dialog test is for non-macOS). Full tests, strict
Clippy and examples are the completion gate. Local detailed comparison evidence remains
under target/release-step2 and is excluded from the source ZIP.
