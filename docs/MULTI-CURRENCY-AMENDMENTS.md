# Owner amendments to multi-currency specification v2

## Numeric format — 2026-10-29

Owner instruction during implementation: use one numeric format `1 500.00`,
with spaces between thousands and a decimal point; no commas. This instruction
supersedes MC-006 and the numeric-format portions of sections 7 and 10 in the
verbatim specification. It applies in both product UI languages.

- Display: `1 500.00 EUR`, `125 000 JPY`, `125.345 KWD`, `-1 500.00 EUR`.
- Input: accept `1500.00` or correctly grouped `1 500.00`; reject `1,500.00`,
  `1,50`, `1 50.00`, mixed separators and excess currency fractional digits.
- Product labels remain English by default; currency codes and exact integer
  money/rate storage are unchanged. JPY retains 0 and KWD 3 fractional digits.
- Machine exports remain ungrouped decimals. Existing bank statement formats
  may contain comma grouping; their dedicated parser validates and normalizes
  it at import, independently of product entry/display. It must never interpret
  comma as a decimal point or silently repair malformed grouping.

The original supplied specification remains unchanged for traceability. Plans,
implementation and validation must follow this amendment wherever they conflict.
