# Multi-currency implementation notes — 2026-10-29

[Specification v2](MULTI-CURRENCY-SPECIFICATION-v2.md) is the owner-approved
product contract, subject to [owner amendments](MULTI-CURRENCY-AMENDMENTS.md).
The source specification is retained verbatim. These notes
record engineering choices, actual repository findings and incremental delivery;
they do not relax the financial invariants or claim the whole feature works.

## Phase 0 audit

- Actual branch: `feature/multi-currency`. Initial worktree already contained
  discovery edits to DEVELOPMENT-PLAN/VALIDATION and an untracked questions file;
  all retained. No Git mutation requested or performed.
- Currency/format helpers are in `src/model/mod.rs`, not a separate money module.
  Current document version is 1. Account native amount and base amount already
  exist, but `transaction_value`, `split_base_amounts`, `prepare_transaction`
  and `store::validate` depend on shared monthly snapshots. Replacing monthly
  rates now would invalidate saved transactions. Keep replacement disabled until
  operation locks and migration are implemented together.
- `accounts.rs` creates nonzero opening balances as transactions; zero opening
  creates no historical record. Currency lock must include posted and scheduled
  references, import-match originals and future saved balances, not just current
  balance. Audit `fresh_start` too. Account creation UI forces budget base.
- `chrome/screens.rs` can change the base of a transaction-free existing budget.
  It must become creation-only. This first numeric increment does not fix it.
- `transactions.rs::transfer` accepts actual unequal cross-currency amounts;
  the dialog disallows unlike currencies. Existing validation assumes same-day
  two-ended transfers. Transit needs an explicit state and stable associations,
  not a temporarily invalid half-pair. Fees must be separate linked expenses.
- `totals.rs` repeatedly scans account/history/category/month combinations.
  `month_summary().income` is a valuation residual, not earned income. Preserve
  that baseline until explicit flow classification and both bridges land.
- `reports.rs` has five existing reports and split-aware flows. Net Worth loops
  over all months and accounts; retain filters, grouping, CSV and print behavior.
- `bank.rs` supplies checked source/budget-bound previews for CSV/QIF/OFX/QFX;
  `store.rs` parses CSV; `imports/legacy_budget` decodes exact JSON numbers and
  matches transfers. Current foreign rows require monthly FX to finish parsing;
  there is no unresolved row state. Phase 2 must stage Pending FX rows before
  final posting, preserving source consistency and matching rules.
- `model/schedules.rs`, `register.rs` and `ui/scheduled_panel` post connected
  schedules atomically but without the required foreign FX confirmation editor.
  Templates must remain unposted until actual amount and independent FX approval.
- `store.rs` validates before saving a whole JSON document and up to 100 SQLite
  revisions in a transaction, WAL + FULL sync. `ui/application.rs` retains up to
  50 whole-Budget Undo states, saves before swapping state, and protects reconciled
  rows. JSON restore currently travels through normal dialog/commit behavior;
  replace with preflight, safe backup and stack reset in the dedicated flow.
- There is **no custom account-group model** in Budget, Account or Preferences.
  `ui/sidebar.rs` has three fixed sections (budget/off-budget/closed), whose
  collapse preferences live beside the budget. Account order is Vec order.
  Do not mistake category groups for account groups. The specified 50 custom
  account-group workload is not representable yet; model and backup design must
  extend the actual sidebar semantics without a competing group store.
- README describes the repository as private; the supplied specification calls
  that inconsistent with public visibility. Visibility was not queried or changed.
  Resolve the wording independently of implementation and publication.
- Frankfurter currently runs curl only on request, deserializes a float, and
  formats to eight decimal places. Phase 1 must parse the numeric response exactly
  and separate suggestion from acceptance. No lookup was used during this work.

## Financial model and migration design (Phase 1, in progress)

Keep `Transaction.amount` and `base_amount` as the single authoritative native
and base postings. Add operation FX metadata with an exact fixed quote or ratio,
date/source, optional merchant money and posted date. `MonthlyDefault` copies the
monthly quote; any snapshot ID is provenance only. Manual exact-base overrides
must record an exact implied ratio/override rather than forcing equality to a
rounded displayed rate. Base-account amounts remain equal to base postings.
Introduce stable split IDs and persisted split base equivalents together with
their parent conversion; preserve deterministic last-nonzero remainder semantics
for migration, including zero-net mixed-sign splits. Do not invent zero locks.

Use a new schema version only once frozen FX, validators and conversion are wired
end to end. Preflight a synthetic/current-v1 candidate without changing the file.
Copy every existing base amount and the exact referenced quote into operation
locks; derive existing split base values using the old algorithm. Preserve all
IDs, category activity, reconcile/import state, schedules, payee rules, notes,
allocations and native balances. Validate missing/duplicate/broken references
before replacement; malformed input leaves the prior document intact.

