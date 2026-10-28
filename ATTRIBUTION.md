# Attribution

## Project assets

| Paths | Author / source | License and notices |
| --- | --- | --- |
| `assets/icon-artwork.png`, `assets/app-icon.png`, `assets/BudgetTinker.ico` | User-supplied BudgetTinker release kit; background edited with built-in imagegen on 2026-10-28 at the owner’s request to match the interface’s teal texture. Processed into platform formats. | [Artwork license](assets/LICENSE.md): MIT OR Apache-2.0 to the extent the project can grant rights; no exclusive copyright, trademark registration or uniqueness claim. The artwork PNG is authoritative. Packaging scales it to 824×824 on a transparent 1024×1024 canvas for Dock sizing and clips the rounded-square boundary. The same padded PNG is embedded by eframe and converted to .icns. |
| `locales/en.json`, `src/theme/icons.rs` | Existing local project, English labels and code-drawn UI symbols | MIT OR Apache-2.0. No old app logo copied. |
| `tests/fixtures/bank/**`, `tests/fixtures/legacy/*.yfull` | Synthetic local regression scenarios: fabricated accounts, payees, dates and amounts | MIT OR Apache-2.0 (see LICENSE-STATUS.md). File extensions/field names describe interoperability, not bundled proprietary application code. |
| `tests/fixtures/bank/matching/expected.json` | Synthetic financial projections and observed match outcomes from local regression research; formerly stored among comparison docs | MIT OR Apache-2.0 (see LICENSE-STATUS.md). Contains test facts only; no executable, resource, UI screenshot or commercial application source. |
| `examples/*.rs` | Local synthetic demo generators | MIT OR Apache-2.0 (see LICENSE-STATUS.md). |
| `docs/screenshots/budgettinker.png` | Real BudgetTinker macOS screenshot with generated demo data | Project screenshot, MIT OR Apache-2.0; includes embedded fonts under their own licenses. |

No commercial reference app binaries, installers, original-app screenshots, comparison
reports, downloaded artwork or external standalone font files are included.

## Embedded dependency assets

`eframe` enables `epaint_default_fonts` 0.32.3. Cargo embeds these upstream fonts:

| Font | Author/source | Exact terms |
| --- | --- | --- |
| Hack Regular | Source Foundry; includes Bitstream Vera and DejaVu contributions | MIT + Bitstream Vera terms and public-domain contributions; `docs/third-party/epaint_default_fonts-0.32.3/Hack-Regular.txt` |
| Ubuntu Light | Canonical / Ubuntu font project | Ubuntu Font Licence 1.0; same directory, `UFL.txt` |
| Noto Emoji | Google / Noto project | SIL Open Font License 1.1; same directory, `OFL.txt` |
| emoji-icon-font | John Slegers | MIT; same directory, `emoji-icon-font-mit-license.txt` |

Source: [epaint_default_fonts 0.32.3](https://crates.io/crates/epaint_default_fonts/0.32.3),
whose crate source and bundled license texts were inspected locally.

## Code dependencies

[Dependency inventory](docs/third-party/inventory.json) records all locked resolved
packages, versions, declared licenses, authors, upstream repositories and copied notices.
Builds include bundled SQLite through libsqlite3-sys; its upstream component terms remain
separate from BudgetTinker's code. See [dependency notices](docs/third-party/README.md)
for provenance and selected-license records. All 327 crates now have collected notice records
and SHA-256 checks. See [the dependency audit](docs/DEPENDENCY-AUDIT.md) for font exceptions,
manual embedded-component review and the owner-approved, deadline-limited RustSec maintenance exception.
Source paths below refer to the source tree; distribution Notices/ uses a sibling
third-party/ directory. This inventory is not a project-ownership certificate.

Presentation was informed by [PhotoCraft](https://github.com/storytold/photocraft)'s public
README/license/attribution/security structure. No PhotoCraft code, artwork or prose was copied.
