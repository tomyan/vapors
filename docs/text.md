# Text

`crates/text` (`vapors-text`) shapes and lays out text with Parley,
rasterises glyphs on the CPU into an atlas, and draws them through wgpu.
The brief is `UI-FRAMEWORK-BRIEF.md` section 3 in the touche repository.

## Stack

| Concern | Crate | Pinned | Why this one |
|---|---|---|---|
| Font collection, fallback | fontique | 0.12.0 | Parley's own; system fonts through fontconfig (dlopen) on Linux, none on the web |
| Shaping, bidi, line breaking | parley | 0.12.0 | Ranged styles, `complex-scripts` on |
| Outlines, hinting | skrifa | 0.44.0 | The release that shares fontique's read-fonts 0.41; its autohinter does the light hinting |
| Rasterisation | zeno | 0.3.3 | The rasteriser swash wraps; used directly, so there is no extra layer and no cap on the skrifa version |

Bundled fonts: Geist and Geist Mono, variable weight, from the official
release (`crates/text/fonts/README.md` has version, URL and checksums).
Neither has an optical size axis; the `opsz` setting is accepted and
ignored by them. On native the bundle is registered first and system
fonts fall back behind it; on the web only the bundle exists and a
character outside it draws `.notdef`. `FontSystem::register_fallback`
adds a font as the fallback for one script, which is how the web will
take a fetched font (F6). The per-character fallback test uses a 2 KB
one-glyph subset of Droid Sans Fallback (`crates/text/fonts/test/`,
Apache 2.0) so it does not depend on the machine's fonts; the
system-font test requires a CJK font only when `VAPORS_SYSTEM_FONTS=1`,
which CI sets after installing `fonts-noto-cjk`.

## Rendering

- Glyphs are rasterised per (font, size, variation, subpixel step,
  hinting) at one of four horizontal subpixel steps; baselines are
  rounded to device pixels. Grayscale coverage only.
- Hinting: skrifa's autohinter in light (vertical-only) mode below 2x,
  none at 2x and above (`Hinting::for_scale`).
- Atlas: one R8 page for coverage, one RGBA8 page for colour glyphs
  (present, not yet filled: colour emoji are deferred). Shelf packing;
  when a page fills, every glyph the current frame has not used is
  evicted and the rest repacked. `GlyphAtlas::evictions()` and
  `rasterisations()` count both. A frame that cannot fit on its own is
  refused with the page size and the fix.
- A glyph that fails to rasterise is drawn as `.notdef` and logged with
  the font blob and glyph id. Only an unreadable font or a broken
  `.notdef` is an error.
- Blending: the shader (`crates/text/src/text.wgsl`) turns coverage into
  alpha the way Skia's mask gamma does. For text of encoded luminance
  `s` it assumes a background of `1 - s`, blends the two in linear light
  (`x^gamma`) with the coverage boosted by `coverage * (1 - coverage) *
  contrast * linear_background`, encodes the result, and solves for the
  alpha that reproduces it when the hardware blends in encoded space.
  The target is `Rgba8Unorm`, so that is the blend the hardware does.

### Parameters

`BlendParams { gamma: 1.2, contrast: 0.2 }`. These are Chromium's Skia
values on Linux (`SK_GAMMA_EXPONENT=1.2`, `SK_GAMMA_CONTRAST=0.2`) and
the comparison page confirmed them on 2026-10-22 against Chromium
155.0.8059.79 on Debian 13. Mean ink of each block, ours divided by
Chromium's, from `cargo run -p vapors-text --example text_comparison`:

