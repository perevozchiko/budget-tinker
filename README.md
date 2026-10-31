<div align="center">

<a href="https://github.com/perevozchiko/budget-tinker">
  <img src="assets/app-icon.png" alt="BudgetTinker application icon" width="84">
</a>

<h1>BudgetTinker</h1>

<p><strong>Plan your money. Know where it goes.</strong></p>

<p>A local-first envelope budgeting app for your desktop.<br>
Give money a purpose, follow your spending and stay in control — without a subscription or cloud account.</p>

<p>
  <a href="#screenshot">Screenshot</a> ·
  <a href="#features">Features</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#downloads">Downloads</a> ·
  <a href="#documentation">Documentation</a>
</p>

<a href="https://github.com/perevozchiko/budget-tinker/actions/workflows/rust.yml">
  <img src="https://github.com/perevozchiko/budget-tinker/actions/workflows/rust.yml/badge.svg?branch=master" alt="Rust checks and desktop builds">
</a>
<a href="LICENSE-MIT">
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="MIT OR Apache-2.0">
</a>
<a href="https://github.com/perevozchiko/budget-tinker/releases">
  <img src="https://img.shields.io/github/v/release/perevozchiko/budget-tinker?include_prereleases&label=release" alt="Latest GitHub release">
</a>

<p><strong>Rust + egui + SQLite</strong> · <strong>Native desktop</strong> · <strong>Local data</strong> · <strong>macOS / Windows / Linux builds</strong></p>

</div>

BudgetTinker is independent, open-source personal budgeting software. It helps you allocate money to categories, record transactions and understand where your money goes. Your budget lives in a local SQLite file you control; there is no sign-in, web server or mandatory cloud service.

> [!IMPORTANT]
> **Early alpha.** Keep independent backups of important budgets. macOS Apple Silicon is the locally verified platform; Windows and Linux builds are available but have not received equivalent end-to-end desktop testing. See [platform support](docs/PLATFORMS.md) before relying on those builds.

## Screenshot

<div align="center">

<a href="https://raw.githubusercontent.com/perevozchiko/budget-tinker/master/docs/screenshots/budgettinker.png">
  <img src="docs/screenshots/budgettinker.png" alt="Real BudgetTinker desktop application showing a budget populated with synthetic demonstration data" width="940">
</a>

<sub>Real macOS application capture with fictional demo data — not a render or mockup.<br>
Classic light interface · click to open the original full-size screenshot.</sub>

</div>

## Features

| Plan | Track | Understand |
| :--- | :--- | :--- |
| **Envelope budgeting** — assign available money to categories and plan monthly allocations. | **Account registers** — keep budget and off-budget accounts with transfers, splits and reconciliation. | **Clear reports** — inspect spending, income vs. expenses and net worth. |
| **Month-to-month control** — category rollover and handling for current- and next-month income. | **Faster entry** — scheduled transactions, payee rules, category search and keyboard-friendly editing. | **Export your data** — CSV, QIF and platform-supported PDF/printing workflows. |

### More to work with

- **Statement imports:** CSV, QIF and OFX/QFX with preview, duplicate detection and matching.
- **Legacy budget import:** validation and preview before applying supported legacy data.
- **Undo and recovery:** atomic Undo/Redo, saved versions and JSON backup/restore.
- **Currency-aware data:** fixed-point monetary amounts and optional exchange-rate lookup. Expanded multi-currency workflows are still under development.
- **Privacy by design:** no bank connection or account registration; budgets remain on your computer unless you choose to export or back them up elsewhere.

## Quick start

### Try it with fictional data

