# Icon adjustment — 2026-10-28

The owner requested a smaller Dock icon with a cyan/teal background and a fine
checkered texture. Built-in imagegen edited the owner-supplied project emblem.

The edit specification retained the white, mint and yellow emblem geometry, placement
and proportions, with transparent corners and no text, new objects, bevels or
presentation margin. No application screenshot or UI layout was generated or modified.

Output was copied into `assets/icon-artwork.png`. `scripts/icon-padding.swift` draws it at
824×824 centered on a transparent 1024×1024 canvas, clipping the rounded-square boundary;
`scripts/icon.sh` derives the native iconset. `assets/app-icon.png` is also embedded by
eframe: changing only .icns does not fix Dock sizing because the runtime PNG overrides it.
The outdated blue SVG was excluded; the ICO is generated from the padded PNG.

## Rights and distribution status

Both the original emblem and this edit were generated with AI assistance and modified
for BudgetTinker. Source PNG, runtime PNG and ICO are project branding, not evidence of
exclusive copyright, registered trademark rights, or guaranteed uniqueness. Artwork terms are in ../assets/LICENSE.md; MIT OR Apache-2.0 applies to rights the project can grant. The edit specification
above records requested invariants, not a claim that generative editing preserved every
pixel. This follow-up does not change the approved image bytes or visual appearance.
