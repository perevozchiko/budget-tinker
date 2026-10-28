# Dependency license and advisory audit — 2026-10-28

## Scope and actual results

- Cargo.lock: **327 registry crates**, all with preserved declarations, collected notice
  records and verified SHA-256 hashes. [Per-crate decisions](third-party/REVIEW.md) and
  [machine inventory](third-party/inventory.json) cover every locked version.
- Active feature graph without a target filter: **291 registry crates**, plus BudgetTinker.
  An additional 36 optional/inactive crates remain in the lock inventory.
- Tool: **cargo-deny 0.20.2**, installed only under target/audit-tools, not added as an app dependency.
- Active licenses: exit **0**, 0 errors and one warning: the CC0 exception for inactive
  `hexf-parse` is unused in that graph. This is expected, not an ignored rejection.
- Full-inventory licenses: exit **0**, **0 errors, 0 warnings**, 328 notes (327 dependencies
  plus the licensed project package).
- Both active and full-inventory advisory checks: exit **0**, 0 errors/warnings, with the
  single owner-approved RUSTSEC-2026-0192 exception reported as notes. No other advisory
  is ignored. No other vulnerability or yanked-crate diagnostic was reported.
  This is not an assertion that no undisclosed defect exists.
- Database: [RustSec advisory-db](https://github.com/RustSec/advisory-db), revision
  `40f5be6fb12ed93fc591bf4ab23b7d793b3f2239`, upstream commit date `2026-10-28T09:50:03+01:00`;
  checked at `2026-10-28T14:24:45.720329+00:00`. Recheck before any real release.

The project package declares MIT OR Apache-2.0 and is checked alongside dependencies:
`[licenses.private] ignore = false`. `publish = false` prevents accidental crate publishing;
it does not make the license inactive. The maintainer instructed finalization without a
separate rights-confirmation step. Dependency grants and embedded component terms remain
separate; this configuration is not an independent ownership certification.

## Reproduce locally

Run from the source root with stable Rust, Python 3.9+ and network access for installation
and an initial RustSec fetch:

```sh
cargo install cargo-deny --version 0.20.2 --locked --root "$PWD/target/audit-tools" --target-dir "$PWD/target/audit-tools-build"
python3 scripts/audit-dependencies.py
python3 scripts/audit-release.py
```

`audit-dependencies.py --offline` reuses an already downloaded, sufficiently fresh database.
In manual GitHub Actions, `actions/checkout@v4` explicitly downloads the current official
`RustSec/advisory-db` `main` branch into cargo-deny 0.20.2's expected cache directory
(`target/release-audit/advisory-dbs/advisory-db-3157b0e258782691`) before invoking
`audit-dependencies.py --offline`. This avoids a Git transport/ref-fetch failure in
cargo-deny while retaining the normal advisory evaluation and its database-staleness guard.
The database is downloaded afresh for each CI run: a failed checkout blocks CI rather than
silently using an old snapshot. The directory name is tied to the pinned cargo-deny 0.20.2
cache naming; update it only after verifying a later cargo-deny version's actual behavior.
For local runs, the default command still fetches RustSec itself, and `--offline` requires
a previously fetched database. The script records tool version, database revision/date,
exact exit codes and diagnostic logs under target/release-audit.
The wrapper fails if the approved dependency versions/direct depender change or the UTC
review deadline has arrived. Every advisory except RUSTSEC-2026-0192 remains blocking;
`unmaintained = "all"`, `unsound = "all"`, and `yanked = "deny"` are explicit.

For all 327 licenses, the script makes a **separate exported Cargo metadata view** in which
all locked packages are audit roots. cargo-deny consumes it via `--metadata-path --workspace`.
The real manifest, lockfile, workspace and activated app features are not changed. This
avoids falsely claiming the default graph includes every optional crate in Cargo.lock.

cargo-deny retrieves its upstream database into a disposable Git checkout under target/.
That is tool data excluded from public candidates; no BudgetTinker repository is initialized,
no remote project repository is created and no application files are committed or uploaded.

## License policy and reviewed obligations

`deny.toml` accepts MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, BSL-1.0 and Unicode-3.0.
Alternative licenses are chosen per inventory record; this does not change upstream terms.
No GPL/LGPL branch or LLVM-exception branch is needed for the selected choices.
No dependency was upgraded, replaced or relicensed in this task.

| Selected terms | Crate records | Distribution action |
| --- | ---: | --- |
| MIT | 284 | Preserve upstream copyright/permission/disclaimer texts. |
| Apache-2.0 | 15 | Include license and any supplied NOTICE; do not claim upstream authors' rights or trademarks. No dependency source modified. |
| Apache-2.0 AND MIT | 1 | Preserve both sets of notices. |
| Unicode-3.0 | 18 | Keep copyright/permission/disclaimer notices for embedded Unicode data/software. |
| MIT AND Unicode-3.0 | 1 | Preserve both, including the data license. |
| Zlib | 3 | Preserve notice, origin and alteration conditions. No dependency source modified. |
| BSL-1.0 (Boost Software License, not Business Source) | 2 | Preserve source/permission notice; included conservatively for binaries too. |
| ISC | 1 | Preserve permission/copyright/disclaimer notice. |
| MIT AND OFL-1.1 AND Ubuntu-font-1.0 | 1 | Keep separate embedded-font grants and copyright metadata; do not relicense fonts under the project license. |
| CC0-1.0 | 1 | Preserve explicit package dedication and standard terms; no copyright/trademark ownership inferred. |

Two **crate-specific license-policy exceptions**, not waived obligations:

1. `epaint_default_fonts = 0.32.3`: OFL-1.1 and Ubuntu-font-1.0, required by unchanged
   embedded Noto Emoji/Ubuntu fonts. Fonts can be bundled, but remain under their own
   terms and may not be sold standalone. Their notices/copyrights accompany the bundle.
   No font names, bytes or font data were modified. Reserved-name/modified-font conditions
   would need reevaluation if future work changes fonts.
2. `hexf-parse = 0.2.1`: CC0-1.0, an optional locked parser dependency. Its release manifest
   explicitly declares CC0. It is absent from the active graph but covered by full-inventory
   checks. This is an explicit dedication, not an unidentified/unlicensed asset.

Manual review supplements Cargo's SPDX scan:

- Hack includes MIT, Bitstream Vera terms and public-domain DejaVu contributions; the
  full Hack notice is retained. It is not covered solely by the crate's top-level SPDX.
- Ubuntu Light: Canonical, UFL 1.0. Noto Emoji: Google, OFL 1.1. emoji-icon-font: John
  Slegers, MIT. All notices are included. Copyright/license name-table records extracted
  verbatim from the unchanged fonts are retained in `FONT-METADATA-NOTICES.txt`.
- SQLite **3.50.2**, bundled by libsqlite3-sys 0.35.0: upstream public-domain dedication
  retained beside the wrapper's MIT license. SQLCipher is disabled; its separate code is
  not shipped as application functionality.
- `r-efi` AUTHORS contains its actual licensing/copyright text; MIT is selected rather
  than optional LGPL. Both locked versions are covered.
- `dispatch` has explicit MIT/author metadata but no upstream standalone license file;
  standard MIT terms plus the original package declaration are assembled transparently,
  with no invented copyright year or purported upstream file.
- GNU winapi import-library companion crates have platform license notices from the
  parent upstream commit. They lack VCS records in their crate archives; no byte-for-byte
  source match is claimed. These packages are outside all three intended desktop targets
  (Windows uses MSVC). See the inventory provenance note; no foreign binary is vendored.

Sources for reviewed extra terms: [OFL-1.1](https://spdx.org/licenses/OFL-1.1.html),
[Ubuntu font license](https://spdx.org/licenses/Ubuntu-font-1.0.html),
[Unicode-3.0](https://spdx.org/licenses/Unicode-3.0.html),
[SQLite copyright statement](https://www.sqlite.org/copyright.html).

## Owner-approved, time-limited maintenance exception

[**RUSTSEC-2026-0192**](https://rustsec.org/advisories/RUSTSEC-2026-0192.html):
`ttf-parser 0.25.1` is classified **unmaintained**, with no patched version listed.
The active path is `eframe/egui → epaint → ab_glyph → owned_ttf_parser → ttf-parser`.
It occurs in the real application graph, not only inactive inventory.

[Upstream issue 217](https://github.com/harfbuzz/ttf-parser/issues/217) discusses a reported
font parsing complexity/DoS concern and maintenance status. The RustSec record is an
informational maintenance advisory; this audit does not establish an exploit in BudgetTinker.
Current source loads embedded egui fonts and fixed paths to locally installed macOS Arial
fonts. No arbitrary font import UI was found. That narrows the observed input path but does
not erase maintenance risk or justify silently suppressing the advisory.

The owner explicitly approved an exception on 2026-10-28, without upgrading eframe/egui.
`deny.toml` ignores only this advisory ID, with the reason, exact chain, font paths and
**review deadline 2027-01-28**. All affected versions remain pinned: eframe/egui/epaint
0.32.3 → ab_glyph 0.2.32 → owned_ttf_parser 0.25.1 → ttf-parser 0.25.1.

Verified font inputs:

- epaint_default_fonts 0.32.3 embeds Hack Regular, Ubuntu Light, Noto Emoji and emoji-icon-font.
- src/ui/application.rs optionally reads these fixed installed macOS paths, never a user-supplied path:
  `/System/Library/Fonts/Supplemental/Arial.ttf`, `Arial Unicode.ttf`, `Arial Bold.ttf`,
  and `Arial Bold Italic.ttf` under that same directory.
- Missing named families fall back to the existing embedded egui fonts. The timeframe
  event test now uses those bundled fonts instead of unconditionally loading system Arial.
- No arbitrary font import UI was found. These observations are a rationale for the
  maintenance exception, not an exploitability or font-safety guarantee.

**Enforcement:** the installed/pinned cargo-deny 0.20.2 supports ID/reason ignores, but not
the newer documented per-ignore `expiry`/`allow` fields. Unsupported keys are not silently
added. `scripts/audit-dependencies.py` verifies the exact six package versions, requires
owned_ttf_parser 0.25.1 to remain ttf-parser's only direct depender, and fails on/after
**2027-01-28 UTC** before invoking cargo-deny. `scripts/test-release-tools.py` exercises
the deadline, changed parser version and additional depender rejection. Directly invoking
cargo-deny alone does not enforce the calendar deadline; CI and the documented release
command must use the wrapper. Review/renewal requires an explicit new owner decision.

No dependency, lockfile version, font binary or application style is upgraded. The original
unsuppressed failure is historical evidence in VALIDATION.md; it is no longer an unresolved
release gate within the approved scope and review window. Foreign-platform validation remains a separate release-readiness item.

## Native dependencies and packaging

macOS executable's `otool -L` output references only Apple system frameworks/libraries,
including AppKit, CoreGraphics, OpenGL, Foundation, CoreServices, Carbon, libobjc,
libiconv and libSystem. They are linked from the OS, not copied/relicensed. Runtime Arial
and Arial Unicode/Bold/Italic are read from the user's OS font installation; no Microsoft
or Apple font binary is redistributed. Swift PDF helper uses system frameworks. `open`,
`osascript`, `curl`, Swift, sips, iconutil and codesign are system/build tools, not assets
claimed under BudgetTinker's license. Linux/Windows native linking is unverified locally.

`scripts/stage-notices.py` validates all collected notice hashes, then stages root license texts,
license status, NOTICE, ATTRIBUTION, branding license, icon provenance and the complete
third-party inventory. macOS bundle and Linux/Windows manual archive steps all call it.
No package is published or uploaded by these local checks.
