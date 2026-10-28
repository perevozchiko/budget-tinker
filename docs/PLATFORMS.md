# Desktop platform support — 2026-10-28

macOS arm64 is the locally built/tested host. Windows x86_64 MSVC and Linux x86_64 GNU
remain manual build targets, not fully verified products. No UI layout, styling or
accounting design is changed by this portability pass.

| Operation | macOS | Windows | Linux |
| --- | --- | --- | --- |
| Native window, register, budget, reports and integer accounting | Locally verified | Source path retained; no local runtime/build verification | Source path retained; no local runtime/build verification |
| SQLite defaults, autosave, stored schema and import calculations | Existing tests pass | Target-specific default path retained | Target-specific default path retained |
| Named bold/italic fonts | Existing installed Arial families preserved | Existing egui bundled fonts used when Arial paths are absent | Existing egui bundled fonts used when Arial paths are absent |
| Ordinary Open/Save file dialog | Existing AppleScript chooser | Explicit English unsupported message | Explicit English unsupported message |
| legacy source folder chooser | Existing AppleScript chooser | Explicit message; enter folder path directly | Explicit message; enter folder path directly |
| Bank statement/import and JSON restore path fields | Existing chooser/manual entry | Manual path entry and model operations remain available | Manual path entry and model operations remain available |
| New budget/Fresh Start, UI backup/CSV/QIF/PDF save workflows requiring a chooser | Existing behavior | Not available through the current native picker | Not available through the current native picker |
| Initial budget selection | Existing defaults, preferences, BUDGETTINKER_DATA | Default or BUDGETTINKER_DATA; recent path preferences | Default or BUDGETTINKER_DATA; recent path preferences |
| Reveal budget | Existing `open -R` in Finder | Launch `explorer.exe` with parent directory | Launch `xdg-open` with parent directory |
| Native print / PDF export | Existing AppleScript/AppKit printing and embedded Swift PDF helper | Explicit unsupported message; on-screen preview remains | Explicit unsupported message; on-screen preview remains |
| Native application menu | Existing AppKit menu | Existing no-op native-menu adapter; in-app commands only | Existing no-op native-menu adapter; in-app commands only |
| Optional exchange-rate lookup | Existing /usr/bin/curl | curl executable required on PATH | curl executable required on PATH |
| macOS app/icon bundling | Swift, sips, iconutil, codesign and Python 3 required | Not applicable | Not applicable |

The unsupported messages use the existing error presentation, with no added controls or
redesigned dialogs. Model/parser support does not imply a native file picker is available.
CSV/report export and JSON backup APIs can still be used from Rust; this task does not add
new CLI commands, platform dialogs or a new dependency. Documents/import identifiers remain
unchanged. Cancellation of a macOS chooser is still a cancellation, not an error.

`src/desktop.rs` isolates dialog and folder-launch commands. Paths are passed as separate
process arguments, never through a command shell. A missing launcher is reported. Launch
success does not prove an external file manager opened a window. Non-macOS desktop sessions
and handlers still require actual native verification.

`build.rs` compiles the Swift helper only for macOS; report PDF/print methods already return
English errors elsewhere. Non-report native printing now returns a clear error before writing
an otherwise orphaned temporary text file. The library's exchange-rate function searches
PATH for curl outside macOS rather than assuming /usr/bin/curl. Network lookup was not
exercised with personal data during this work.

The timeframe preset/Done/Cancel/Custom keyboard/event test is retained on every platform,
using egui's default embedded fonts under the existing arial-bold alias. There is no platform
skip. Headless font tests verify missing named families render and preinstalled families
are not overwritten. Product validation strings are translated at UI error boundaries;
Russian names/memos entered by users, serialized legacy strings and import recognizers
are not rewritten.

## Verification limits

Only macOS/iOS Rust targets are installed locally. No Linux/Windows Rust target, Linux
sysroot/host, Windows SDK/linker/runtime, Docker, Podman, Zig, MinGW or PowerShell is available.
The platform-specific Windows/Linux branches and native launchers have not been compiled or
run here. Target dependency graphs and manual workflow/package definitions can be inspected
without claiming a successful build. Run real native builds and GUI/import/print-error flows
before advertising Windows/Linux feature support. Never copy system Arial font files into
the source tree as a workaround; existing dependency font notices remain packaged.
