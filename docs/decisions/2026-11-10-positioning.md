# Positioning: DX and AX, efficiency, portability

Date: 2026-11-10. Decided by: Tom.

## Decision

vapors' differentiated capability is three things together (Tom,
2026-11-10):

1. **DX and AX**: the developer's and the agent's experience. "I think the
   DX/AX combined with the (eventually) best in class efficiency is the
   differentiated capability."
2. **Efficiency, eventually best in class.**
3. **Portability**: "I think the third pillar is the portability."

Later choices are tested against these three. A claim of "best in class"
is made only with measurements against named competitors on the same
workloads (docs/briefs/INCREMENTAL_UI_PERFORMANCE_BRIEF.md says the same).

## Where each pillar stands (2026-11-10)

**DX and AX**
- Has: HTML and CSS semantics, so web knowledge applies; `.wisp`
  components compiled to Rust; refusals that name the element, the
  attribute and the fix; layout, paint and accessibility checked against
  Chromium by oracle fixtures; the accessibility tree as the driving
  interface (touche R1: find by role and name, press, type).
- Missing: a documented drive API in vapors for any app (touche's R2 to R6
  are touche's); refusals and errors as structured data an agent can act
  on; docs and examples for someone outside touche; a fast feedback tier;
  a wasm test tier; CSS a web developer meets early (`transform`,
  `opacity`, `filter`, `order`; tooltips are in progress).

**Efficiency**
- Has: equal writes skipped (#146); structural update counters (#157) and
  GPU counters (#159); a correctness oracle comparing incremental and full
  updates (#153); atlas region uploads (#160: the stress demo's first frame
  44 MB to 1 MB); draw on demand with no idle frames (#155, #167).
- Missing: every update still walks the whole tree (at 1000 rows an idle
  update visits 4,002 elements and builds and paints 7,002 boxes); no
  retained box tree or paint (IU3, IU4); no release timing baseline
  (IU1 PR 4); no comparison with competitors.

**Portability**
- Has: one codebase drawn natively and on the web from one GPU renderer,
  with no webview; Linux native (lavapipe) and web (Chromium WebGPU) in CI;
  touche's desktop app and page share their controls and top bar; Cmd on a
  Mac in the browser.
- Missing: macOS and Windows native in CI; Firefox and Safari WebGPU in any
  check; native mobile untested; no defined behaviour where WebGPU is
  absent (it should be a loud refusal naming what is missing); a written
  statement of supported platforms and how each is proven.

## Direction (captured, not scheduled)

Tom: "Don't need to do everything at once, but do capture." These are
candidates for the plan, ordered by the lead's current view; each is
scheduled through the plan when its turn comes.

1. Efficiency: IU1 PR 4 (release timing baseline, on a quiet machine),
   then IU3 (retained box tree) and IU4 (retained paint), then a
   comparative benchmark on the same workloads against Blitz and one or
   two others.
2. Portability: a supported-platforms statement; Windows and macOS native
   and Firefox and Safari WebGPU in CI (factory changes, Tom's clear); a
   loud refusal where WebGPU is missing.
3. AX: lift the drive API into vapors as a documented, agent-facing
   surface; structured refusals; touche's R2 to R6 on top.
4. DX: docs and examples for an outside developer; the fast tier; the
   wasm test tier.

## Context

The competitors considered on 2026-11-10: Blitz (Dioxus Labs: HTML and CSS
drawn natively with Stylo, Taffy, Parley and Vello), Tauri and Electron
(a browser in a native shell), Flutter and Slint (their own renderer and
language), GPUI, Iced, egui and Xilem (Rust GPU UI with a Rust API), and
Leptos and Sycamore (compile-time templates on the web DOM). The
comparison is in the lead's session notes and is a starting point, not a
measurement.
