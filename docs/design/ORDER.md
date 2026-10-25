# Build order

Status: 2026-10-25. Driven by what touché needs next: the edit UI, in
Tom's slices (2026-10-25): (1) a layer stack of read-only cards and a
history timeline for agent edits; (2) draggable sliders; (3) agent
changes animating the controls; (4) tools from one catalogue; (5) the
curves control and the sampler. Then the chat panel in vapors, then the
rest. Each row names the area slice (`<area>-<id>` from the area
documents), what it depends on, and what touché ships with it. Rows in
one step are independent and can run in parallel worktrees.

The rule from the brief holds: a framework slice is pulled by a touché
slice, and after each step the plan is reread.

## Step 0: in flight

| Slice | Depends on | touché ships |
|---|---|---|
| F2c S9 scrolling (B–D) | | Lists that scroll inside their region, with a thumb. S9 lands without momentum: its curve has no oracle until a transcript is recorded from the Pixel, and that transcript is F2d's first step |
| F2c S10 `img` | | Thumbnails |
| 2-S1 phases and clock | | Per-phase frame time in the stress demo and P0 |
| 11-Q1 DevTools in the oracle runner | | Chromium's a11y tree, event logs and focus after key dispatch, which step 1's 5-A1 and 3-E2 compare against; the runner is `--dump-dom` only today |

## Step 1: cards

The layer stack is a scroll region of cards. A card is a component with
a title row and slotted controls; its contents come from the layer's
adjustment. The history timeline is a second list of entries.

| Slice | Depends on | touché ships |
|---|---|---|
| 1-R1 snippets and children | | `<Card title=..>` with controls inside; one card component for every layer kind |
| 3-E2 keyboard and focus (Tab, `:focus-visible`, `outline`, button keys) | | Every card button reachable by keyboard |
| 5-A1 roles, names, the tree, the unnamed-control lint | 2-S1 | The layer list is a list; every control named (acceptance 6, native) |
| 10-C1 recorder core | 2-S1 | Dev builds record; P0 timings from the Pixel |
| 11-Q5 budgets in CI | 2-S1 | Per-phase ceilings held from the first measurement |

touché slice 1 ships here: read-only cards and the timeline, agent
edits appearing as cards.

## Step 2: sliders and toggles

| Slice | Depends on | touché ships |
|---|---|---|
| 3-E1 capture and drag | | A thumb that follows the finger past the track |
| 3-E3 range, checkbox, number with UA parts (Q1) | 3-E1, 3-E2 | Draggable sliders, visibility toggles, numeric readouts that scrub |
| 3-E4 wheel routing and `touch-action` | S9 | The stack scrolls beside the canvas on the phone without zooming it |
| 2-S3 animation tick | 2-S1 | The phase 6-A1's running transitions are sampled in |
| 11-Q3 stepped clock and manual executor | 2-S1 | Transition and async tests that never sleep |
| 6-A1 transitions (paint and layout properties, `transform` 2D, leaf `opacity`) | 2-S1, 2-S3, 11-Q3 | Hover and pressed states ease |

touché slice 2 ships here: dragging a slider edits the layer, history
gets one entry per drag.

## Step 3: agent edits animate

| Slice | Depends on | touché ships |
|---|---|---|
| 6-A2 value controls animate their display (Q2) | 6-A1, 3-E3 | An agent's edit moves the slider and readout from old to new |
| 6-A5 FLIP on keyed rows | 6-A1 | Cards reorder with motion |
| 6-A6 enter and leave (`@starting-style`, `allow-discrete`) | 6-A1 | A new layer's card slides in; a removed one leaves; toasts |
| 2-S2 executor | 2-S1 | Futures polled in phase 2, woken through `Waker` |
| 1-R2 async memo and the executor | 2-S2, 11-Q3 | A card's measurement computed off the frame |
| 1-R3 held updates | 1-R2 | A readout and the canvas change in the same frame when the render lands |
| 5-A2 a11y actions through dispatch | 5-A1 | A screen reader moves a slider |
| 7-P1 service traits, `Outcome<T>`, the lint | | impl-shell's guard check has its list |

touché slice 3 ships here.

## Step 4: one tool catalogue, the curves control

