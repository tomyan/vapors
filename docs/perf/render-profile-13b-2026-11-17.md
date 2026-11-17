# Render profile of the racer and one-scene, 2026-11-17 (queue item 13b)

Tom's targets, from L81:
- The racer's frame under 1.5 ms of GPU time at 1 sample and 2.5 ms at
  4 samples on the UHD 630.
- Under 1 ms of CPU time.

This is a profile only: nothing is optimised. It records where the time
goes and recommends an order of slices relative to F8a to F8d
(`docs/plans/flat-scene.md`, "F8's decisions").

## Setup

- **Build.** Release at 2cd269c2, the branch before squashing; the
  report's commit has the same code. Every run prints `2cd269c20b3d`.
- **Machine.**
  - GPU: Intel UHD 630 (CML GT2), Mesa 25.0.7 ANV and llvmpipe, Vulkan.
  - CPU: i5-10500T, 12 threads, `intel_pstate` powersave governor.
  - Headless: an offscreen texture, no present.
- **Runs.** `scripts/render-profile.sh <dir> anv|llvmpipe|draws` under
  `cargo xtask quiet 60`. Each run is `vapors-perf --scene-run <scene>
  --profile-only`, at 1 and 4 samples.
- **What a run is.** The scene's scripted 3,750 frames. The racer is
  racing, with its camera switches. One-scene runs its schedule.
- **Sizes.** The scene's own size, 1100 x 720 for both scenes, and
  1920 x 1080 (`--size`).
- **Data.** Outputs, logs and the per-draw tables are in
  `render-profile-13b-2026-11-17/`. The INTEL_MEASURE CSVs (112 MB) are
  not kept; the script makes them again.

New in this branch, each tested and proven by a break:
- **GPU split.** `GpuReading::surfaces` is the part of a frame's GPU
  time spent in `gpu-surface` renders, from the timer's section pairs.
  The scene report shows it in two rows: "GPU, in gpu-surface renders"
  and "GPU, in the page's passes".
- **Surface CPU phase.** A `surface` phase (`SURFACE_PHASE`) spans a
  surface renderer's `render`, inside `draw`.
- **Phase table.** The scene report has a median and p99 for every CPU
  phase.
- **Flags.** `--size WxH` sets the frame's size. `--profile-only`
  reports with no budget verdict and runs on any adapter, a software
  one included; a busy machine is still refused.
- **Per-draw tables.** `scripts/intel_measure_table.py` turns Mesa's
  `INTEL_MEASURE=draw` capture into a table of each pipeline's GPU time
  per pass, as a mean per frame.

### What qualifies these numbers

- **The GPU clock is at its floor.** The UHD 630 ran at 350 MHz
  (`gt_act_freq_mhz`, sampled every second) through the 1100 x 720
  racer run. It runs at up to 1150 MHz. Under the heaviest load, the
  racer at 1920 x 1080 and 4x, it stayed between 383 and 517 MHz. The
  driver raises the clock only when the GPU is busy for most of the
  time, and these frames leave it idle for most of each 16.7 ms.
  - Every GPU time below is therefore close to the slowest this GPU
    runs.
  - At 1150 MHz, ALU-bound work such as the ground shader would take
    about 0.3 of these times. Clears, copies and resolves are bound
    more by memory and would shrink less.
  - Which clock the targets mean is open (see "Asked").
- **CPU times vary about 2x between quiet runs.** The CPU's clock
  follows the powersave governor.
  - The racer's CPU frame median was 2.37 ms in the matrix run and
    1.19 ms in a run 40 minutes later; its layout was 0.51 and 0.28 ms.
  - On llvmpipe, which keeps every core busy, the same layout took
    0.20 ms.
  - Rank CPU phases against each other within one run, not across runs.
- **The load check.** llvmpipe uses every core, so each llvmpipe run's
  own load fails the end-of-run check (exit 1). Every such run is
  complete (`Frames 3750`). One INTEL_MEASURE run (one-scene, 1x,
  1920 x 1080) ended at a load of 1.82 and exited 1; its capture is
  complete.

## GPU time per run (ANV, median per frame)

| run | GPU time | in the scene's surface | in the page's passes |
|---|---|---|---|
| racer 1x, 1100 x 720 | 5.31 ms | 4.18 ms | 1.12 ms |
| racer 4x, 1100 x 720 | 6.99 ms | 5.20 ms | 1.78 ms |
| racer 1x, 1920 x 1080 | 12.44 ms | 10.30 ms | 2.12 ms |
| racer 4x, 1920 x 1080 | 12.77 ms | 9.82 ms | 2.96 ms |
| one-scene 1x, 1100 x 720 | 1.57 ms | 0.66 ms | 0.90 ms |
| one-scene 4x, 1100 x 720 | 2.89 ms | 1.36 ms | 1.52 ms |
| one-scene 1x, 1920 x 1080 | 3.50 ms | 1.55 ms | 1.96 ms |
| one-scene 4x, 1920 x 1080 | 6.98 ms | 3.32 ms | 3.66 ms |

