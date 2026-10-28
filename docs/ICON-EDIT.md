# Icon adjustment — 2026-10-28

Owner requested a smaller Dock icon and a teal background matching the application sidebar.
Built-in imagegen skill edited the original kit PNG with the owner's UI texture screenshot
as reference. No application screenshot or UI layout was generated or modified.

Prompt: Change only the original blue rounded-square background to the cyan/teal color
and fine checkered texture of the supplied sidebar reference. Preserve the white, mint and
yellow emblem geometry, placement and proportions; keep transparent corners, with no text,
new objects, bevels or presentation margin.

Output was copied into `assets/icon-artwork.png`. `scripts/icon-padding.swift` draws it at
824×824 centered on a transparent 1024×1024 canvas, clipping the rounded-square boundary;
`scripts/icon.sh` derives the native iconset. `assets/app-icon.png` is also embedded by
eframe: changing only .icns does not fix Dock sizing because the runtime PNG overrides it.
The outdated blue SVG was excluded; the ICO is generated from the padded PNG.

## Rights and distribution status

Both the original emblem and this edit were generated with AI assistance and modified
for BudgetTinker. Source PNG, runtime PNG and ICO are project branding, not evidence of
exclusive copyright, registered trademark rights, or guaranteed uniqueness. Artwork terms are in ../assets/LICENSE.md; MIT OR Apache-2.0 applies to rights the project can grant. The prompt
above records requested invariants, not a claim that generative editing preserved every
pixel. This follow-up does not change the approved image bytes or visual appearance.