Install [Rust](https://www.rust-lang.org/tools/install) (stable, with Rust 2024 edition support), then:

~~~sh
git clone https://github.com/perevozchiko/budget-tinker.git
cd budget-tinker
cargo run --locked
~~~

To create and open an **isolated demo budget** on macOS or Linux, instead run:

~~~sh
export BUDGETTINKER_DATA="$(mktemp -d /tmp/budgettinker-demo.XXXXXX)/demo.budgettinker"
cargo run --locked --example demo
cargo run --locked
~~~

The demo uses synthetic financial data. Its location is separate from your everyday budget. Fixture generators can overwrite the chosen destination, so use a fresh temporary directory.

### Build a macOS application

On macOS, install Apple Command Line Tools (<code>xcode-select --install</code>). Swift is needed for the PDF helper and Python 3 for distribution-notice staging.

~~~sh
bash scripts/bundle.sh
codesign --verify --deep --strict dist/BudgetTinker.app
plutil -lint dist/BudgetTinker.app/Contents/Info.plist
~~~

This creates an ad-hoc signed, **not Apple-notarized**, local application. For Windows, use the Rust MSVC toolchain and Visual Studio C++ Build Tools. For Ubuntu, install the native dependencies listed in [platform support](docs/PLATFORMS.md) or below:

<details>
<summary>Ubuntu build dependencies</summary>

~~~sh
sudo apt-get install build-essential pkg-config libx11-dev libxcursor-dev libxrandr-dev libxi-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev libdbus-1-dev
cargo run --locked
~~~

</details>

## Downloads

**[Download a BudgetTinker build](https://github.com/perevozchiko/budget-tinker/releases)** · [View tags and version history](https://github.com/perevozchiko/budget-tinker/tags)

Release archives are built for:

| Platform | Archive format | Verification |
| --- | --- | --- |
| macOS Apple Silicon | <code>BudgetTinker-&lt;version&gt;-macos-arm64.zip</code> | Locally verified |
| Windows x64 | <code>BudgetTinker-&lt;version&gt;-windows-x64.zip</code> | CI build; desktop behavior not fully verified |
| Linux x64 | <code>BudgetTinker-&lt;version&gt;-linux-x64.tar.gz</code> | CI build; desktop behavior not fully verified |

These are early **alpha** builds, not production-ready stable releases. GitHub Actions artifacts may also be downloaded from workflow runs; those downloads are wrapped by GitHub in another ZIP. Some file dialogs and printing features remain macOS-specific; read the [platform matrix](docs/PLATFORMS.md) for details.

## Your data stays yours

Budgets are stored locally as **<code>.budgettinker</code> files** using SQLite. Older compatible SQLite budgets can be opened explicitly without renaming; existing data is not moved or removed automatically.

| System | Default budget location |
| --- | --- |
| macOS | <code>~/Library/Application Support/BudgetTinker/budget.budgettinker</code> |
| Windows | <code>%LOCALAPPDATA%/BudgetTinker/budget.budgettinker</code> |
| Linux | <code>$XDG_DATA_HOME/budgettinker/budget.budgettinker</code> (or <code>~/.local/share/</code>) |

Set <code>BUDGETTINKER_DATA</code> to choose a different budget file. The application can keep recent-file preferences beside your budget. Backups and exports use their own formats.

**Privacy and safety:** budget files are **not encrypted**. There is currently no mobile/cloud synchronization and no bank integration. Optional currency-rate lookup contacts Frankfurter with the requested currency and date, not your transactions. Avoid copying a live SQLite database while WAL writes are active; make a backup through the application or the SQLite backup API.

## Releases and development

The <code>master</code> branch is the default branch. Pushes to <code>master</code> run Rust validation and builds for all three platforms. Other branches can be built manually using **Actions → Rust checks and desktop builds → Run workflow**, selecting a target platform or all platforms.

To publish a new release, use **Actions → Release → Run workflow** on <code>master</code>. Select a version change (**Current / Patch / Minor / Major**) and release stage (**Alpha / Beta / RC / Stable**). The release workflow calculates the version, performs validation and desktop builds, and publishes a tag and GitHub Release **only if all checks pass**.

<details>
<summary>Manual release and source-packaging details</summary>

<code>Cargo.toml</code> is the source of truth for the version. To update the package and lockfile together:

~~~sh
python3 scripts/bump-version.py 0.1.0-alpha.5
~~~

Commit and merge the version change into <code>master</code>, wait for CI to pass, then tag that exact commit:

~~~sh
git pull --ff-only origin master
git tag v0.1.0-alpha.5
git push origin v0.1.0-alpha.5
~~~

The version above is **an example**, not a claim that it is the currently published release. Tags must match the Cargo version and point to a commit on <code>master</code>. Tag pushes validate and build before publishing the release. Notes are generated from PR labels configured in [<code>.github/release.yml</code>](.github/release.yml).

To create a locally verified source-review archive without publishing:

~~~sh
python3 scripts/package-source.py
~~~

This writes <code>dist/BudgetTinker-source.zip</code>, including workflow files, licenses and third-party notices while excluding generated builds, local budgets and caches.

</details>

### Development checks

~~~sh
cargo fmt --all --check
cargo test --locked --quiet
cargo clippy --all-targets --locked -- -D warnings
cargo check --examples --locked
~~~

## Documentation

| Guide | What you'll find |
| --- | --- |
| [Platform support](docs/PLATFORMS.md) | Verified environments, limitations and native desktop integrations |
| [Development plan](docs/DEVELOPMENT-PLAN.md) | Completed work, ongoing improvements and follow-up items |
| [Validation](docs/VALIDATION.md) | Test and verification information |
| [Release audit](docs/PUBLIC-RELEASE-AUDIT.md) | Release-readiness checks and known limitations |
| [Dependency audit](docs/DEPENDENCY-AUDIT.md) | Third-party license checks and the narrowly scoped font-parser advisory exception |
| [Contributing](CONTRIBUTING.md) | Working on the codebase |
| [Security](SECURITY.md) | Security notes and reporting |
| [Attribution](ATTRIBUTION.md) | Assets, branding and third-party attributions |

## License

BudgetTinker is available under **MIT OR Apache-2.0**, at your option: [MIT](LICENSE-MIT) · [Apache-2.0](LICENSE-APACHE) · [license scope](LICENSE-STATUS.md).

Third-party components retain their own license terms. BudgetTinker is an independent project and is not affiliated with any commercial budgeting software.
