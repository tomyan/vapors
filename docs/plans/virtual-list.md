# Virtual list: thin slices

The gui-bench `list` scenario (`docs/gui-bench/spec/dashboard.rs`,
`LIST_ROWS_TOP`) shows `--rows` order rows in the framework's virtual
list, with only the rows in and near the window existing. These slices
build towards it. Each slice ends with a page or a test a person can
check.

## V1: the `scroll` event (done, #600)

A scroll container's listener hears `scroll` when its effective offset
changes, so a component can choose which rows to show from the offset.

Behaviour, as Chromium's element `scroll` event, which the browser
fires once per frame in its "run the scroll steps" before style and
layout:

- `scroll` is in the events the tree fires (`vapors_elements::FIRED_EVENTS`),
  so `onscroll={handler}` on an element is accepted.
- `Document::settle_scroll(&doc)` fires one `scroll` at each scroll
  container whose offset differs from the one its last `scroll`
  reported (0, 0 for a container that has not fired). The offset is
  the clamped offset `Document::scroll_offset` answers, in CSS pixels,
  unrounded, compared exactly. The app calls it once per frame before
  the update, after `settle_selection`; so a listener's change to the
  tree is drawn in the same frame.
- The event does not bubble (an element's `scroll` does not in the
  DOM) and carries the offset: `Event::scroll_offset()`, `Some([x, y])`
  on a `scroll`, `None` on any other event.
- Routes that change the offset, all through `Offsets::scroll` or the
  update's clamp: wheel (`scroll_by`), touch and fling
  (`scroll_node_by`), programmatic (`scroll_to`), the a11y press's
  scroll into view, and the update clamping an offset whose content
  shrank.
- No `scroll` when the offset did not change: a `scroll_to` the same
  offset, a wheel at the end of the range, a scroll and its reverse
  between two settles.
- Several scrolls between two settles are one `scroll`, carrying the
  last offset.
- A clamp made by an update is owed until the next settle:
  `Document::take_changed` answers true while any `scroll` is owed, so
  an app that redraws on change draws the frame that fires it.
- Idle: with no offset changed, `settle_scroll` fires nothing and
  `take_changed` stays false.

Tests:

- `crates/elements/tests/scroll_event.rs`: each route fires one
  `scroll` with the offset; unchanged and clamped offsets fire none;
  coalescing; no bubbling; the layout clamp; idle.
- `crates/elements/tests/a11y_act_reach.rs`: a press that scrolls its
  node into view owes the container one `scroll` with the new offset.
- `crates/compiler-examples/tests/scroll_rows.rs`: a small compiled list
  (`src/scroll_rows.wisp`) whose `onscroll` picks the first visible row
  shows the rows at the new offset after a wheel scroll, and none
  change while it is not scrolled.
- `crates/app/tests/gpu_scroll_event.rs`: the app fires `scroll` in the
  frame after a wheel event, and the frame after that fires none.

Out of scope: the virtual list component itself, keyboard scrolling
(the tree has none), `scrollend`, and a `scroll` on the viewport (the
page does not scroll in vapors).

## V2: a virtual list over a fixed row height (done)

- **V2a.** `vapors_components::virtual_list::VirtualList`
  (`docs/virtual-list.md`): rows of one height in a window of a given
  height; the rows touching the window and `overscan` (4) beyond each
  edge exist, below a spacer as tall as the rows above them, in a box
  as tall as every row. Rows are keyed by the identity `keys` give, not
  by index, so a kept row is the same element across a scroll and moves
  with its key on a reorder. Tests: `crates/components/src/virtual_window_tests.rs`,
  `crates/compiler-examples/tests/virtual_rows.rs` (20 rows of 20 px).
- **Compiler prerequisite** (met by V2b): a `.wisp` component of the
  same directory inside a snippet now compiles (`docs/components.md`,
  "A component inside a snippet"); one of another crate still does not.