On llvmpipe the racer takes 6.06 ms at 1x and 13.57 ms at 4x at its own
size, and 13.23 ms and 31.24 ms at 1920 x 1080. One-scene takes
2.32 ms, 9.28 ms, 6.09 ms and 23.25 ms in the same order.

## GPU time per pass and draw kind (ANV, INTEL_MEASURE)

INTEL_MEASURE gives each draw's time, so it also covers clears, resolves
and copies, which the timestamp pairs do not. Its means run about 5%
over the timestamps' medians. The values are means per frame over
3,714 frames: 3,734 for the racer and 3,733 for one-scene, after the
first 20.

The draw kinds are named by their shaders:
- The ground: fragment shader `ecbe00d0`, one indexed quad.
- Meshes: `ae52fe7e`.
- The surface composite: `8b81f945`, the first draw of the page.
- HUD rects: `0c79091a`.
- Glyphs: `88034306`.

### The racer at 1100 x 720

| what | pass | 1x | 4x |
|---|---|---|---|
| ground material, 1 draw | scene | 3.49 ms (62%) | 3.48 ms (47%) |
| MSAA resolve (blit) | scene | none | 0.70 ms |
| MSAA resolve (blit) | page | none | 0.56 ms |
| HUD rects, 12 draws | page | 0.52 ms | 0.52 ms |
| composite of the scene's texture, 1 draw | page | 0.40 ms | 0.42 ms |
| meshes, 9 draws | scene | 0.36 ms | 0.63 ms |
| slow colour clear | scene | 0.29 ms | 0.36 ms |
| slow colour clear | page | 0.29 ms | 0.35 ms |
| glyphs, 12 draws | page | 0.13 ms | 0.14 ms |
| buffer copies, 8.7 | outside passes | 0.11 ms | 0.11 ms |
| depth (HiZ) clear | scene | 0.04 ms | 0.04 ms |
| total | | 5.63 ms | 7.33 ms |

At 1920 x 1080 and 1x:
- The ground is 8.50 ms of 12.23 ms.
- The composite is 0.94 ms.
- Each slow clear is 0.69 ms.
- The HUD rects are 0.48 ms. HUD costs do not grow with the frame,
  because the HUD's boxes keep their CSS size.

At 1920 x 1080 and 4x, the two resolve blits are 1.27 ms and 1.06 ms.

### One-scene at 1100 x 720

| what | pass | 1x | 4x |
|---|---|---|---|
| composite, 1 draw | page | 0.41 ms | 0.42 ms |
| slow colour clear | scene | 0.29 ms | 0.37 ms |
| slow colour clear | page | 0.29 ms | 0.36 ms |
| meshes, 3.5 draws | scene | 0.27 ms | 0.37 ms |
| rects, 4 draws | page | 0.24 ms | 0.24 ms |
| MSAA resolve (blit) | scene / page | none | 0.55 ms / 0.53 ms |
| total | | 1.64 ms | 3.00 ms |

### Notes on the draw kinds

- **The ground is one full-frame draw.** Its fragment shader compiles
  to 331 instructions (SIMD16), with no spills. Per pixel it runs:
  - `atan2` twice;
  - a `sin` of an angle;
  - two `sin`-based hashes;
  - about nine `smoothstep` calls.

  Its pattern does not change; only the camera moves over it.
- **HUD rects.** The rect shader compiles to 381 instructions with a
  loop. Draws 1 and 3 of the page take 91 and 103 µs; the other ten
  take 24 to 52 µs each.
- **Slow clears.** Mesa reports each frame's colour clear of the
  scene's texture and of the frame as a "slow color clear": ANV could
  not use its fast-clear path. Why is not yet found; the clear value
  and the formats' views are the first suspects.
- **Resolve blits.** At 4x, ANV resolves each multisampled target with
  a blit over the whole target.

## CPU time per phase (ANV, median per frame)

From the racer's matrix run at 1100 x 720 and 1x. Mind the run-to-run
spread described above.

