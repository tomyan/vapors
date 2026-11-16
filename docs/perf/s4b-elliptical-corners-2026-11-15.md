# S4b: elliptical corners against Chromium (2026-11-15)

One-scene S4b, PR 1. The closed-form shader draws a rect's corners with
rx and ry unequal, and ellipses, from their exact distance.

- **The distance:** the nearest point of a quarter ellipse is refined
  by a fixed four steps (`quarter_ellipse` in `closed_forms.wgsl`, C.
  Chatfield's trig-free method). A rect's elliptical corner takes the
  nearest of its top edge, its side edge and that quarter ellipse,
  each as a piece of the outline. So a stroke deeper than the smaller
  radius is continuous where the pieces meet (#386 review, H1).
- **Before:** the PR's first version took the corner's distance to
  first order. That gave 0.72 px at an 8 x 1 px corner and 0.53 px on
  a 1.5 px stroke at 3 x 1.5 px. It also jumped at the corner/edge
  boundary under a deep stroke.

## The step count

Measured in f64 against the exact distance, which a dense search
narrowed by ternary search finds. The radii run from 1 x 1 to
200 x 25 px, with ratios to 8:1 either way, at points across the whole
corner, inside and out. The largest error, in px:

| steps | radii to 12 x 6 px | radii 30 x 30 to 200 x 25 px |
|---|---|---|
| 1 | 0.27 | |
| 2 | 0.012 | |
| 3 | 0.0027 | 0.0077 |
| 4 | 0.0027 | 0.00003 |

The 0.0027 left at small radii is the reference's own resolution: it
does not fall from 3 steps to 5. The error at three steps grows with
the radius, so the shader takes four.

## The cost per pixel

These counts are scalar operations in the WGSL source, not the
compiled instructions:
- **One refinement step:** about 46, made of 2 lengths, a normalize, a
  divide, a clamp and selects. Four steps come to about 185.
- **An ellipse:** the refinement plus about 20.
- **An elliptical corner:** the refinement plus about 35, for the two
  edge distances and the comparisons.
- **The first-order formula it replaces:** about 15.
- **A circle, and a rect whose radii are equal:** they keep their exact
  formulas without the refinement, because the radii are flat per
  instance and the branch is uniform across a quad.

## On the GPU

`crates/renderer/tests/gpu_closed_forms_exact.rs` holds every pixel to
within 0.05 px (plus 1.5/255 for rounding) of the exact distance. It
maps that distance to device pixels as the shader does, along the
exact gradient. The cases are:
- rects with 8:1 and 1:8 corners, filled and stroked 3, deeper than
  the 1 px radius;
- the 3 x 1.5 px corner stroked 1.5 at two fractional offsets and at
  2x;
- 8:1 and 1:8 ellipses, filled and stroked 4;
- a rotated corner, and a corner and an ellipse under a skewed
  non-uniform scale (`matrix(1.6, 0.3, 0.5, 0.9)`).

The largest error measured is 0.0035 coverage, which is within one
8-bit level. On the same machine:
- dropping the refinement (0 steps) fails every case, up to 1.0;
- the first-order formula fails the ellipses (up to 1.0 at stroke 4);
- the earlier corner/edge split fails the deep strokes (0.43);
- dropping the quadrant's sign from the gradient fails the skewed cases
  (0.25 for the corner, 0.17 for the ellipse).

## Against Chromium

Measured with `cargo run -p vapors-elements-corpus --example aa_methods`
on the browser and adapter below. This is a record of one run, not a
gate. PR 2's `<rect rx ry>` paint fixture puts the case under
`gpu_paint`, so until then CI compares this shader against the exact
reference only (#386 review, L1).

```
Chromium: Chromium 155.0.8059.79 built on Debian GNU/Linux 13 (trixie) (/usr/bin/chromium)
Adapter: Intel(R) UHD Graphics 630 (CML GT2) (Vulkan; Intel open-source Mesa driver Mesa 25.0.7-2+deb13u1)
```

The cases are a 10 x 8 rect with rx 3 and ry 1.5, in a 16-unit
viewBox, filled and stroked 1.5 wide. Chromium draws the same rect
from its path with arcs. The cells are judged by the `paint_check`
rules. Each entry gives pixels over rules 1-2, then pixels strongly
off, then the maximum difference in levels.

| placement | elliptical rect fill | elliptical rect stroke | round rect stroke (circular, for scale) |
|---|---|---|---|
| 1x | pass 0 / ≤2 / 35 | pass 0 / ≤2 / 23 | pass 0 / ≤2 / 15 |
| 2x | pass 0 / ≤2 / 39 | pass 0 / ≤2 / 29 | pass 0 / ≤2 / 32 |
| rotated 30 | pass 0 / ≤2 / 20 | pass 0 / ≤2 / 27 | pass 0 / ≤2 / 34 |
| scaled 1.37 | pass 0 / ≤2 / 38 | pass 0 / ≤2 / 43 | pass 0 / ≤2 / 33 |