- **V2b.** `vapors-bench --scenario list --rows N`: the orders header
  over `VirtualList` of N order rows keyed by order index, sorted by the
  AMOUNT title into `list_order`. Tests at 300 rows:
  `crates/gui-bench-app/src/tests/list_layout.rs`, `list_behaviour.rs`.

## V3: the list at a million rows (done)

- **Placement prerequisite.** A million 52 px rows is 52,000,000 px,
  where an f32 steps by 4 px: placement put the boxes inside the deep
  rows on a 4 px grid, and the debug placement check panicked. A lane
  whose children start 65,536 px or more down its scroll frame now
  frames them however few (`docs/plans/splice-layout.md` section 11.1),
  and the check composes in f64. Tests:
  `crates/elements/tests/deep_scroll_rows.rs`.
- **The list.** `crates/gui-bench-app/src/tests/list_million.rs`: the
  last order reachable at the clamped end, the deepest row's boxes at
  the spec's offsets, 19 rows existing at the top and 23 mid-list at a
  thousand rows and at a million alike, rows 599,996..600,019 existing
  at row 600,000, the sort's first rows and last row, and idle (same
  elements, `take_changed` false).

## V4: the window follows the list's laid-out height (done)

The spec's list fills the window below the header at any window size,
as Zgui's virtual list does: its height comes from layout, and the list
reads the scrollport's laid-out size after each layout.

