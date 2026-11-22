# Virtual list: thin slices

The gui-bench `list` scenario (`docs/gui-bench/spec/dashboard.rs`,
`LIST_ROWS_TOP`) shows `--rows` order rows in the framework's virtual
list, with only the rows in and near the window existing. These slices
build towards it. Each slice ends with a page or a test a person can
check.

## V1: the `scroll` event (this slice)

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

## Next slices (to replan after V1)

- V2: a virtual list component over a fixed row height: spacer above
  and below, rows in and near the window keyed by index, held to the
  `list` scenario's geometry at a few hundred rows.
- V3: the gui-bench `list` scenario at its full `--rows`, with the
  scaling judge's counts flat in the row count.
