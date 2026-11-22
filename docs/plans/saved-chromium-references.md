# Saved Chromium references: thin slices

The paint corpus (`crates/elements-corpus/tests/gpu_paint.rs`) launches
headless Chromium in every run to screenshot each fixture, and holds
vapors' render to that screenshot. On the hosted runner the same pinned
Chromium 157 draws `text_line_box_paint`'s glyph rows one device pixel
off from this machine (runs 45067990897 and 45072756560: 9 boxes one
row off), so the check depends on where it runs. Tom decided on
2026-11-22 that ordinary checks would compare against a committed
Chromium screenshot instead, captured with the pinned build (#610) and
regenerated on request.

## R1: `text_line_box_paint` at 2x from a saved reference (this slice)

The two 2x tests of `text_line_box_paint`, `gpu_at_2x_placement` (the
ink-bounds check) and `gpu_at_2x_still_differs` (the tripwire), would read
Chromium's side from a committed file and launch no browser. The 1x test
and every other fixture would stay live until later slices.

- Files: `crates/elements-corpus/fixtures/references/text_line_box_paint_2x.png`
  holds Chromium's own PNG, byte for byte as it wrote it, and
  `text_line_box_paint_2x.json` is its manifest:
  - `page_sha256`: the HTML page the capture loaded, which covers the
    `.wisp` and the page template;
  - `fonts` and `images`: the sha256 of each file written beside the page;
  - `browser`: the `chromium.version` and `chromium.binary_sha256` pinned
    in `.config/tools.toml` at capture time;
  - `width`, `height` and `scale`: 640, 400 and 2;
  - `flags`: Chromium's render flags at this size and scale, as
    `vapors_text::comparison::render_flags` gives them. That is a new
    function holding the six flags `Chromium::command` passes today, and
    `command` would call it, so the capture and the check read one list.
    The per-run paths (`--screenshot`, #603's `--user-data-dir`) stay out
    of it;
  - `png_sha256`: the PNG's hash;
  - `host`: the capture machine's OS and kernel, recorded for a reader
    and not checked, since it is expected to differ.
- Check: before comparing, the test would recompute each checked field
  from the tree as it is and refuse a missing manifest or PNG, a field
  that differs, or a PNG whose hash differs. Each refusal names the file,
  the field, both values, and the regeneration command.
- Regeneration: `cargo xtask reference text_line_box_paint 2` runs the
  corpus example `paint_reference`. The example finds the pinned
  Chromium, refuses a binary whose sha256 is not the pin's (for example
  a `VAPORS_CHROMIUM` override), screenshots the page as the live path
  does, and writes the PNG and its manifest.
- One answer: the corpus reads the pin's version and hash from
  `.config/tools.toml`, the same file `vapors_text`'s `chromium_pin`
  reads. A unit test would hold the version it reads to the directory
  `chromium_pin::pinned_binary` names.
- The ink-bounds assertion stays as it is. Only where Chromium's image
  comes from changes.
- Shifted text: a gpu test would move the saved reference's pixels down
  one device row and assert the placement check refuses it.
- No browser: the slice's tests are run with `VAPORS_CHROMIUM` set to a
  path that does not exist and `/usr/bin/chromium` hidden from PATH, and
  must pass. The output goes in the PR.
- Unchanged: ci.yml, the rest of `crates/text/src/comparison`, xtask's downloads,
  and the withdrawn hosted diagnostic (branch
  `diag/chromium-text-placement`, no PR).

Tests:

- unit (`paint_reference` in vapors-elements-corpus): a missing manifest,
  a missing PNG, each stale field, a field the manifest lacks, a missing
  `host`, an unrecognised field, and a PNG of another hash or size are
  each refused with the file, field and command; a matching manifest
  gives the decoded image; the pin read from tools.toml is
  `chromium_pin`'s; a capture of a fixture that is not a paint fixture,
  or at scale 0, is refused before any browser starts.
- gpu: the two 2x tests of `text_line_box_paint` pass from the saved
  reference; the reference moved one device row down, or one device
  column right, is refused.

Known limits, from review:

- A capture checks the sha256 of the Chromium executable only. A copy of
  the pinned executable beside other `.pak` or `.so` files would be
  accepted. Rendering inputs outside the page (system fallback fonts,
  fontconfig) are recorded only in `host`, which is not compared.
- The corpus parses `[downloads] chromium.*` itself, mirroring
  `chromium_pin::pinned_field`. A test holds the version it reads to
  `chromium_pin::pinned_binary`. Making `pinned_field` public in #610
  would leave one parser.
- `sha256` waits on `sha256sum` without a deadline, as xtask's does; it
  reads a file or a pipe this process has already closed.

## Next slices (to replan after R1)

- R2: the 1x test of `text_line_box_paint` and the other `only_1x`
  fixtures' placement tests.
- R3: every paint fixture at 1x and 2x, then `chromiums` removed from the
  ordinary path, keeping the live comparison as a named command.