Before the first upgrade write, create a recoverable SQLite backup through the
SQLite backup API (enable the rusqlite feature deliberately) or verified SQLite
`VACUUM INTO`, not file copying across live WAL. Confirm upgrade in native UI,
then write the validated document atomically and reset incompatible in-memory
history. Keep the original backup and test reopen, interrupted/failed writes,
rollback and failure to create the backup. Never test on the owner's budget.
Revisions need an explicit compatible conversion/read policy, not silent deletion
or loading v1 values as though they already contained v2 locks.

After the schema change, open-month and historical commands operate on monthly
valuation records only. Finalization is explicit after month end; historical
corrections retain audit metadata and before/after adjacent-period bridges.
Independent transaction locks and split amounts must compare byte/semantically
equal before/after rate edits and Undo. Only then enable rate replacement in UI.

### Detached preflight and SQLite backup increment

`model/operation_fx.rs` now holds copied operation quotes and independently
validated native/base amounts, with MonthlyDefault provenance and stable
transaction-local split IDs. `store/upgrade.rs::FrozenBudget::preflight` validates
the existing v1 document first, retains a complete clone and separately prepares
locks for posted rows and both hidden match originals. Scheduled templates remain
unchanged and receive no invented posting locks. Split amounts copy the existing
last-nonzero remainder algorithm, including mixed-sign zero-net splits; all-zero
foreign rows have no fabricated quote. Conversion validation never looks up a
monthly snapshot from a detached lock. These are preparation types, **not the
active v2 storage format**. Exact-base and manually rebalanced split overrides
still need the subsequent format/command integration.

`Store::preflight_upgrade` reads the current JSON and every retained revision in
one SQLite read transaction, validating each independently. A malformed revision
blocks preparation; nothing is deleted. `backup_for_upgrade` binds to those exact
source JSON strings, revision IDs and timestamps, rejects stale preparation and
uses rusqlite's deliberately enabled `backup` feature. The online backup includes
committed WAL pages and additional tables. A unique backup directory prevents
overwriting earlier recovery files; integrity and exact document/history equality
are checked before success. The file and, on Unix, containing directories are
synced. On verification/durability failure, keep the recovery candidate and name
its path in the error. No migration write or automatic startup invocation exists
yet. This backup is preparatory infrastructure, not a claim that atomic schema
replacement/rollback or restore/Undo reset is complete.

An extreme mixed-sign split exposed another overflow boundary: `split_base_amounts`
now carries the remainder in i128 and range-checks its final i64 posting. Current
document validation checks split conversion too, rejecting unrepresentable final
parts before save/load rather than accepting data that later fails during totals.
Valid existing rounding results are unchanged.

## Explicit schema v2 activation increment

Budget Properties now offers **Upgrade currency data…** for v1 files. A native-app
confirmation reports validated operation/revision counts, explains recovery and
incompatible old readers, and requires **Back Up and Upgrade**. Cancel/Escape
perform no document/revision write and create no backup. Startup and new-budget
creation continue using v1 until the owner explicitly upgrades that file. This
increment does not enable monthly-rate replacement, network lookup or the full
manual FX editor.

V2 stores `Transaction.operation_fx` on every posted row and both hidden match
originals. Scheduled templates have no posting locks. `Split.id` persists stable
transaction-local identities; the editor carries them through part deletion and
metadata edits. V1 fields default to absent/empty without changing old JSON.
V2 readers validate copied account/base currencies, native/base amounts, split
identities and amounts against their parent before using the lock. Snapshot IDs
are provenance only: clearing monthly snapshots or changing their rates cannot
reprice a v2 posting or category activity. New postings freeze their selected
monthly default; editing an existing posting uses its copied quote. Exact-base
postings record an explicit override (the native/base pair is the exact ratio),
and explicit split base entries must total the parent, with zero native parts
remaining zero. `set_operation_base` applies and validates on a disposable copy;
reconciled/transfer rows are excluded. Monetary edits to an explicit override
require new base approval; metadata edits preserve it. Editor controls for that
approval are still outstanding.

`Store::upgrade` derives candidates from private source JSON rather than mutable
preview presentation fields. It verifies the online recovery copy before beginning
an IMMEDIATE transaction, checks source equality again under the write lock, and
updates the current document and every retained revision together. IDs/timestamps
are retained. Exact written-document/history comparison and normal validation run
before commit, detecting ignored writes and trigger interference. Failed writes
roll back together; the error identifies the retained recovery file. Ordinary
save cannot change schema or change a v2 budget's base currency. The existing v1
whole-budget legacy-import/Undo path stays compatible; replacing a v2 file with a
v1 JSON/legacy document is rejected pending the dedicated Phase 5 replacement
flow, rather than silently dropping independent locks.

