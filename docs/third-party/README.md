# Dependency notice inventory

All **327 locked registry crates** are inventoried, including inactive optional and foreign
target packages. `active_feature_graph` distinguishes the default/all-root-feature graph
without a platform filter from inventory-only packages. It does not mean linked on macOS.
Each record preserves the original SPDX declaration, selected permissive branch, upstream
authors/source, notice filenames and SHA-256 hashes. This is a record of dependency terms,
not an assertion that BudgetTinker owns them. Its active project license does not replace these terms.

Every crate has collected notice records. Most texts are upstream files reproduced
verbatim. `notice_sources` and `notice_kinds` distinguish other cases:

- `r-efi` uses AUTHORS for its full license and copyright notices (MIT branch selected).
- Winapi GNU companion crates have platform-specific licenses from the exact parent
  winapi source commit. Their crate manifests lack VCS metadata; no byte-for-byte source
  match of the generated import archives is asserted. These GNU packages are not selected
  by the intended Windows MSVC, macOS arm64 or Linux desktop build targets.
- `dispatch` declares MIT and Steven Sheldon as author in its release manifest, with no
  standalone license/copyright file in the exact release tree. The preserved declaration
  and transparently assembled standard MIT terms do not invent a copyright holder/year.
- `hexf-parse` explicitly declares CC0-1.0. Its exact manifest and standard CC0 text are
  retained; the canonical text is not misrepresented as an upstream package file.
- The embedded SQLite public-domain notice is retained separately from its MIT wrapper.
- Additional font terms are retained in epaint_default_fonts-0.32.3. They govern the fonts,
  not all application code; see ../DEPENDENCY-AUDIT.md and ../../ATTRIBUTION.md.

No upstream license has been rewritten or replaced to force a policy pass. The optional
LGPL branches are not selected. Original author email addresses in these legal records
are intentionally retained. Build tools/system frameworks are distinguished in the audit.

`scripts/stage-notices.py` verifies hashes and stages the entire inventory for every OS;
it conservatively includes foreign/inactive package notices too. No dependency source,
font binary, commercial application resource or personal budget is vendored here.
