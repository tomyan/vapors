# The one-scene example's judged run, 2026-11-14 (M3)

The minimal scene for property 9 (`crates/one-scene`): an HTML HUD
over a `gpu-surface`, a 3D scene of a ground plane and three cubes
turning, and CSS reaching the meshes during the run: a click on a HUD
button every 2 s toggles a cube's `lit` class, the page's time signal
hides the third cube in every other 5 s span and switches the ground's
`--tint` token in every other 15 s span. Driven by its own script
(`vapors-perf --scene-run one-scene`, #351), each frame confirmed to
draw every mesh as the schedule says.

- The run is 3,750 frames on a 16 ms frame clock: 60 s of scripted
  time. It is not paced: frames run back to back, each starting when
  the GPU has finished the one before, with no present, so a run takes
  about 12 s of wall time. What a display's pacing would bring (idle
  gaps between frames, GPU clock ramps, heat) is not in it.
- Every frame is judged from the first, with no warmup (D7 (a)).
- A frame is over budget when its time from the start of `App::frame`
  to the return of the GPU wait after it is over 16.7 ms (D6 (a)).

## Setup

- Release build at 7bcb59a, which holds #351 with its review round
  and this record's script. Every run prints
  `7bcb59a2a384`, with no `-dirty` suffix. After the runs, #351 and this record were
  replayed onto main (after #344's squash-merge) with no change of
  their own; main's commits since 7bcb59a's base (the elements layout
  lane's flex work and vapors-shapes' tessellation) are in this
  branch's head and not in the runs. The binary's sha256 is in
  `one-scene-run-2026-11-14/sha256.txt`.
- The dev machine: Intel UHD 630 (CML GT2), Mesa 25.0.7 ANV, Vulkan,
  headless, rendering to an offscreen texture with no present. Scale
  1, 1100 x 720.
- Command, the whole window in one line:
  `cargo xtask quiet 30 -- scripts/scene-runs.sh docs/perf/one-scene-run-2026-11-14 one-scene`.
  The quiet lock holds every xtask ci, test and gate run. The script
  then waited 521 s for the 1-minute load to fall under 1.5 before the
  first run (`settled after` in the log). Three pairs, each a cold run
  on a new, empty `MESA_SHADER_CACHE_DIR` then a warm run on the cache
  it filled. Each run's output is kept verbatim in
  `one-scene-run-2026-11-14/`; the tables are
  `scripts/scene-run-table.sh` on those files.
- The load: the 1-minute average was 1.04 to 1.38 across the runs,
  against the runs' refusal at 1.5. The 5- and 15-minute averages were
  11.37 and 26.34 at the start and still 9.18 and 24.49 at the end:
  the machine had been busy in the quarter hour before (a gate had
  just ended), and those averages had not caught up. What was running
  during the runs is not known: the logged `ps` figures are each
  process's average over its lifetime, not its use at that moment.

```
settled after 521 s, load 1.35 11.37 26.34; busiest:
%CPU     ELAPSED COMMAND
 100       00:00 ps
15.6  9-21:44:25 claude
 2.6       20:05 kworker/u48:5-flush-259:0
 2.4  2-01:37:29 tailscaled
 1.5    04:05:43 kworker/u48:3-kvfree_rcu_reclaim
run1-cold start 08:38:01 load 1.35 11.37 26.34
run1-cold end 08:38:13 exit 0 load 1.38 11.04 26.07
run1-warm start 08:38:13 load 1.38 11.04 26.07
run1-warm end 08:38:24 exit 0 load 1.28 10.55 25.67
run2-cold start 08:38:24 load 1.28 10.55 25.67
run2-cold end 08:38:36 exit 0 load 1.16 10.22 25.40
run2-warm start 08:38:36 load 1.16 10.22 25.40
run2-warm end 08:38:47 exit 0 load 1.06 9.90 25.13
run3-cold start 08:38:47 load 1.06 9.90 25.13
run3-cold end 08:38:59 exit 0 load 1.13 9.48 24.75
run3-warm start 08:38:59 load 1.13 9.48 24.75
run3-warm end 08:39:11 exit 0 load 1.04 9.18 24.49
```

## Result

Every run exits 0: no frame over budget in any of the six, 22,500
frames judged, each also confirmed to draw the scheduled colours and
visibility. The slowest frame of all is run1-cold's frame 0, 7.6 ms
to idle (6.4 ms of CPU, against 2.0 to 2.2 ms for the other five frame
0s; cause not known, and not repeated in the other runs). After frame
0, no frame in any run took more than 5.5 ms to idle. The start-up
(`App::start`, before frame 0, not judged) took 69 to 75 ms cold and
13 to 14 ms warm.

### Cold: a new, empty shader cache

Revision 7bcb59a2a384.

| run | cache | over budget | frame 0 CPU / GPU / to idle µs | frame (CPU) median / p99 / max µs | GPU time median / p99 / max µs | frame to idle median / p99 / max µs |
|---|---|---|---|---|---|---|
| run1-cold | cold | 0 | 6379.7 / 955.0 / 7602.0 | 1699.4 / 2230.8 / 6379.7 | 1526.4 / 1701.8 / 1963.2 | 3332.6 / 3841.4 / 7602.0 |
| run2-cold | cold | 0 | 1985.8 / 999.4 / 3120.3 | 1223.5 / 2138.7 / 3070.5 | 1528.8 / 1729.7 / 2065.6 | 3011.6 / 3762.9 / 4524.4 |
| run3-cold | cold | 0 | 1991.0 / 966.3 / 3042.5 | 1261.2 / 2252.6 / 3600.2 | 1527.6 / 1674.0 / 1947.6 | 3048.3 / 3853.6 / 5077.6 |

| time, over these runs | median of medians µs | median of p99s µs | median of maxima µs | worst frame µs |
|---|---|---|---|---|
| frame (CPU) | 1261.2 | 2230.8 | 3600.2 | 6379.7 |
| GPU time | 1527.6 | 1701.8 | 1963.2 | 2065.6 |
| frame to idle | 3048.3 | 3841.4 | 5077.6 | 7602.0 |

### Warm: the cache the cold run filled

Revision 7bcb59a2a384.

| run | cache | over budget | frame 0 CPU / GPU / to idle µs | frame (CPU) median / p99 / max µs | GPU time median / p99 / max µs | frame to idle median / p99 / max µs |
|---|---|---|---|---|---|---|
| run1-warm | warm | 0 | 2221.9 / 984.2 / 3281.3 | 1414.3 / 2186.2 / 3833.0 | 1527.2 / 1681.7 / 2158.7 | 3166.8 / 3803.4 / 5501.2 |
| run2-warm | warm | 0 | 2153.0 / 948.8 / 3302.9 | 966.2 / 2230.6 / 3810.8 | 1527.5 / 1713.6 / 1986.9 | 2822.7 / 3831.5 / 5359.6 |
| run3-warm | warm | 0 | 2104.4 / 932.4 / 3110.0 | 1442.7 / 2121.4 / 3674.0 | 1527.2 / 1645.3 / 1948.9 | 3212.2 / 3757.3 / 5153.1 |

| time, over these runs | median of medians µs | median of p99s µs | median of maxima µs | worst frame µs |
|---|---|---|---|---|
| frame (CPU) | 1414.3 | 2186.2 | 3810.8 | 3833.0 |
| GPU time | 1527.2 | 1681.7 | 1986.9 | 2158.7 |
| frame to idle | 3166.8 | 3803.4 | 5359.6 | 5501.2 |

### The slowest frame after frame 0, in each run

Each run's report lists its ten slowest frames with their lines. The
slowest after frame 0:

| run | frame | at | scripted changes | to idle µs |
|---|---|---|---|---|
| run1-cold | 2813 | 45.008 s | cube 3 hidden, night on | 5429.1 |
| run1-warm | 2825 | 45.200 s | none | 5501.2 |
| run2-cold | 2178 | 34.848 s | none | 4524.4 |
| run2-warm | 2813 | 45.008 s | cube 3 hidden, night on | 5359.6 |
| run3-cold | 1875 | 30.000 s | click light 3, cube 3 shown, night off | 5077.6 |
| run3-warm | 1875 | 30.000 s | click light 3, cube 3 shown, night off | 5153.1 |

In four of the six runs it is a frame with two or three scripted
changes (2813 or 1875); in the other two it carries none.

## What this does not show

- The cubes are unlit, flat colours: R1's per-vertex shade is not
  built. GPU time (1.5 ms median) is expected to grow with it; not
  measured.
- No hover: C4's 3D hit test is not built, so the run has no pointer
  over a mesh.
- No decal: the scene extends with one on a cube at S8.
- It is the minimal example, not the racer: the racer's scene
  (`docs/plans/one-scene.md`) is larger and has its own record
  (`scene-run-2026-11-13.md`).
- Wall time on one machine, one window. The instruction-count judge of
  the CPU side across the run (P2) is not built.