| gamma, contrast | 13 px light on dark | 13 px dark on light | 28 px heading, both | worst block |
|---|---|---|---|---|
| **1.2, 0.2** | 0.999 | 1.025 | 0.999, 1.008 | 0.978 (14 px light on dark) |
| 1.0, 0.0 (no adjustment) | 0.948 | 1.027 | 0.981, 1.009 | 0.929 |
| 1.2, 0.5 | 1.003 | 1.096 | 1.000, 1.033 | 1.103 |
| 1.8, 0.2 | 1.124 | 0.885 | 1.043, 0.960 | 0.869 |
| 2.2, 0.5 (Skia's Windows defaults) | 1.193 | 0.867 | 1.067, 0.954 | 0.851 |

Without the adjustment light-on-dark text is 5% to 7% lighter than
Chromium's; with 1.2 and 0.2 every block is within 3% in both
polarities. Stem darkening is off: it would add weight Chromium does
not have here. macOS and Windows renderers were not compared (there is
no such machine in this factory); the brief expects per-platform tuning,
and `BlendParams` is where it would go.

The composite from that run is `docs/text_comparison.png`: left half
vapors-text, right half Chromium, each with the light-on-dark and
dark-on-light panels.

## The comparison page

```
cargo run -p vapors-text --example text_comparison -- --out target [--gamma G --contrast C]
```

writes `target/text_comparison_vapors.png` (ours, offscreen on the
adapter `vapors-gpu` selects), `target/text_comparison.html` with the
fonts beside it, `target/text_comparison_chromium.png` (headless
Chromium at `--window-size=920,580`, scale 1, 5 s virtual time so the
fonts load), and the labelled composite `target/text_comparison.png`.
It prints the mean ink of every block in both renderings and fails
unless the 13 px light-on-dark body's ratio is inside
`comparison::INK_BAND` (0.7 to 1.4). The band is loose on purpose: it
catches a renderer that went blank, bold or misplaced, and tolerates
Chromium's fontconfig hinting differing between machines. The same
check runs in the `gpu` tier as `gpu_body_paragraph_ink_is_within_the_band_of_chromiums`,
and CI's `gpu` job runs the example and uploads the PNGs as the
`text-comparison` artefact.

Chromium is found through `VAPORS_CHROMIUM`, else `chromium`,
`chromium-browser`, `google-chrome`, `google-chrome-stable` on `PATH`;
the failure lists what was tried. The run has a 60 s deadline (about 1 s
here, under 10 s measured on a cold runner) and refuses a screenshot
that is not the requested size.

The page spec (`comparison/spec.rs`) places every block at fixed
coordinates, and the HTML uses the same absolute positions, so a region
means the same thing in both renderings. Family names inside the inline
`style` attributes are single-quoted; double quotes there end the
attribute and Chromium silently falls back to a serif (the first run did
exactly that, and a test now pins it).

## The label in the image app

`vapors-canvas`'s `ImageApp` draws a 44 px strip (`#171b1f`) across the
top of every frame with `"<file name>  <zoom>%"` in Geist 13 px
(`#e4e8ec`, tabular figures) 12 px from the left; `no file` before
anything is opened. The constants are in `crates/canvas/src/label.rs`.
The strip is the touche design's bar; the centre pixel the screenshot
steps check is below it. On the web the page's Open button sits at the
top right so it does not cover the label.

## Using it

```rust
let mut fonts = FontSystem::new();            // bundle, then system fonts; bundle_only() on the web
let style = Style { tabular_figures: true, color: ink, ..Style::new(Family::Geist, 13.0) };
let layout = Layout::new(&mut fonts, "R 238, G 231, B 222", &style, &LayoutOptions::default());
let mut atlas = GlyphAtlas::new(AtlasConfig::default());
let mut pipeline = TextPipeline::new(&device, target_format, &atlas);
atlas.begin_frame();                           // once per frame: what eviction keeps
pipeline.draw(&RenderTarget { device, queue, view, viewport }, &mut atlas, &layout, [12.0, 14.0], BlendParams::default())?;
```

`LayoutOptions::scale` is device pixels per logical pixel; the layout
comes back in device pixels and `Hinting::for_scale` picks light
hinting below 2x. `Layout::styled` takes spans that override ranges.

## Deferred

- Text editing: selection, caret, IME, clipboard (F4).
- Web font fallback fetching for scripts outside the bundle (F6); such
  text draws `.notdef` on the web today.
- Colour emoji (COLRv1 and bitmap glyphs): the RGBA8 page exists, nothing
  is rasterised into it yet.
- Large text above about 64 px through a capped atlas size or paths.
- Atlas uploads copy the whole page when anything changed; per-rectangle
  uploads when it shows in a profile.
- Per-platform parameters for macOS and Windows, and the native text
  view comparison the brief asks for on those platforms.
