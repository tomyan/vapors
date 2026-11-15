# S4b: elliptical corners against Chromium (2026-11-15)

One-scene S4b, PR 1. The closed-form shader draws a rect's corners with
rx and ry unequal. Inside a corner, past the corner's centre on both
axes, it takes the distance to that quarter ellipse to first order. It
uses `ellipse_distance`, as the ellipse kind already does. Measured
with `cargo run -p vapors-elements-corpus --example aa_methods` on the
browser and adapter below. This is a record of one run, not a gate.
PR 2's `<rect rx ry>` paint fixture puts the case under `gpu_paint`.

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
| 1x | pass 0 / ≤2 / 34 | pass 0 / ≤2 / 30 | pass 0 / ≤2 / 15 |
| 2x | pass 0 / ≤2 / 39 | pass 0 / ≤2 / 57 | pass 0 / ≤2 / 32 |
| rotated 30 | pass 0 / ≤2 / 19 | pass 0 / ≤2 / 48 | pass 0 / ≤2 / 34 |
| scaled 1.37 | pass 0 / ≤2 / 38 | pass 0 / ≤2 / 35 | pass 0 / ≤2 / 33 |

## The first-order error

The table below gives the error of `ellipse_distance` against the exact
distance to the ellipse. It is the largest over the corner, sampled at
1/4 px steps across the coverage ramp: the band from the outline where
coverage is between 0 and 1. All values are in device pixels.

| corner radii (px) | filled: ramp ±0.5 px | stroked 1 px: ramp ±1 px | stroked 1.5 px: ramp ±1.25 px |
|---|---|---|---|
| 12 x 6 | 0.008 | | |
| 8 x 4 | 0.013 | 0.057 | |
| 6 x 3 | 0.018 | | 0.14 |
| 3 x 1.5 | 0.042 | | 0.53 |
| 4 x 1 | 0.28 | | |
| 8 x 1 | 0.72 | | |

The error is always largest on the inside, where the ramp comes closest
to the corner's smallest radius of curvature, ry² / rx. It grows as
the ramp's inner edge approaches that radius.
- **Filled:** the error stays under 0.05 px while ry² / rx is above
  about 0.75 px. It reaches 0.7 px for an 8 x 1 px corner.
- **Stroked:** a stroke's inner ramp lies further inside, so its error
  is larger.

On the GPU (`crates/renderer/tests/gpu_closed_forms_elliptical.rs`),
the 8 x 4 px corner is within 0.010 of the exact reference filled and
0.042 stroked 1 px at 1x.

On the cases above, the 3 x 1.5 px stroked corner at 1x has a computed
0.53 px error at the inner ramp's far edge, but its largest difference
from Chromium is 30 levels: the pixel centres do not fall where the
error peaks.

An ellipse with unequal radii, already drawn by the closed-form
pipeline, has the same error.
