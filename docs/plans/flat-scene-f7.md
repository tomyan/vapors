# Flat scene F7: the MSAA decision

Design note for slice F7 of `flat-scene.md` (section 8; antialiasing in
4.1 and 4.2; decision D2 in section 9). Status: the plan is agreed (the
lead accepted all eight recommendations below, L12, 2026-11-17); no
code yet.

The F7 row: the pass's sample count from a page flag; every 2D pipeline
in both counts; a resolve into the target; `GpuTimer` A/B on the rows
page, W10 and the racer page at 1 and 4 samples on the UHD 630 and the
Pixel 9 Pro; a tilted cube's edge compared with an 8x reference at 1
and 4 samples. Acceptance: the numbers in `docs/perf/`, D2 answered
with them, pixels unchanged at 1 sample.

D2 as decided (Tom, 2026-11-13): (a) MSAA 4x for the whole pass on
pages with a mesh pipeline, subject to F7's number. If 4x costs more
than a tenth of 16.7 ms on the UHD 630, use (b), per-scene multisampled
targets, on desktop and (a) on tilers, chosen per adapter and reported.

## Today

- **One sample everywhere.** Every pipeline is built with
  `MultisampleState::default()`, which is one sample: the instanced
  pipelines (`renderer/src/quads_setup.rs`), meshes
  (`meshes_setup.rs`), images (`image_setup.rs`), `solid.rs`, the scene
  pass's pipelines (`projective.rs`), text (`text/src/pipeline_gpu.rs`),
  `scene3d-gpu/src/pipeline.rs` and the racer's own
  (`racer/src/gpu/pipeline.rs`).
- **The pass.** `OpenPass::begin` (`renderer/src/open_pass.rs`) has one
  colour attachment, the target's view, no resolve target and no depth.
  It stores the view. `ScenePainter::draw` begins it with a clear;
  `draw_over` loads what the target holds and draws over it.
  Consecutive draws on one view share the pass. A frame can open more
  than one pass, one per view: `gpu-surface` textures
  (`app/src/painter_surface_texture.rs`) and the frame clock's
  (`frame_clock.rs`), and `FrameEncoder::flush` ends the open pass
  mid-frame (at its pass budget, or for a repacked atlas page), so the
  next draw on the view begins another. Every vapors-perf workload drew
  one pass a frame when last counted (`docs/perf/baseline-2026-11-10.md`,
  the harness at 03c4514); the racer's track renders in its
  `gpu-surface`'s own encoder, which that count leaves out.
- **No registry and no `PaintUses`.** `ScenePainter::new(device,
  format)` builds each pipeline once. `PaintUses` arrives with F10.
- **3D is drawn outside the painter's pass until F8.** A scene renders
  into its own offscreen texture (`scene3d-gpu/src/draw.rs`), and the
  racer renders through `gpu-surface` into its own texture; the painter
  composites both as images. So a 4x painter pass, before F8, puts
  samples on 2D content only and antialiases no 3D edge.
- **2D meshes do not need MSAA.** Since S3a3, the mesh pipeline computes
  each pixel's coverage itself from the outline's corners (the
  exact-corner method). In S3b (`docs/perf/s3b-antialiasing-2026-11-14.md`),
  MSAA 4x passed the same 39 of 40 icon cells as that method, and
  failed the 0.25 px stroke and circle. MSAA is for 3D meshes, which
  have no outline to measure.
- **An MSAA path already exists in a measurement.**
  `elements-corpus/examples/aa_methods` (`offscreen.rs`, `msaa.rs`)
  draws into its own 4x target and resolves it.

## The plan

1. **The flag.** A page is 4x when it contains a `<Scene>`, and 1x
   otherwise. A page with only 2D meshes stays 1x (Q1). This is our
   reading of D2, not its wording: D2 (a) turns 4x on for "pages whose
   table names a mesh pipeline" and keeps 2D coverage analytic, and
   4.2's rule is `uses.scenes || uses.pipelines has a mesh pipeline`.
   Since S3a3, 2D meshes have exact coverage without MSAA, so we read
   the mesh pipeline D2 means as the 3D one and drop 4.2's second
   condition. `flat-scene.md` 4.2 and the F7 row now say so.
2. **One count a painter.** `ScenePainter::new` takes the sample count,
   and passes it to every pipeline's constructor. A page needing the
   other count gets a new painter, as a format change already does.
   That replaces "both counts in the registry", since there is no
   registry (Q2).