Only a successful commit swaps App state and clears Undo/Redo, selection, drafts
and reconciliation. A success dialog identifies the recovery copy and how to open
it. Failure/cancellation retain in-memory history. All retained versions are v2
and can subsequently be opened through the existing version reader. The base
selector in properties is creation-only, including transaction-free budgets;
the format sample now uses the approved space grouping and ISO code.

## Aggregation and invalidation design

Before extending reports, build an ephemeral index by transaction ID, account ID,
monthly account/native flow, monthly classified frozen base flow and category
activity. Compute account prefix balances and category carry-forward once, with
the existing deferred-income and overspending rules. Use checked wide totals;
round converted account balances only at the defined stock valuation boundary.
Keep the all-account and eligible budget populations distinct. Revaluation is
the stock change minus frozen flows; named flow components partition the flows.
Include transit exactly once, with its own currency/exposure and lifecycle.

Cache ownership must be explicit: a borrowed immutable Budget calculation context
can build indices without any hidden stale interior cache. A UI-owned derived
view cache must carry a monotonically increasing state generation, bumped only
after successful commit/Undo/Redo/restore/switch/upgrade. Budgets are publicly
mutable, so never key on object address, row counts, date or a rate value alone.
Any future partial invalidation must cover both endpoints/fees/refund origins,
old and new dates/categories, split changes, reconciliation, schedules, group
membership and account on-budget/type edits. Initially rebuild the index after
each accepted command; no serialization or Budget clone on text-field keystrokes.

Keep valuation and frozen-flow derivations separate so rate edits cannot mutate
activity. Tests must compare indexed results to the current scan implementation
on synthetic single-currency/legacy-compatible cases, then to acceptance bridges
for new semantics. Exercise invalidation after failed commands, historical
correction, Undo/Redo and whole-document replacement. No cache is implemented in
this first numeric increment; measurements below establish its urgency.

## Repeatable baseline and provisional responsiveness targets

Run `cargo run --release --locked --example multi_currency_baseline` on macOS arm64.
The example creates a unique temporary directory, never consults startup preferences
or a live database, validates every synthetic posting and removes its own files.
It uses 100 accounts, 240 months (2006–2025), 100,000 posted rows, positive/negative
flows and all supported currencies. JSON round-trip asserts full semantic equality.
It does not simulate exchanges, refund links or 50 unsupported custom groups yet.
It measures actual store operations including validation. Undo/Redo timings are
model clone + store surrogates; historical correction is a clone/valuation preview
surrogate because the current model rejects actual corrections. No UI or migration
latency is inferred from these measurements.

Single-run observations, milliseconds (not a stable statistical benchmark):

| Operation | Original 10 currencies | First increment, 11 currencies |
| --- | ---: | ---: |
| Validate | 249.118 | 262.083 |
| SQLite save | 361.891 | 404.696 |
| SQLite load | 291.090 | 319.845 |
| Month summary | 4223.601 | 4081.602 |
| Register filter | 1.055 | 0.494 |
| Month Net Worth | 43.215 | 29.942 |
| Spending reports 0–3 | 47.864–49.082 | 47.554–50.024 |
| 20-year Net Worth report | 6479.905 | 4782.229 |
| JSON export / validated import | 157.274 / 298.920 | 44.866 / 324.454 |
| Undo snapshot clone | 9.957 | 9.612 |
| Save with revision | 383.945 | 453.133 |
| Undo save / Redo save | 367.806 / 388.053 | 444.557 / 472.477 |
| Correction preview surrogate | 61.299 | 64.627 |

Logs are local under `target/multi-currency-phase-0/`. Currency distributions
differ between runs; timings include filesystem/cache variation and are **not**
evidence of a speedup. Current month summary and long Net Worth reporting are
unresponsive at this workload. Provisional engineering targets for Phase 5 on
this machine: p95 warm month/filter/valuation/report updates <=100 ms; accepted
command including save <=250 ms; load and validated JSON replacement <=2 seconds.
These are proposed targets, not owner-approved guarantees. Before claiming them,
run at least five measured repetitions after warmup, report hardware/build/data
shape and p50/p95, separate cold IO, and add linked/refund/transit/group scenarios.
Whole-document revision retention is provisional until those measurements pass;
evaluate indexed/granular persistence if command latency remains above target.

## First Phase 1 increment

