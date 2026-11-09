# Homepage racer

Authorised by Tom: build here in an isolated checkout; camera zooms out at higher speed. One car, one circuit, three timed laps, keyboard and touch driving, compiled Vapors HUD and tuning controls, pause/restart, stop simulation/rendering when paused or unfocused. Replace the homepage image-viewer embed after validation. Keep the image viewer available separately.

## Implementation

Use the existing platform App, DocumentInput, ScenePainter::draw_over and .wisp compiler. A racer crate owns simulation and a GPU track/car pass; compiled components own HUD and settings. No changes to active touché/Vapors checkouts. Scope: arcade driving, not a physics simulator. Steering and grip are adjustable. Camera stays north-up and follows with speed-dependent lookahead and smooth zoom.

## Slices and acceptance

1. Deterministic driving model and speed-dependent camera. Red tests: acceleration/braking, grass drag, bounded timestep/input, camera widening and smoothing, ordered laps and no shortcuts. Green then reassess.
2. GPU track/car plus compiled HUD. Validate actual native and WebGPU renders, restart/pause and tuning. No screenshots or JavaScript substitutes for the running Rust app.
3. Keyboard/touch and focus. Validate held inputs, simultaneous touch controls, cancellation, pause on blur and no idle frames. Camera must widen as acceleration increases speed.
4. Homepage integration. Fix embed URL, describe real controls, keep touch usable at 320px, visible startup failure. Build/check site, verify deployed embed, save evidence.

Each slice uses red/green tests, relevant defect proofs, refactoring and commits. Framework branch uses PR review and CI before merge; deployment can serve the reviewed local build without modifying origin/main. Reassess if a necessary API is missing rather than changing architecture silently.

## Numeric controls boundary

Select-all is being implemented by the other coding agent. No number-field implementation changes belong in this branch. Browser checks use the existing Tab-to-select behaviour. Racer-level validation refuses invalid tuning before resuming, with fields left available for correction.

## Validation completed

- 11 racer tests cover physics, camera, invalid input, compiled desktop/phone layout, independent touch input, pause/reset, tuning refusal and three-lap progression.
- Camera and lap checks failed under deliberate defects and passed after restoration. Invalid-tuning guard was tested red before implementation.
- Browser checks pass: driving/steering, camera widening, idle while paused, typed tuning via Tab selection, real focus transfer, simultaneous touch input with independent release and cancellation at 390px and 320px.
- Full `cargo xtask ci` completed all 11 steps, including native/wasm lint, unit and GPU tests. Log: `/tmp/vapors-racer-ci.log`.
- Browser evidence: `/home/tom/.cache/vapors-racer-evidence/browser.json` and screenshots beside it. Hardware touch/stylus and non-Chromium WebGPU remain untested.
- Next slice: website embed, startup error and deployment validation.

## Android viewport and slow-frame regression (2026-11-09)

- Reproduced clipped controls by shrinking the visual viewport independently of
  the layout viewport. The browser regression failed before the host canvas fix.
- Size the racer canvas from the visual viewport and update it when browser bars
  change its height. The compiled GPU controls continue to use the canvas size.
- Reproduced unwanted pause with a 500 ms frame in an app test. Cap simulation
  catch-up at six steps without clearing held controls; focus loss still pauses.
- Validate with `check-racer-android.mjs` (viewport changes and 400 ms browser stall),
  the existing browser interaction suite, racer tests, and clippy.

## FPS readout (2026-11-09)

Show wall-clock frame rate beside the race stats, averaged over half a second.
Include slow frames before simulation catch-up is capped. Show a dash while
paused rather than retaining a stale rate. Validate the calculation and pause
reset with a test proved red first, then check mobile HUD bounds in Chromium.

## Tilted 3D view and phone landscape (2026-11-09)

Approved: real 3D geometry with a tilted overhead camera, a closer view that
still widens at speed, and a compact landscape HUD.

1. Short viewports: one stats row, no branding/keyboard hints, status only while
   paused, full-size driving buttons. Validate visible bounds and no overlap.
2. Geometry: solid car, trees and pit building, with a shared tilted projection
   and depth buffer. Keep existing planar driving physics. Validate mesh height,
   GPU shader compilation, occlusion screenshots and existing driving tests.
3. Publish after mobile portrait/landscape and slow-frame regressions pass.

Trees remain scenery; this does not add collision physics or crash recovery.

Validation: the short-screen stats test failed on the previous HUD. Flattening
all mesh Z coordinates deliberately failed the solid-height test; restored
geometry passes. Browser checks cover 740×250, 844×300 and 640×260 landscapes,
Android visual-viewport resizing, a 400 ms stall, desktop driving, tuning and
multi-touch. Screenshots confirm solid sides and the slim landscape stats row.
The orthographic camera is tilted about 35 degrees from vertical; the view scale
is 1.4 times the previous scale. Physical Android GPU stability remains unverified.

## Fixed / Follow camera (2026-11-09)

Approved: a Camera: Fixed / Follow toggle. Fixed retains world orientation;
Follow turns the car toward screen-up, moves the view ahead, and sits closer
and lower. Blend position offset, rotation, tilt and zoom; retain speed zoom.

1. Camera model: shortest-path rotation, smooth mode transitions, validated
   inputs. Prove tests fail before implementation, including heading wrap.
2. Connect the same camera transform to ground and solid geometry; expose the
   mode through a compiled HUD button. Permit transitions while paused, then
   return to idle rendering. Check phone control bounds and both camera modes.
3. Run browser driving/viewport regressions and publish the verified build.

Validation: 19 racer tests and clippy pass. Camera-mode tests were red before
implementation; removing shortest-angle wrapping made the heading-wrap test
fail, then restoration passed. Chromium checks cover switching while driving,
switching while paused and settling back to idle, plus 320 px portrait and
740×250 landscape. Existing driving, tuning, focus-loss, touch, viewport resize
and slow-frame regressions pass. Ordinary pause freezes the camera; an explicit
camera toggle while paused runs only its transition.

## Component-driven HUD (2026-11-09)

Approved scope: extract the HUD into compiled RaceStats, RaceMenu,
CameraToggle, DrivingControls and CarSetup components. Add a reusable HoldButton
using existing pointer events/capture; remove racer-specific pointer hit-testing.
Keep rendering, physics, layout and camera behavior unchanged.

1. HoldButton: independent held pointers, release/cancellation/capture loss,
   keyboard activation, reset/disable/unmount. Prove behavior tests red first.
2. Compose the HUD with shared signal props and scoped component styles. Route
   platform input through DocumentInput; combine held signals with keyboard input.
3. Validate mounted components and real touch capture, including moving outside,
   releasing one finger, cancellation and focus loss. Run existing camera,
   viewport and driving checks, then publish the verified build.

Validation: all 11 repository CI steps pass, including GPU tests. The racer has
21 tests; HoldButton has six new behavior tests, alongside the existing component
suite. The initial hold tests failed before implementation. Removing explicit
capture failed the mounted-document regression; resetting on unchanged signals
failed the repeated-state regression. Both mutations were restored and tests pass.
Browser tests pass for multi-touch moving outside the buttons, independent
release, cancellation, focus loss, tuning, camera switching, Android viewport
changes, slow frames and short landscapes.

The component uses aria-disabled with input guards because dynamic HTML boolean
attribute removal is unsupported in this checkout. External component props
produce the compiler's existing diagnostic; generated Rust checks their types.
