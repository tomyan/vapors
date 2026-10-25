# Questions for Tom

Status: 2026-10-25. API-taste and product decisions that are Tom's,
from the area documents. Ordered by how much they block: the first
block the edit UI's first slices; the last can wait months. Each has
the options, the trade-offs and a recommendation.

Decided 2026-10-25 (Tom): Q1 (c), Q2 (b), Q3 (a); Q4 to Q27 as
recommended. Each question carries its decision below; the area
documents carry the wording that changed (areas 1, 3 and 6).

## Blocking the edit UI

**Q1. How does touché style a control's parts (the slider's thumb and
track, the checkbox's mark)?** Area 3 E3.
- (a) Prefixed custom properties the UA sheet reads:
  `--vapors-thumb-size`, `--vapors-thumb-color`, `--vapors-track-color`.
  Tokens and themes apply; no new selector machinery; the vocabulary is
  a short list.
- (b) `::part(thumb)` pseudo-elements, as the web's shadow parts.
  Arbitrary styling; needs pseudo-element support in `vapors-css`, which
  refuses every pseudo-element today, and parts have no Chromium oracle
  for a native `input`.
- (c) Both, (a) first.
- Recommendation: (c). (a) covers the design system's tokens for the MVP;
  (b) when a pattern needs a shape (a) cannot say.
- Decided 2026-10-25 (Tom): (c).

**Q2. Should a value control animate its display when the app sets its
value?** Area 6 A2; this is what "agent edits animate the controls"
means for a slider.
- (a) Always, as UA behaviour with a default duration. No app code; a
  deviation from the web, where a programmatic set jumps.
- (b) Opt-in by a prefixed CSS longhand on the input,
  `--vapors-value-transition: 300ms ease`, so tokens set it and reduced
  motion applies. Default off, so a fixture with Chromium still matches.
- (c) Never in the framework; touché animates by writing intermediate
  values from a frame task. Costs a signal write per frame per control.
- Recommendation: (b).
- Decided 2026-10-25 (Tom): (b).

**Q3. When a signal change restarts an async derivation, what is held?**
Area 1 rule 2; ARCHITECTURE conflict 1.
- (a) Narrow: the async derivation and its downstream. A dragged slider
  moves at once; a readout derived from the render changes when it
  lands; consistency between a sync and an async value is the author's
  job (derive both from the async value).
- (b) Svelte's rule: every update caused by the write is held until the
  async work settles. Nothing can disagree; a control under the finger
  waits for the GPU.
- (c) (b), with a per-boundary opt-out attribute for direct-manipulation
  regions. Two rules to learn.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q4. Snippets and children: Svelte's syntax?** Area 1 R1.
- (a) `{#snippet name(args)}...{/snippet}` and `{@render name(args)}`;
  children inside a component tag are the `children` snippet. Matches
  the rest of the template syntax, which is Svelte's; Svelte's tests
  port.
- (b) sumi's slots (`<slot>`), refused today with a pointer to snippets.
- (c) Rust closures returning a built fragment, no template syntax.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q5. What is the unit of off-thread work?** Area 1 rule 4.
- (a) A named `#[vapors::offload] fn` with serialisable owned input; the
  same call natively and on the web.
- (b) A closure; works natively, cannot cross a web worker, so the web
  would need a second API.
- (c) (a) on the web, (b) allowed natively. Two shapes for one idea.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q6. Reads of a signal after an `await` in a tracked scope.** Area 1.
- (a) Refused by the compiler when it can see them; read before the
  `await`. Timing-independent restarts.
- (b) Allowed and untracked, silently. A hidden non-dependency.
- (c) Tracked, as Svelte does. Restart depends on timing.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

## Blocking the next framework slices

**Q7. Enter and leave animations: the web's mechanism or directives?**
Area 6 A6.
- (a) `@starting-style` and `transition-behavior: allow-discrete`, with
  the element kept until its transitions end. Chromium is the oracle.
- (b) Svelte's `in:`/`out:`/`transition:` directives with built-in
  `fade`, `fly`, `slide`. Familiar; no oracle; a second animation system.
- Recommendation: (a). FLIP stays a directive (`animate:flip`) because it
  has no CSS equivalent.
- Decided 2026-10-25 (Tom): (a).