| phase | racer | one-scene |
|---|---|---|
| frame (CPU) | 2.37 ms | 1.42 ms |
| update | 0.68 ms | 0.53 ms |
| - layout | 0.51 ms | 0.20 ms |
| - paint | 0.11 ms | 0.04 ms |
| - style | 0.03 ms | 0.02 ms |
| draw | 1.59 ms | 0.85 ms |
| - encode | 0.36 ms | 0.16 ms |
| - submit | 0.33 ms | 0.23 ms |
| - surface (the scene's prepare and record) | 0.17 ms | 0.07 ms |
| - write | 0.14 ms | 0.07 ms |
| - batches | 0.04 ms | 0.02 ms |

- **The rest of draw.** About 0.55 ms of the racer's draw is in none of
  its sub-phases: the painter's own work. That figure is a difference
  of medians, so it is approximate.
- **Two submits.** Today a frame submits twice: once for the surface's
  render and once for the frame.
- **At 4x** the racer's CPU frame is 2.88 ms; each phase grows by about
  the same share.
- **At 1920 x 1080** the CPU times barely change.

## Ranked costs, the racer at 1100 x 720 (GPU at 350 MHz)

1. The ground material: 3.49 ms at 1x and 3.48 ms at 4x; 8.50 ms at
   1080p.
2. The resolve blits at 4x: 0.70 ms for the scene and 0.56 ms for the
   page.
3. The two slow colour clears: 0.58 ms at 1x and 0.71 ms at 4x.
4. HUD rects: 0.52 ms.
5. The composite of the scene's texture: 0.40 ms.
6. Meshes: 0.36 ms at 1x and 0.63 ms at 4x.
7. CPU layout: 0.28 to 0.51 ms per frame, because the HUD's text
   changes every frame.
8. CPU encode and submit: about 0.7 ms together, with two submits per
   frame.
9. Glyphs (0.13 ms), copies (0.11 ms) and the depth clear (0.04 ms).

## Recommended order of slices

The estimates below are this table's arithmetic: each removed item is
counted as costing nothing elsewhere. They are not measurements.

1. **G1, the ground's cost: before F8a2, independent of F8.** This is a
   material change. The material contract is the same in both
   renderers, and F8c only moves materials into the pass, so G1 does
   not wait for F8. Two options to choose between:
   - (a) Cheaper math with the same picture: no `atan2` for the kerb's
     stripes, an integer hash.
   - (b) The static pattern baked into a mipmapped texture at start and
     sampled per pixel, at the cost of a texture's memory and of
     sharpness at grazing angles.

   Either takes off most of the 3.5 ms. The ground at 0.3 ms would
   bring the racer to about 2.4 ms at 1x and 4.2 ms at 4x.
2. **F8a2 as planned.** It takes away:
   - the scene's own pass: its clear (0.29 to 0.36 ms) and its resolve
     at 4x (0.70 ms);
   - the composite (0.40 ms);
   - the surface's own submit, on the CPU.

   With G1 that is about 1.75 ms at 1x and 2.67 ms at 4x.
3. **C1, the remaining slow clear: after F8a2.** F8a2 leaves one clear,
   the frame's. If it becomes a fast clear, about 0.28 to 0.35 ms goes,
   giving about 1.47 ms at 1x and 2.32 ms at 4x. That meets both GPU
   targets at 350 MHz with little margin.
4. **F8b, F8c and F8d in their order.**
   - F8b (instances) lowers the meshes' encode and draw count on the
     CPU more than their GPU time, which is mostly pixels.
   - F8d (4x discard) stores no multisampled colour.
   - F8d should measure whether the page's resolve blit (0.56 ms)
     remains on ANV once the pass resolves into its resolve target.
5. **R1, the HUD rects (0.52 ms): after F8d, for margin.** Find what
   draws 1 and 3 cover and why the rect shader loops.
6. **The CPU slices, after F8a2:**
   - L1, the racer's per-frame layout. It should touch only the
     changed texts.
   - E1, encode and submit after F8a2's single submit, measured again
     first. Most of the 1 ms CPU target sits in these two and in the
     painter's own draw work.
   - Each takes a `perf record` profile at a pinned clock first; this
     run's flat profile was refused by the load.

## Also found

- **The racing workload draws no world.** The full run's `racer,
  racing` workload re-renders the world in no frame after its start.
  INTEL_MEASURE shows only the page pass in each of its frames, so it
  times the HUD alone. The judged scene run (`--scene-run racer`) does
  render the world, and it is what this report uses.
- **Frequency scaling runs inside the measurement.** Comparisons across
  revisions on this box include the GPU's and CPU's frequency choices.

## Asked

- **Which clock do the targets mean?** At the 350 MHz this box runs,
  the targets need G1, F8a2 and C1 together. At a pinned 1150 MHz they
  need much less.
- **Pinning clocks for timed runs.** This would mean
  `gt_min_freq_mhz=gt_max_freq_mhz=1150` and the `performance`
  governor, set by sudo and restored after the run. It changes the
  machine for every lane while it lasts, so it is Tom's or the lead's
  call.