TRY joins the ten existing codes. Precision metadata is centralized (JPY 0,
KWD 3, others 2). Per the owner amendment, user input accepts strict space grouping and decimal
points (`1 500.00`); all commas are rejected in product entry. Ungrouped
`1500.00` remains accepted. This supersedes the original MC-006 separator rule. Keep the established
100,000,000,000,000 minor-unit input bound pending a separate bounds review.
Generic fixed precision rejects unsupported scales instead of overflowing.
Conversions check i128 intermediates and use symmetric half-away-from-zero
rounding; extreme inputs return errors. Display uses space grouping, decimal
points and ISO codes in both UI languages, sharing the model formatter; `plain` remains ungrouped
for machine exports. CSV/QIF/OFX/QFX retain comma-grouped statement compatibility
through a dedicated strict import adapter, also accepting space-grouped statements. Neither path
strips malformed grouping into a different monetary value. Existing exact
legacy JSON decoding is unchanged.

This increment leaves schema version 1 intact. It does not yet implement operation
locks, immutable base, account currency UI, migration, mutable monthly rates or
any later phase. AC-11 and MC-002/005/006 receive focused numerical coverage;
an all-currency current-document round-trip is a foundation test, not AC-12/20.


## Phase 1 operation FX editor increment — 2026-10-29

The v2 ordinary foreign-account editor now exposes an opt-in `Enter operation FX`
section. The canonical direction is labeled `1 <account currency> = <rate> <base>`.
Rate entry uses eight-decimal fixed point; quote date is explicit and cannot be
later than today. The current monthly quote, or latest earlier quote, may prefill
these fields; earlier suggestions are labeled Stale. Existing rows show their
frozen base amount and load their copied operation quote when the section opens.
Save explicitly confirms the selected operation quote. Editing a suggested quote
or its date changes provenance to Manual. The command never writes monthly rates.
Reciprocal market notation/quote inversion remains outstanding for AC-21.

Exact signed base input can accompany a quote or stand alone. With no quote, the
editor derives an eight-decimal reference from integer native/base amounts using
i128 and symmetric rounding; the entered base minor amount remains authoritative.
An exact split requires every signed base part and exact equality to the parent.
Split IDs remain stable; deleting a part removes its matching base draft field.
Unchanged operations retain their old locks during ordinary metadata edits.
Account/date changes require reviewing the operation FX fields again.

`prepare_operation_fx` constructs and validates a detached posting, then uses the
ordinary preparation path for date/account/category/split/reconciliation checks.
The ordinary command preserves an already valid v2 lock during the second
editor-to-command validation pass. This prevents confirmed manual FX from being
replaced by an old lock or a monthly quote. The UI commits once through existing
SQLite and Undo/Redo handling; no whole-budget clone occurs on field keystrokes.
Legacy v1 entry, transfer and schedule editors retain their previous paths.

This increment does not implement merchant amounts, posted dates, linked refunds,
manual exchange legs, explicit scheduled FX confirmation, quote lookup acceptance,
account currency controls, monthly replacement/finalization or historical
correction. Missing monthly valuation still makes budget totals unavailable;
operation posting with its own FX does not fill or finalize monthly valuation.


## Monthly lifecycle command foundation

`model/monthly_fx.rs` adds serde-defaulted lifecycle metadata beside existing
snapshots. Upgraded files retain their quotes without inventing finalization.
Only the current month has the direct editable command; a past month uses an
opaque correction proposal, including when its first quote is missing. Explicit
finalization requires a past month and every nonzero account exposure's quote.
Zero balances need no quote. Historical quote replacement retains snapshot IDs
as provenance, while every posting and hidden matched original remains intact.
Audit records preserve the prior/new rate, date/source, reason and timestamp;
legacy empty source text is preserved in the prior quote, while newly entered
quotes require a source. Existing low-level `add_snapshot` remains an append-only
construction/import primitive; new interactive flows must use lifecycle commands.

The correction proposal includes authoritative before/after Net Worth, budget
stock value, Available and residual revaluation for the selected and following
months. Missing comparison quotes produce unavailable deltas, never invented
zeros or estimated authoritative totals. Residuals partition actual frozen
postings by all-account/on-budget population; this does not replace the existing
Income display or complete the Phase 4 flow classification/debt rules. Net Worth
accumulates in i128 and rejects an unrepresentable i64 total. Approval rejects a
changed source document before applying the private candidate. Existing App
commit/Undo/Redo handles correction/finalization as individual atomic changes;
failed persistence retains document, audit and both stacks.

An account-only preview can use the latest earlier quote with its month/date and
an estimated flag, or return unavailable; it never inserts that quote or changes
authoritative totals. No monthly native controls, report badges, network lookup,
transit model, or performance index is introduced by this increment. GUI wiring
and live acceptance remain subsequent work.