- **The `resize` event.** `Document::settle_scroll`, after the
  `scroll`s, fires one `resize` at each scroll container whose laid-out
  client size (its padding box: vapors' thumbs overlay it) differs from
  the one its last `resize` carried, (0, 0) for one that has fired none,
  carrying it in CSS px, unrounded, compared exactly:
  `Event::client_size()`. Only a container with a `resize` listener
  owes one, as a `ResizeObserver` reports only what it observes, so a
  page without one draws no extra frame. It does not bubble, and only
  scroll containers fire it. The DOM has no element `resize`; a page there
  uses a `ResizeObserver`, whose callbacks run once per frame after
  layout, and this is modelled on that report for scroll containers.
  Tests: `crates/elements/tests/resize_event.rs`.
- **In the frame that lays it out.** A browser runs the observer's
  callbacks before it paints and lays out again if they changed
  anything, repeating while each round reaches elements deeper in the
  tree than the last, then reporting a loop error, so it never paints
  the old rows at a new size.
  `Document::update_frame`, the app's Update phase, runs one such round
  within the bounded updates it already ran for the focus fixup's blur
  and the rehover:
  1. the `scroll`s and `resize`s owed, before the first update: one
     `scroll` a container a frame, as Chromium's scroll steps;
  2. the first update; then the `resize`s it owes, each carrying the
     offset as laid out, so a list at its end that grew taller builds
     the rows of the clamped offset now (the clamp's `scroll` fires in
     the next frame's step 1, with the offset the list already has);
  3. the second update, if anything changed;
  4. a third, only when step 2 fired a `resize` and the second update
     left the document changed: the hover moved by rows built under a
     resting pointer, or the second fixup's blur;
  5. the refusal, if the last update left the document changed.

  So the first frame drawn holds the rows of the list's height, as does
  each frame after a resize, with the pointer anywhere. Zgui's frame,
  at the commit the bench pins (zgui 5bb4387,
  `crates/zgui-runtime/src/window/frame.rs` and `observe.rs`), also
  draws the new rows in the frame that resizes: after layout it
  dispatches the frame's `scroll`s, then delivers observations, at most
  two passes, then settles again. It differs in two ways. Zgui fires
  `scroll` after layout, so an offset that layout clamped (a taller list at
  its end, keys that shrank) reaches the list in that frame; vapors
  fires it before the first update, and only a `resize` carries a clamped
  offset in the same frame. Keys that shrink at the end with no
  resize, which main has too, draw one frame without rows. And a
  `scroll_to` from a `resize` handler reaches `onscroll` in the next
  frame. Zgui runs up to two observation passes; vapors runs one. A size the
  last update changes again (a handler that resizes its own container)
  is owed, `take_changed` answers true, and the next frame fires it, so
  such a page draws a frame each time it changes the size, where a
  browser would report a ResizeObserver loop error. Tests:
  `crates/elements/tests/update_frame.rs` (the order, the bound, a
  settled frame updating once), `update_frame_settle.rs` (a resting
  pointer, a hover that moves boxes refused after two updates or three,
  one `scroll` a frame, the offset on the `resize`) and the single-frame
  checks of `virtual_fill.rs` and `list_resize.rs`.
- **The list.** `VirtualList`'s window is the height the last `resize`
  reported, `height` until the first. `height` above 0 still sets the
  container's CSS height; at 0, now the default, the list fills its
  parent's height (`height: 100%`). Tests: `crates/compiler-examples/tests/virtual_fill.rs`
  (embedded in a panel under a header: 200, 300 and 120 px windows, at
  the top, 400 px down and at the end); `virtual_rows.rs` keeps a
  60 px `height`.
- **The bench.** The list sits in a box from the header to the
  window's bottom and fills it; `crates/gui-bench-app/src/tests/list_resize.rs`
  holds a million rows in 1000 and 600 px windows: the bottom row of
  the window, the last order reachable at the end of the shorter, the
  rows below row 600,000 and the last order at the end after the
  taller.

## Open

- Rows of differing heights; another crate's `.wisp` component inside
  a snippet; a `{render}` of a snippet prop inside a local snippet,
  which compiles against `dyn Dom` where the block needs the mount's
  `D` (`VirtualList` writes its content out in both branches of
  `{if fixed}` for this).
- A deep lane that declines framing (`LaneFramePixel`: a fractional
  start) is placed in f32, off by fractions of a pixel, and not
  reported.
- The debug placement check's on-page comparison is in f32: below
  16,777,216 / scale px it holds a framed lane's starts to whole device
  pixels, deeper only to the f32 step there (4 px at 52,000,000 px).
  Declared RELAXED at `placement_check.rs` `in_lane_frame`.
- Follow-ups filed: #608 (judge depth framing on a deep page of small
  lanes, such as a grid of cards) and #609 (`cargo xtask scaling` looks
  for the judge binary under `./target`, not cargo's target directory).
- The corpus event player (`crates/elements-corpus/src/events_vapors_act.rs`)
  neither settles scrolls nor hears a non-bubbling `scroll`; it refuses
  `wheel` today. Needed when wheel playback lands.
- The scaling judge (`cargo xtask scaling`) has no `list` scenario: the
  counts' flatness in the row count is pinned by the tests above, not
  by the judge. No benchmark timing was taken.

## Framework guide impacts (`plugins/vapors/skills/vapors`)

Recorded, not yet edited; the guide is the skill an app author reads.

- `SKILL.md` "Rough edges" says an element's position cannot follow a
  signal, and `reference/wisp.md` ("Classes and inline style") sends
  movement to the 3D scene. `style:height={..}` and the other bindable
  properties (`docs/components.md`, "Style bindings") do this today, and
  `VirtualList` depends on them; neither page mentions `style:`.
- `reference/wisp.md` ("Events") does not show `onscroll` or
  `event.scroll_offset()`; `reference/supported.md` lists `scroll` only.
- No page names a component vapors ships (`HoldButton`, `Combobox`,
  `VirtualList`) or how to mount one from another crate: every prop
  needs a default, since such a tag cannot pass a required prop yet.
- Workaround an author meets: a `.wisp` component of another crate
  inside a snippet does not compile (rustc: mismatched types at its
  `mount_before`); wrap it in a component of the app's own directory.
  `VirtualList`'s `row` snippet holding `<OrderRow>` works because
  `OrderRow` is in the gui-bench app's directory.

