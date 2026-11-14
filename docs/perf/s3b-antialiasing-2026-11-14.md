# S3b: antialiasing per shape kind against Chromium (2026-11-14)

One-scene row S3b (`docs/plans/one-scene.md`). Measured with
`cargo run -p vapors-elements-corpus --example aa_methods` (add
`-- --out <dir>` for the images) at main 854ccb8, Chromium 155.0.8059.79,
on the Intel UHD 630 (Vulkan). A second run on lavapipe gave the same
table except one or two strongly-off pixels in the sub-pixel case.

What is drawn: touche's ten icons (`fixtures/icons_paint.wisp`: paths
stroked 1.5 wide in a 16-unit viewBox, dark #1d1d1f on #ececf0), each
in a 48 px cell. The placements are 1x, 2x (device scale 2), rotated 30
degrees, and scaled 1.37 (CSS transforms on the svg). Each cell is
judged by the `paint_check` rules (`crates/elements-corpus/src/paint_check.rs`)
against Chromium's drawing of the same svg. The closed-form kind is two
circles and two rects, filled and stroked, also drawn from their paths
by the mesh methods. The known limits are a 0.25 px stroke, and Tools
and Paste in translucent ink (alpha 0.5) where their strokes overlap.

The methods:
- **analytic**: the mesh pipeline as built (S3a2: outline moved half a
  device pixel out, coverage from the triangle's own outline edges);
- **MSAA 4x**: the same triangles filled solid, not moved, in a 4x
  multisampled target that is resolved;
- **exact corners**: a shader-only prototype of the candidate in S3b's
  row. Each triangle carries its three corners' positions, miter offsets
  and convexity, and coverage is taken from each corner's wedge. It uses
  the same mesh and the same move as the analytic method;
- **closed form**: the closed-form pipeline (exact distance to a rect or
  ellipse);
- **icon route**: vapors-text's icon rasteriser. It draws only
  axis-aligned at whole pixels, so it has no result for the rotated and
  scaled placements.

The flattening tolerance is 0.1 device pixels throughout, except in the
tolerance table at the end.

| placement | method | pass | failing icons (pixels over rules 1-2 / strongly off / max difference) |
|---|---|---|---|
| 1x | analytic | 3/10 | open (fail 7 / 6 / 196); paste (fail 2 / ≤2 / 117); export (fail 1 / ≤2 / 101); undo (fail 1 / ≤2 / 175); redo (fail 1 / ≤2 / 175); expand (fail 1 / ≤2 / 101); mute (fail 3 / ≤2 / 135);  |
| 1x | MSAA 4x | 9/10 | mute (fail 2 / ≤2 / 127);  |
| 1x | exact corners | 9/10 | mute (fail 2 / ≤2 / 135);  |
| 1x | icon route | 9/10 | mute (fail 2 / ≤2 / 163);  |
| 2x | analytic | 0/10 | open (fail 16 / ≤2 / 156); paste (fail 22 / 4 / 156); export (fail 11 / ≤2 / 104); undo (fail 4 / ≤2 / 104); redo (fail 4 / ≤2 / 105); tools (fail 24 / ≤2 / 104); expand (fail 1 / ≤2 / 101); voice (fail 18 / ≤2 / 104); mute (fail 7 / 3 / 156); send (fail 2 / ≤2 / 104);  |
| 2x | MSAA 4x | 10/10 |  |
| 2x | exact corners | 10/10 |  |
| 2x | icon route | 10/10 |  |
| rotated 30 | analytic | 3/10 | open (fail 5 / 4 / 193); paste (fail 7 / 6 / 180); export (fail 2 / ≤2 / 174); undo (fail 3 / 3 / 209); redo (fail 3 / 3 / 183); tools (fail 2 / ≤2 / 107); mute (fail 2 / ≤2 / 178);  |
| rotated 30 | MSAA 4x | 10/10 |  |
| rotated 30 | exact corners | 10/10 |  |
| rotated 30 | icon route | n/a | draws axis-aligned at whole pixels only |
| scaled 1.37 | analytic | 3/10 | open (fail 7 / 3 / 182); paste (fail 5 / 5 / 185); export (fail 2 / ≤2 / 170); undo (fail 4 / 4 / 156); redo (fail 3 / 3 / 162); tools (fail 8 / ≤2 / 104); mute (fail 3 / 3 / 188);  |
| scaled 1.37 | MSAA 4x | 10/10 |  |
| scaled 1.37 | exact corners | 10/10 |  |
| scaled 1.37 | icon route | n/a | draws axis-aligned at whole pixels only |

| placement | method (closed forms) | pass | failing shapes |
|---|---|---|---|
| 1x | closed form | 4/4 |  |
| 1x | MSAA 4x | 4/4 |  |
| 1x | exact corners | 4/4 |  |
| 2x | closed form | 4/4 |  |
| 2x | MSAA 4x | 4/4 |  |
| 2x | exact corners | 4/4 |  |
| rotated 30 | closed form | 4/4 |  |
| rotated 30 | MSAA 4x | 4/4 |  |
| rotated 30 | exact corners | 4/4 |  |
| scaled 1.37 | closed form | 4/4 |  |
| scaled 1.37 | MSAA 4x | 4/4 |  |
| scaled 1.37 | exact corners | 4/4 |  |

| case | method | pass | detail |
|---|---|---|---|
| sub-pixel stroke | analytic | 0/1 | 0.25px line (fail 4 / 11 / 173);  |
| sub-pixel stroke | MSAA 4x | 0/1 | 0.25px line (fail 0 / 4 / 78);  |
| sub-pixel stroke | exact corners | 0/1 | 0.25px line (fail 0 / 9 / 80);  |
| sub-pixel stroke | icon route | 1/1 |  |
| translucent overlap | analytic | 2/2 |  |
| translucent overlap | MSAA 4x | 2/2 |  |
| translucent overlap | exact corners | 2/2 |  |
| translucent overlap | icon route | 2/2 |  |

| placement | tolerance (px) | undo | redo |
|---|---|---|---|
| 1x | 0.25 | fail 2 / ≤2 / 198 | fail 2 / ≤2 / 198 |
| 1x | 0.1 | fail 1 / ≤2 / 175 | fail 1 / ≤2 / 175 |
| 1x | 0.05 | fail 3 / 3 / 198 | fail 3 / 3 / 198 |
| 2x | 0.25 | fail 4 / ≤2 / 104 | fail 4 / ≤2 / 105 |
| 2x | 0.1 | fail 4 / ≤2 / 104 | fail 4 / ≤2 / 105 |
| 2x | 0.05 | fail 4 / ≤2 / 104 | fail 4 / ≤2 / 105 |
| rotated 30 | 0.25 | fail 5 / 5 / 209 | fail 4 / 4 / 183 |
| rotated 30 | 0.1 | fail 3 / 3 / 209 | fail 3 / 3 / 183 |
| rotated 30 | 0.05 | fail 3 / 3 / 209 | fail 2 / ≤2 / 142 |
| scaled 1.37 | 0.25 | fail 4 / 4 / 163 | fail 3 / 3 / 151 |
| scaled 1.37 | 0.1 | fail 4 / 4 / 156 | fail 3 / 3 / 162 |
| scaled 1.37 | 0.05 | fail 1 / ≤2 / 156 | fail 4 / 4 / 181 |

## Reading

- The analytic mesh method fails most icons at every placement, all ten
  at 2x. The failing pixels are at stroke ends and joins. For example,
  Tools at 2x has 24 failing pixels, four corners on each of its six
  segments. A triangle that does not carry a corner's outline edge
  covers the pixels on that edge fully (one-scene 1.2's corner limit).
  Lyon's stroke triangles make this three pixels long at a butt cap,
  not the "at most one pixel per corner" 1.2 estimated.
- The exact-corner prototype and MSAA 4x pass 39 of the 40 icon cells
  and all 16 closed-form cells. The one failure, Mute at 1x, fails for
  every method including the icon route: its joins are sharper than
  the miter limit, and Chromium's join differs from all of ours there.
- The closed-form pipeline passes all 16 closed-form cells.
- No shape method passes the 0.25 px stroke, though the icon route does.
  Strongly-off pixels: analytic 11, exact corners 9, MSAA 4.
- The translucent overlap passes for every method. Blending twice where
  Tools' lines cross stays inside the rules at alpha 0.5.
- The tolerance makes no consistent difference between 0.25 and 0.05
  px. The analytic method's failures come from the corners, not from
  flattening.
