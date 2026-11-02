# The text and layout stack's size in touché's page

Measured 2026-11-02 for the tool card (TOOLS-1), to start the next
slimming round from. The touché page's budget was raised from 900 KiB
to about 1.7 MB of brotli wasm for the card; this records where the
card's part goes and what each possible cut loses.

## How it was measured

touché main at b33c116 with vapors at 8a6fbba. A local probe (not
merged) opened the Exposure card, laid it out, gave it input and drew
it over the frame, behind a branch the optimiser cannot see through, so
the wasm holds what the card brings. Each figure is the page wasm's
brotli as `cargo xtask web` reports it (the `web` profile: fat LTO, one
codegen unit, opt-level "s"). "Measured" is a full build with the cut;
"estimated" scales a module's raw code bytes by the page's ratio of
shipped brotli to raw code (about 0.25).

| Build | Brotli wasm |
|---|---|
| touché without the card (probe off) | 794 KiB |
| with the card, parley's `complex-scripts` on | 3728 KiB |
| with the card, `complex-scripts` off | 1666 KiB (+872) |

## Where the +872 KiB goes

Raw bytes from twiggy on the unoptimised wasm, with an estimated brotli
share; the data figures are brotli of the data section.

| Part | Raw | Brotli (est.) |
|---|---|---|
| Fonts: Geist (72 KiB) and Geist Mono (75 KiB), embedded | 340 KB | 147 KiB, measured per file |
| Unicode, ICU, emoji and shaping tables | ~370 KB | ~80 KiB |
| harfrust (shaping) | 264 KB | ~65 KiB |
| read-fonts (font tables, CFF charstrings) | 255 KB | ~63 KiB |
| skrifa (outlines, hinting, autohint) | 236 KB | ~58 KiB |
| vapors-elements | 211 KB | ~52 KiB |
| vapors-css | 161 KB | ~40 KiB |
| taffy (grid 114 KB of it) | 153 KB | ~38 KiB |
| parley, parley_engine, fontique | 161 KB | ~40 KiB |
| zeno (raster) | 98 KB | ~24 KiB |
| core, alloc, hashbrown instances | ~300 KB | ~75 KiB |

## Cuts, ranked

Savings are against 1666 KiB.

| # | Cut | Saves | Loses | Decision |
|---|---|---|---|---|
| 1 | parley `complex-scripts` off | 2062 KiB, measured | Dictionary line breaking for Thai, Lao, Khmer and Burmese: text in those scripts breaks only at spaces and grapheme boundaries | Off in touché's page; keep it a vapors-text feature, on for native |
| 2 | Geist Mono out of the bundle | 84 KiB, measured | The bundled monospace face | Not taken: the design system sets numbers in Geist Mono with tabular figures |
| 3 | CSS grid (vapors-elements `grid` feature) | 54 KiB, measured | `display: grid` and the grid properties, refused when off | Taken: the `grid` feature, default on |
| 4 | Geist subset to Latin and punctuation | 43 KiB, measured (with 2) | Greek, Cyrillic, Vietnamese and other glyphs of the bundled face: tofu on the page | Not taken: layer and file names are the person's text, in any script; revisit only as a subset by what is displayed |
| 5 | CFF outlines (read-fonts `ps`, skrifa `cff`) | ~30 to 36 KiB, estimated | OpenType CFF fonts; skrifa 0.44 has no feature for it | Not now (upstream) |
| 6 | harfrust's complex shapers (Indic, USE, Khmer, Myanmar, Arabic, Hangul, Thai) and AAT | ~20 + ~9 KiB, estimated | Shaping for those scripts, and Apple AAT fonts; no feature upstream | Not now (upstream) |
| 7 | Glyph hinting (draw unhinted) | 9 KiB, measured | Hinted small text at 1x | Not taken |
| 8 | zeno stroking, skrifa variable composites | ~6 + ~4 KiB, estimated | Little | Not taken |

Cuts 2, 3, 4 and 7 together measured 1490 KiB (−176). What remains over
the old budget, about 600 KiB, is the engine itself: elements, CSS,
flex and block layout, the shaping core, glyph outlines, raster and the
Unicode data.
