# Insert and remove: the style phase flat in rows (2026-11-13, main at adb3b677)

`--scaling Insert` at ed4e4704 (`docs/perf/splice-layout-2026-11-13.md`,
PR #270's harness, which reports insert and remove frames apart) put
`style` at 620k / 721k / 1,036k instructions on insert frames and
59.8k / 151k / 468k on remove frames at 1,000 / 3,000 / 10,000 rows:
45 instructions per extra row on both kinds, against finish-line
property 3 (`docs/plans/perf-finish-line.md`). The restyle itself was
already narrow: `elements_restyled` is 4 on a middle insert at every
size (`crates/elements/tests/insert_restyle_rows.rs`).

## Where the growth was

Measured by splitting the `style` span into one `update phase` span
per step of `Document::styled` (`crates/elements/src/update.rs`) and
`Document::restyle`, on a scratch copy of this branch (not committed),
`vapors-perf 30 5 --instructions --only "<n> rows, Insert"`, release,
median over frames (the remove frames' median; the insert frames' is
the same plus the new row's cascade):

| step | 1000 rows | 10,000 rows | per extra row |
|---|---|---|---|
| `touched` | 2,261 | 2,261 | 0 |
| `recheck_tree` | 53,878 | 461,668 | 45.3 |
| `tree_refusals` | 1,198 | 1,198 | 0 |
| `style_attribute_refusals` | 1,186 | 1,186 | 0 |
| `restyle` (`dirty_roots`, `restyle_all`, `sweep`) | 14,874 | 13,756 | 0 |
| `rechecked` | 2,838 | 2,823 | 0 |
| `recheck_styled` | 4,262 | 4,412 | 0 |
| `all_styled_refusals` | 1,236 | 1,236 | 0 |

All of it was `recheck_tree`: an insert or remove puts a children mark
on the body (`Marks::mark_children`, `dom_impl.rs`), `touched` checks
every marked node again, and the body's tree check
(`check_element`, `crates/elements/src/check.rs`) called
`right_to_left_text`, which collected `arena.children(body)` into a
`Vec` and read each child's kind: one allocation and about 45
instructions per row, on every insert and every remove.

## The change

The right-to-left text check is the text node's own, kept in
`KeptChecks::tree` by the text node's id and named for its parent in
the message as before (`check_node`, `check.rs`). An element's tree
check reads the element alone, so a parent whose children changed is
checked in constant time. For that:

- `touched` and the added subtrees list every node, text nodes
  included, each once (`style_checks.rs`); `rechecked`, whose checks
  read computed styles, keeps the elements among them.
- `set_text` marks the text node for its own check as well as its
  parent's box (`Document::mark_text_changed`, `restyle.rs`).
- `forget_subtree` forgets text nodes too, so a removed text node's
  refusal goes with it.
- `UpdateCounts::nodes_checked` counts the nodes the tree check read
  again; `crates/elements/tests/insert_checks_rows.rs` pins it at 8
  for a middle insert (the body, the row's four elements and three
  text nodes) and 1 for the remove, the same at 100 and 400 rows.
- The count says which nodes were checked, not what each check read
  (review of #279, M2). So the checks are given the node's own data
  and a name for messages, never the arena (`check_element`,
  `right_to_left_text`, `attribute_refusals` in `check.rs`): a check
  that read children again would not compile. And `--scaling Insert`
  now judges `style` flat beside the accessibility ceiling
  (`Judged::AccessibilityPerRow { flat: &["style"] }`,
  `scaling_judged.rs`; `scaling_flat_tests.rs` shows the numbers above
  named as a violation), so a run fails if it grows per row again.
- A node moved into a parent outside the root's tree, not removed,
  kept its checks before (review M1: a refused text node or element
  moved out this way refused the next update as unmarked, in release
  too). `Document::mark_moved_out` (`restyle.rs`) forgets the subtree
  when the new parent is not attached; `kept_checks_moved.rs` pins it
  for a text node, for a refused-tag element, and for a subtree moved
  out and back.
- One refusal per text node now, where it was one per element: an
  element with two right-to-left text children is named twice
  (`check.rs`, `right_to_left_text`).

Each check was proven by breaking what it guards: the pin by
checking each mark's subtree (708 and 2,808 at 100 and 400 rows); the
inserted text node's check by walking elements only in the added
subtrees (the nested case too); the removed text node's by forgetting
elements only; the `set_text` mark by dropping it; the move out of the
tree by dropping the forget in `mark_moved_out`; the flat judge by
dropping the `flat` loop in `Scaling::violations`; and a children read
in `check_element` by writing one, which does not compile. Each failed
on its test; each is restored.

## After

`vapors-perf 60 10 --scaling Insert`, this branch with PR #270's
harness applied for the run (release, Intel UHD 630, load 35 to 45;
instruction counts do not move with load):

| phase | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 1000 to 3000 | per extra row, 3000 to 10,000 |
|---|---|---|---|---|---|---|
| style | insert | 554,616 | 557,049 | 557,366 | 1.2 | 0.0 |
| style | remove | 8,145 | 8,145 | 8,146 | 0.0 | 0.0 |
| update | insert | 15,578,878 | 46,039,374 | 156,014,641 | 15,230 | 15,711 |
| update | remove | 14,963,360 | 45,438,743 | 155,383,261 | 15,238 | 15,706 |
| layout | insert | 3,567,198 | 10,238,350 | 33,908,167 | 3,336 | 3,381 |
| paint | insert | 11,062,588 | 34,231,795 | 118,035,370 | 11,585 | 11,972 |
| accessibility | insert | 9,728,510 | 28,522,027 | 95,873,156 | 9,397 | 9,622 |

Style is within `--scaling`'s tolerance on both kinds (under 2% and
under 5,000 instructions between sizes): the insert frames' 554k is
the new row's cascade and used-style derivation (four elements;
`docs/plans/style-allocations.md`), the remove frames' 8.1k the checks
of the body and the restyle pass's bookkeeping. Layout, paint and
accessibility are unchanged and still grow per row; the layout lane
(`docs/plans/splice-layout.md`) is theirs. The run exits non-zero on
the accessibility ceiling as it does on main (`docs/perf/
splice-layout-2026-11-13.md`, "The accessibility ceilings fail on
main"); `Insert` is judged on accessibility alone
(`scaling_judged.rs`) and, since the review of this change, on `style`
held flat; the lane's PR 5 moves it to `EveryPhase`.
