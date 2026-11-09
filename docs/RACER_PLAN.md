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
