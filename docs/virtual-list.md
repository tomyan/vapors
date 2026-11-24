# VirtualList

`vapors_components::virtual_list::VirtualList` is a vertical scroll
container that holds only the rows in and near its window. Every row is
`row_height` px tall; the window is the container's laid-out height,
which follows the page as it resizes. The rows any part of
which is in the window exist, and `overscan` more beyond each edge (4 by
default), cut to the keys there are (`crates/components/src/virtual_window.rs`).

```html
<script>
use vapors_components::virtual_list::{VirtualList, VirtualListProps};
let keys: Signal<Vec<usize>> = Signal::new((0..1000).collect());
</script>

<VirtualList keys={keys} row_height={20.0} height={300.0}>{snippet row(key: usize)}<div>Row {key}</div>{/snippet}</VirtualList>
```

`keys` (a `ReadSignal<Vec<usize>>`; a `Signal` or `Memo` binding
converts) are the rows' stable identities in the order shown. Each
existing row is `row(key)` inside a box of the row height, so the
geometry holds whatever the snippet's own height. A row whose key stays
near the window keeps its elements across a scroll, and a reorder of
`keys` moves them with the key; a key that leaves the window drops its
row. The list's `scroll` event sets the offset the rows follow, so a
scroll draws its new rows in the same frame (`docs/plans/virtual-list.md`,
V1). With no scroll and no change to `keys`, nothing changes.

The container is `overflow-y: auto`. With `height` above 0 its height
is bound to `height`; at 0, the default, it is `height: 100%`, so it
fills a parent whose height is definite (a box with a set height, or
an absolutely placed box between `top` and `bottom`). Either way the
window is the height its `resize` last reported
(`docs/components.md`, "Resize"), `height` until the first. The
frame's first update lays the list out at its new height, its `resize`
fires, and the frame's second update builds the rows of that height,
so the first frame drawn, and each frame after a resize, shows the
rows of the height it lays out. Its content is a box `keys.len() * row_height` tall holding a spacer as
tall as the rows above the first that exists, then the rows. Inside it:
`.virtual-list`, `.virtual-content`, `.virtual-above`, `.virtual-row`.

Refused, naming the prop: a `row_height` that is not a positive finite
number (the default, 0, is refused at mount), a negative or non-finite
`height`, and a count of rows some count up to which an `f32` holds
only to more than 1/128 px (it would put rows off their pixel). The
refusal depends on the count alone and only rises with it, so a list
whose content height holds at mount cannot be refused by a scroll. A
million rows of 52 px are exact; 33.3 px rows hold up to 7,872 rows and
are refused from 7,873 (33.25 or 33.5 px hold far more).

Every prop has a default, because a tag from another crate cannot pass
a required prop yet (`docs/components.md`, "Required props").

Not built: rows of differing heights, horizontal lists, and keys other
than `usize`.
