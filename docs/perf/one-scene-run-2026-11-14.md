# The one-scene example's 60 s run, 2026-11-14 (M3)

The minimal scene for property 9 (`crates/one-scene`): an HTML HUD
over a `gpu-surface`, a 3D scene of a ground plane and three cubes
turning, and CSS reaching the meshes over the run: a click on a HUD
button every 2 s toggles a cube's `lit` class, the page's time signal
hides the third cube in every other 5 s span and switches the ground's
`--tint` token in every other 15 s span. Driven by its own script
(`vapors-perf --scene-run one-scene`, #351), each frame confirmed to
draw every mesh as the schedule says.

- The run is 3,750 frames: 60 s of the frame clock at 16 ms a frame.
- Every frame is judged from the first, with no warmup (D7 (a)).
- A frame is over budget when its time from the start of `App::frame`
  to the return of the GPU wait after it is over 16.7 ms (D6 (a)).

## Setup

- Release build at 3c839a9 (#351's head and the script change below).
  Every run prints `3c839a9ae6de`, with no `-dirty` suffix. The
  binary's sha256 is in `one-scene-run-2026-11-14/sha256.txt`.
- The dev machine: Intel UHD 630 (CML GT2), Mesa 25.0.7 ANV, Vulkan,
  headless, rendering to an offscreen texture with no present. Scale
  1, 1100 x 720.
- The window: `cargo xtask quiet 30 -- <script>`, which held every
  xtask ci, test and gate run, then a wait for the 1-minute load to
  fall under 1.5 (it took 400 s after the lock was taken). Each run
  checks its load at the start and the end and refuses 1.5 or more;
  the highest read was 1.49, at run1-cold's end.
- Command: `scripts/scene-runs.sh docs/perf/one-scene-run-2026-11-14
  one-scene` (the script now takes the scene; racer by default). Three
  pairs, each a cold run on a new, empty `MESA_SHADER_CACHE_DIR` then
  a warm run on the cache it filled. Each run's output is kept
  verbatim in `one-scene-run-2026-11-14/`; the tables are
  `scripts/scene-run-table.sh` on those files.
- A run takes about 12 s of wall time.

```
run1-cold start 07:19:59 load 1.37 8.38 25.06
run1-cold end 07:20:10 exit 0 load 1.49 8.06 24.69
run1-warm start 07:20:10 load 1.49 8.06 24.69
run1-warm end 07:20:22 exit 0 load 1.41 7.83 24.43
run2-cold start 07:20:22 load 1.41 7.83 24.43
run2-cold end 07:20:33 exit 0 load 1.35 7.61 24.18
run2-warm start 07:20:33 load 1.35 7.61 24.18
run2-warm end 07:20:45 exit 0 load 1.38 7.40 23.94
run3-cold start 07:20:45 load 1.38 7.40 23.94
run3-cold end 07:20:57 exit 0 load 1.35 7.11 23.58
run3-warm start 07:20:57 load 1.35 7.11 23.58
run3-warm end 07:21:09 exit 0 load 1.37 6.92 23.34
```

## Result

Every run exits 0: no frame over budget in any of the six, 22,500
frames judged, each also confirmed to draw the scheduled colours and
visibility. The slowest frame of all is run2-warm's frame 0, 6.8 ms
to idle; after frame 0 no frame in any run took more than 5.4 ms.
The start-up (`App::start`, before frame 0, not judged) took 70 to
72 ms cold and 13 to 14 ms warm.

### Cold: a new, empty shader cache

Revision 3c839a9ae6de.

| run | cache | over budget | frame 0 CPU / GPU / to idle µs | frame (CPU) median / p99 / max µs | GPU time median / p99 / max µs | frame to idle median / p99 / max µs |
|---|---|---|---|---|---|---|
| run1-cold | cold | 0 | 2254.2 / 948.8 / 3278.5 | 1360.7 / 2315.9 / 3753.8 | 1526.8 / 1733.6 / 1960.4 | 3147.0 / 3832.8 / 5296.1 |
| run2-cold | cold | 0 | 2068.1 / 967.6 / 3131.4 | 1097.2 / 2325.9 / 3585.0 | 1529.6 / 1674.9 / 1994.6 | 2910.8 / 3847.7 / 5230.3 |
| run3-cold | cold | 0 | 2166.6 / 985.4 / 3209.4 | 1424.4 / 2088.5 / 3738.7 | 1526.1 / 1609.8 / 1788.4 | 3197.2 / 3708.4 / 5363.6 |

| time, over these runs | median of medians µs | median of p99s µs | median of maxima µs | worst frame µs |
|---|---|---|---|---|
| frame (CPU) | 1360.7 | 2315.9 | 3738.7 | 3753.8 |
| GPU time | 1526.8 | 1674.9 | 1960.4 | 1994.6 |
| frame to idle | 3147.0 | 3832.8 | 5296.1 | 5363.6 |

### Warm: the cache the cold run filled

Revision 3c839a9ae6de.

| run | cache | over budget | frame 0 CPU / GPU / to idle µs | frame (CPU) median / p99 / max µs | GPU time median / p99 / max µs | frame to idle median / p99 / max µs |
|---|---|---|---|---|---|---|
| run1-warm | warm | 0 | 1984.0 / 949.6 / 3035.3 | 1185.8 / 2148.9 / 3583.4 | 1526.2 / 1748.2 / 1961.0 | 2984.0 / 3746.6 / 5308.6 |
| run2-warm | warm | 0 | 5625.7 / 960.2 / 6842.7 | 1346.3 / 2256.6 / 5625.7 | 1528.3 / 1628.3 / 1783.8 | 3101.8 / 3799.9 / 6842.7 |
| run3-warm | warm | 0 | 2080.3 / 1009.5 / 3194.9 | 1804.6 / 2102.1 / 3684.8 | 1523.7 / 1618.8 / 2060.4 | 3406.8 / 3688.8 / 5267.6 |

| time, over these runs | median of medians µs | median of p99s µs | median of maxima µs | worst frame µs |
|---|---|---|---|---|
| frame (CPU) | 1346.3 | 2148.9 | 3684.8 | 5625.7 |
| GPU time | 1526.2 | 1628.3 | 1961.0 | 2060.4 |
| frame to idle | 3101.8 | 3746.6 | 5308.6 | 6842.7 |

### Where the margin is

run2-warm's ten slowest frames (each run's report lists its own). The
frames carrying a scripted change are among them but not above the
frames with none, so no one change is the cost:

```
frame 0 at 0.000 s, changes []: to idle 6842.7 µs, CPU 5625.7 µs, GPU 960.2 µs
frame 919 at 14.704 s, changes []: to idle 4423.1 µs, CPU 1866.0 µs, GPU 1525.2 µs
frame 1875 at 30.000 s, changes [click light 3, cube 3 shown, night off]: to idle 4378.5 µs, CPU 2597.3 µs, GPU 1524.5 µs
frame 3455 at 55.280 s, changes []: to idle 4159.4 µs, CPU 2722.5 µs, GPU 1543.8 µs
frame 803 at 12.848 s, changes []: to idle 4132.3 µs, CPU 2497.8 µs, GPU 1550.7 µs
frame 281 at 4.496 s, changes []: to idle 4121.5 µs, CPU 2547.1 µs, GPU 1531.3 µs
frame 2500 at 40.000 s, changes [click light 2, cube 3 shown]: to idle 4035.8 µs, CPU 2386.1 µs, GPU 1556.9 µs
frame 2361 at 37.776 s, changes []: to idle 4005.9 µs, CPU 2382.5 µs, GPU 1540.8 µs
frame 920 at 14.720 s, changes []: to idle 3998.4 µs, CPU 2422.6 µs, GPU 1531.2 µs
frame 533 at 8.528 s, changes []: to idle 3979.6 µs, CPU 2518.7 µs, GPU 1510.7 µs
```

## What this does not show

- The cubes are unlit, flat colours: R1's per-vertex shade is not
  built. GPU time (1.5 ms median) will grow with it.
- No hover: C4's 3D hit test is not built, so the run has no pointer
  over a mesh.
- No decal: the scene extends with one on a cube at S8.
- It is the minimal example, not the racer: the racer's scene
  (`docs/plans/one-scene.md`) is larger and has its own record
  (`scene-run-2026-11-13.md`).
- Wall time on one machine, one quiet window. The instruction-count
  judge of the CPU side across the run (P2) is not built.