**Q8. Accessibility on the web: a DOM mirror?** Area 5 A3.
- (a) A hidden, focusable DOM subtree beside the canvas, driven by the
  same tree diff as the native adapters (Flutter web's approach).
  TalkBack on the Pixel works; two focus states kept in step.
- (b) None on the web until a native phone build exists. Fails MVP
  acceptance step 6 on the phone.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q9. Web IME for the MVP: Chromium's EditContext only?** Area 3 E7.
- (a) EditContext first. Chrome on Android is the phone build and
  Chromium is the desktop oracle; Safari and Firefox get the hidden
  textarea in a later slice and the status line says so.
- (b) Hidden textarea first, everywhere. Works in every browser; known
  gaps in composition and caret placement.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q10. Where do remote functions live?** Area 8 F2.
- (a) `#[vapors::query]`/`#[vapors::command]` on async functions in
  `*.remote.rs` files; the build splits client and server from the file
  name, as SvelteKit does from `.remote.ts`.
- (b) The same attributes anywhere in a crate, split by `cfg` features.
  Harder to see what runs where.
- (c) A `<script server>` section in `.wisp`. Mixes server code into a
  component file; a component is not the unit of an API.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q11. Remote transport encoding.** Area 8.
- (a) JSON with `schemars` validation; readable in a browser's network
  panel; the stand-in server is a few lines.
- (b) CBOR; smaller; opaque.
- Recommendation: (a). Recordings (area 10) use CBOR because they are
  large and never read by hand.
- Decided 2026-10-25 (Tom): (a).

**Q12. Is the brief's event-sourced `store` crate still a vapors
feature?** Brief 4.10; touché built `touche-document` with it.
- (a) It stays touché's. vapors provides nothing; the store is app
  state.
- (b) Extract it into vapors as a generic event log and projections.
  No second user yet; generality without a pull.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q13. Is the brief's JSON-RPC automation API (4.9) still wanted?**
Recording (area 10), scenario replay and remote functions (area 8) cover
the test harness, the UX review and external calls.
- (a) Defer it. Tests inject at the platform layer; an inspector is a
  later tool over the recorder's snapshot; external scripting goes
  through remote functions when accounts exist.
- (b) Build it in F2 as the brief says. A fourth surface to keep in step.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q14. 3D scene vocabulary: a fixed typed set, or reflection over a
Rust 3D crate?** Area 9 T3.
- (a) Fixed, typed tags (`<mesh>`, `<camera>`, `<light>`, `<instances>`)
  over vapors' own instanced renderer on the one device. Small; refuses
  what it does not know; the engine's textures are materials directly.
- (b) threlte-like reflection over a crate such as `three-d` or
  Bevy's renderer. Breadth; a second renderer and device ownership
  questions.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q15. Web shared memory: detect the isolation headers, or never?**
Area 2 S5.
- (a) Detect COOP/COEP at load; `offload` uses `SharedArrayBuffer` when
  present and copies otherwise; the choice is reported and recorded.
  Shared wasm memory needs a nightly toolchain with `build-std` and
  atomics today, so this is a second build of the web package.
- (b) Never; always copy. One path, one toolchain; a 24 MP decode copies
  96 MB.
- Recommendation: (b) until a measured copy costs a frame, then (a).
- Decided 2026-10-25 (Tom): (b).

**Q16. WPT reftests as paint comparisons?** Area 11 Q2.
- (a) Yes, with a committed skip list naming why each skipped test is
  skipped.
- (b) Only hand-ported testharness cases.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

## Product decisions

**Q17. Recording retention.** Area 10 C5.
- (a) 7 days locally, 30 days uploaded, stated at opt-in.
- (b) 30 and 90.
- Recommendation: (a) until a review needs older sessions.
- Decided 2026-10-25 (Tom): (a).

**Q18. Is the agent's generated text recorded?** Area 10.
- (a) Recorded: it is the product's output and what a UX review reads.
- (b) Masked like user text.
- Recommendation: (a), with a per-app policy switch.
- Decided 2026-10-25 (Tom): (a).

**Q19. Live-region cadence for streamed replies.** Area 5 A5.
- (a) Announce at sentence boundaries, at most once per second.
- (b) Once per second regardless.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q20. Momentum scrolling curve.** Area 3 E4 / F2d.
- (a) Android Chrome's, recorded from the Pixel as a transcript.
- (b) iOS's decay constant.
- Recommendation: (a): the phone build is Chrome on Android.
- Decided 2026-10-25 (Tom): (a).

**Q21. Routing: when, and does native have routes?** Area 8 F6.
- (a) After accounts; natively a saved route state, no URL.
- (b) With the library view, whichever comes first.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q22. Priority of 3D stage 1 (CSS 3D on elements) against the rest of
the edit UI.** Area 9 T1, T2.
- (a) After the edit UI's slices 1 to 3 (cards, sliders, animation).
- (b) Before slice 3, so the layer view's fan-out is the animation
  showcase.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

## Small taste questions

**Q23. A failure outside every boundary.** Area 1 rule 5.
- (a) Stops the app with the report, as a refused update does; an app
  that wants a default card wraps its root in a boundary.
- (b) An implicit root boundary with a default failed card.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q24. Capture-phase listeners (`oncapture:click`).** Area 3.
- (a) Deferred until something needs one.
- (b) Now, for completeness.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q25. Positive `tabindex`.** Area 3.
- (a) Refused, naming the fix (`0` or `-1`).
- (b) Honoured in numeric order.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).

**Q26. The boundary's tag.** Area 1 R5.
- (a) `<boundary>`.
- (b) `<vapors:boundary>`, marking it as not HTML.
- Recommendation: (a); the compiler's vocabulary already has non-HTML
  tags for scenes.
- Decided 2026-10-25 (Tom): (a).

**Q27. Virtualised list: a vapors component now or when measured?**
Brief component 2.
- (a) When the layer stack or history list shows a measured frame over
  budget.
- (b) Now, since the conversation history will need it.
- Recommendation: (a).
- Decided 2026-10-25 (Tom): (a).