3. **The pass at 4x.** The painter's pass draws into a 4x colour texture
   and resolves into the target's view. `draw` clears the 4x texture. A
   later pass on the same view in the same frame (another view's pass
   between, or a flush) loads it, so the texture is stored at the end
   of every pass, and every pass resolves (Q3).
   - **Owner and key.** The painter owns its 4x textures, one for each
     target view it draws to, at the viewport's size (not the texture's,
     so a view of a smaller mip level gets a texture of that level's
     size). Keyed by size alone, two views of one size would share one,
     and a pass loading it would resolve the other view's frame (#472's
     review, M1).
   - **Lifetime.** A texture is made on the first draw to its view,
     kept while each frame draws to that view, and dropped after a frame
     that does not, the rule kept bytes and table entries already
     follow. Each `draw` is a frame, so a painter drawing two targets in
     an app frame remakes each one's texture on each call, and a tooltip
     on a view after another view's `draw` meets a fresh texture, which
     F7a1's uncleared-load check refuses. A resize is a new size: the old
     texture goes after the first frame at the new size.
   - **Where.** `Target` gains the 4x view beside the target's view
     (`None` at 1x). `FrameEncoder::begin` passes both to
     `OpenPass::begin`, which attaches the 4x view with the target's
     view as `resolve_target` (today `None`, `open_pass.rs`), and
     `OpenPass::is_on` still compares the target's view.
   - **Not multisampled:** `gpu-surface` textures, which application
     renderers draw with their own pipelines, and the frame clock's
     pass.
   `draw_over`
   on a 4x painter is refused, naming `draw_over` and the sample count:
   the 4x texture holds nothing to draw over, and loading the view into
   it would need a copy pass (Q4).
4. **Timing.** A script runs each page at 1 and 4 samples and reads
   `GpuTimer`: the rows page, W10 and the racer page. W10 is added by
   F3, which F7 follows; F7b stops and reports if it is not on main. For
   the racer, the painter's pass is timed at 4x. The racer's own 3D
   pass is a different renderer and stays at 1x, so 3D at 4x is
   measured on the one-scene example instead (Q5). The UHD 630 is run
   by the script; the Pixel 9 Pro is run by hand from a served build
   that has a sample-count switch, debug only.
5. **The cube edge.** A tilted cube drawn through the scene pass's mesh
   pipeline (`ProjectiveMeshes`) into an offscreen target, at 1 and 4
   samples, against a reference drawn at 8 times the size and averaged
   down (64 samples a pixel). WebGPU guarantees only 1 and 4 samples, so
   8x MSAA cannot be the reference on every adapter (Q6). Recorded: the
   largest and the mean difference over the edge's pixels.
6. **Pixels.** At 1 sample, every existing pixel test runs unchanged. At
   4 samples, 2D pixels can differ slightly: a fragment is shaded only
   for the samples its triangle covers, so a pixel on a quad's or
   triangle's geometry edge takes a fraction of its alpha. Those edges
   lie where the analytic coverage is near 0, so the difference should
   be small. The pixel tests' pages are drawn at 4x and the largest
   difference from 1x is recorded; acceptance takes that measured
   difference as the bound, and the constant in the test cites the
   measurement (Q7).
7. **Memory.** The 4x colour texture is 16 bytes a pixel: 12.7 MB at
   1100 x 720, and 41.5 MB on the Pixel 9 Pro at 1080 x 2400. That is
   colour alone; 4.1's estimate also counts a 4x depth buffer, which F8
   adds. It is
   recorded beside the times. A memoryless attachment on tilers needs
   the texture discarded at the end of the pass, so it waits for F8,
   where the frame has one pass.
8. **D2's answer.** The report gives D2's verdict with the numbers. If
   4x is over 1.67 ms on the UHD 630, building (b) per adapter is a
   separate slice, not part of F7 (Q8).

## Slices

- **F7a, the count.** The painter takes a sample count. The 4x pass
  resolves into the view, and `draw_over` is refused at 4x. Tests: 1x
  pixels unchanged (the existing suite); a page drawn at 4x within Q7's
  bound of 1x; a frame of two passes on one view at 4x matching the
  same frame at 1x within that bound; the `draw_over` refusal proven by
  breaking it.