| Slice | Depends on | touché ships |
|---|---|---|
| 3-E5 `contextmenu`, long press, `details` | 3-E1 | Card actions ("a bit less", remove); collapsible cards |
| F3 custom paint: `canvas` element with paths and polylines, overlays in image coordinates (no area document yet; the brief's 4.3 and 4.5 are its specification and one is written before the slice starts) | 3-E1 | The curves control with draggable points; the histogram; the sampler marker |
| 5-A4 text and 5-A6 dialogs and landmarks | 5-A1 | Curve points as focusable, named children |
| 6-A4 `@keyframes` | 6-A1 | The busy line on a working button |
| 7-P2 clipboard | 7-P1 | Phone Paste and native Ctrl/Cmd+V |
| 10-C2 redaction and 10-C3 replay | 10-C1 | UX review from recordings |

touché slices 4 and 5 ship here.

## Step 5: the chat panel in vapors

| Slice | Depends on | touché ships |
|---|---|---|
| F2d `br`, `a`, mixed inline metrics, `tnum` and variation settings | | Chat text with inline code; readouts that do not jitter |
| 3-E6 text editing, 3-E7 web IME (Q9) | 3-E2 | The Composer in vapors on both targets |
| 1-R4 `await` in script and template, 1-R5 `<boundary>` | 1-R3 | The timeline's first load with pending content; failed decodes shown in place |
| 5-A3 web a11y mirror (Q8), 5-A5 live regions | 5-A1 | TalkBack on the Pixel; the reply read as it streams |
| 6-A3 opacity layers and blur | 6-A1 | The before/after fade; sheets over a blurred canvas |
| 7-P3 `KeyStore` and `Transport` move in | 7-P1 | touché's runtime has no platform code |
| 10-C4 scenarios, 11-Q4 scenario tier | 10-C3 | Phone and desktop walkthroughs per PR, judged |

The native app has every MVP surface in vapors after this step; the
HTML chrome is retired.

## Step 6: heavy work off the thread, export

| Slice | Depends on | touché ships |
|---|---|---|
| 2-S4 offload pools, 1-R6 `#[offload]` (Q5) | 2-S2 | Export encoding and decode off the UI thread on both targets, Cancel |
| 7-P4 storage, 7-P5 save picker, share, download | 7-P1 | Export's three paths and the profile on one API |
| 7-P6 theme and motion preference, 6-A7 reduced motion | 7-P1 | The accessibility pass |
| 2-S5 shared memory (Q15) | 2-S4 | Only if a copy costs a frame |

## Step 7: remote

| Slice | Depends on | touché ships |
|---|---|---|
| 8-F1 remote base over 7-P3's `Transport`: the stand-in server, deadlines for remote calls | 7-P3 | One shape for the key check and events upload |
| 8-F2 `#[query]`/`#[command]` (Q10, Q11), 8-F3 batching and `updates` | 8-F1 | `/events` upload; the licence check; the broker's allowance in one round trip |
| 8-F4 streaming query | 8-F2 | The agent's turn through the broker |
| 10-C5 upload and consent, 10-C6 measures | 8-F2 | Recordings from people who are not Tom; the UX scorecard |
| 8-F5 embedded binding | 8-F2 | The paid desktop offline |

## Step 8: 3D

| Slice | Depends on | touché ships |
|---|---|---|
| 9-T1 3D transforms, flat; 9-T2 `preserve-3d`, `backface-visibility` | 6-A1, 11-Q2 WPT reftests | The layer view's fan-out, animated |
| 9-T3 the `scene` node, 9-T4 presentational CSS, 9-T5 bulk tasks and instancing (Q14) | 9-T2, 2-S1 | The layer view as a scene with the engine's textures |
| 2-S6 render in a worker | 2-S4, 10-C1 timings | Only if P0 shows main-thread drops |

## Step 9: routing and later

| Slice | Depends on | touché ships |
|---|---|---|
| 8-F6 router and load (Q21) | 8-F2 | Library and settings; deep links |
| 3-E8 `select`, `dialog`, popover | 3-E2 | The model choice; About; sheets as elements |
| 3-E9 pen | 3-E1 | Brush masks (milestone 3) |
| 9-T6 lights, PBR, glTF | 9-T3 | None scheduled |

## What this order assumes

- Scrolling (S9) lands before the cards; a stack that cannot scroll is
  not a stack.
- Q1 (c) and Q2 (b) were decided on 2026-10-25; the rows assume them.
- The oracle runner gains DevTools (11-Q1) in step 0, since step 1's
  roles-and-names and focus oracles need Chromium's a11y tree and key
  dispatch, which `--dump-dom` cannot give.
- The recorder (10-C1) is early because P0's phone numbers are what
  decide steps 6 and 8's optional rows.
- Nothing in steps 7 to 9 is needed for the free web MVP; the order
  there follows Tom's week plan (release build, signal and speed first)
  and can move.
