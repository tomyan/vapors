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

## Open

- A height that follows the list's laid-out size (`height` is read at
  mount); rows of differing heights; another crate's `.wisp` component
  inside a snippet.
- A deep lane that declines framing (`LaneFramePixel`: a fractional
  start) is placed in f32, off by fractions of a pixel, and not
  reported.
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