- **F7b, the numbers.** The timing script; the debug-only sample-count
  switch on the served page, for the Pixel 9 Pro run by hand; the cube
  fixture against the reference; a report in `docs/perf/`; and D2
  answered in `flat-scene.md`.

## Questions, answered

The lead accepted all eight recommendations as written (L12,
2026-11-17). Q1 is recorded in the plan (item 1 above, `flat-scene.md`
4.2 and the F7 row) as our reading of D2, so a reviewer can contest
it. On Q4, touche does not call `draw_over`
either (the lead checked).

1. **Flag: scenes only, not 2D meshes?** 4.2's rule turns 4x on for
   scenes or a mesh pipeline. Since S3a3, 2D meshes get exact coverage without
   MSAA. I recommend `scenes` alone, so a page of SVG icons stays at 1x.
2. **A painter per count, rather than a registry?** No registry exists.
   A painter built for one count matches "fixed per page" in 4.2 with no
   new structure. I recommend it.
3. **Store and resolve at the end of every pass at 4x?** That is
   correct for frames with several passes on one view, and costs the
   store and a resolve per pass. Every vapors-perf workload had one pass
   a frame at the last count (`baseline-2026-11-10.md`), so the measured
   cost is one store and one resolve. F8 pins
   one pass and can switch to discard. I recommend storing until F8.
4. **Refuse `draw_over` at 4x?** The other option is a copy pass that
   loads the view into the 4x texture first. Nothing in vapors calls
   `draw_over` outside tests, and a caller that draws over its own
   content can stay 1x. I recommend the refusal.
5. **How to time 3D at 4x before F8?** The racer's 3D pass is its own
   renderer (`racer/src/gpu/pipeline.rs`) until one-scene R3a2. I
   propose timing the painter's pass at 4x on the racer page, and the
   scene pass at 4x on the one-scene example (`crates/one-scene`, the
   page of the 60 s run), whose scene draws through scene3d-gpu's
   `SceneRenderer`. F8 re-measures in one pass.
6. **Reference: 8x size averaged down, rather than 8x MSAA?** It is
   portable and has more samples. 8x MSAA would work on ANV and
   lavapipe, but WebGPU does not guarantee it. I recommend the
   supersampled reference.
7. **The bound on 4x 2D pixels?** I propose measuring first, on the
   existing pixel tests' pages at 4x against 1x, and then pinning the
   measured largest difference as the bound in F7a's test. The other
   option is to require them identical, which is likely to fail at
   geometry edges.
8. **(b) per adapter: in F7, or a slice of its own if needed?** I
   recommend a slice of its own, built only if the UHD 630's number is
   over the threshold.

## As built: F7a

In two pull requests, the renderer half (F7a1) and the painter half
(F7a2), to keep each near 400 lines.

- **F7a1.** `PassFormat` (a format and `Samples::One` or `Four`) is what
  every 2D pipeline is made for; a format alone is one sample.
  `Target::multisampled` names the 4x view a pass attaches; the pass
  resolves into the target's view and stores the multisampled colour.
- **F7a2.** `ScenePainter::new` takes a `PassFormat`. At 4 samples the
  painter owns its 4x textures (`painter_multisampled.rs`), keyed and
  dropped as plan item 3 says, and attaches them to `draw`'s and
  `draw_tooltip`'s targets; `draw_over` is refused by name. A target
  that already names a multisampled view is refused.
- **The two-pass case is tested in the renderer.** A painter's frame
  splits its pass on one view only at `FrameEncoder::flush` (its budget,
  or a repacked atlas page): `gpu-surface` textures are drawn in their
  own encoders, on their own views. Both cases go through the same
  `FrameEncoder` and `OpenPass` that F7a1's
  `gpu_multisampled_pass.rs` drives across a flush, so no painter-level
  test repeats it.
- **Q7's measurement** (2026-11-17, ANV on the UHD 630 and llvmpipe):
  every demo with a `.pixels` file, at every viewport, draws the same
  pixels at 4 samples as at 1 (quads and text on whole pixels cover
  every sample). An svg of a triangle (a mesh) and a circle (a closed
  form) off the pixel grid differs on 51 pixels, by at most 29 of 255,
  on both adapters. `gpu_samples.rs` pins both: the demos identical,
  the shapes within (29, 51) and differing somewhere, so a painter
  that fell back to 1 sample fails.
