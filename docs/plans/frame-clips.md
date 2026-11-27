# Frame clips on the clip table (before F3.2c)

Status: slices 1 and 2 built (see "As built"); slice 3 is a proposal.

## Why

F3.2c (the order card's rounded clip) measured two costs of a rounded
clip over a lane (`flat-scene-f3.md`, F3.2b note, 2026-11-27):

- One row inserted mid-list under a rounded `overflow: hidden` list
  painted 306 boxes at 100 rows and 3006 at 1000, against 6 under a
  square one. The list's lane declines to frame its rows
  (`LaneDecline::LaneFrameRoundedClip`), so its rows are placed and
  painted as before lane frames.
- A list past 65,536 px is refused: its rows' cells frame for depth,
  under the rounded clip, and a frame's clip is a square scissor
  (`RoundedRefusal::Frame`).

Both come from one fact: a frame's clip reaches the GPU only as a
scissor. Tom chose to put frame clips on the clip table first.

## Today (read 2026-11-27, at 66a3eebf)

- **Elements.** A frame's clip is `FrameAt::clip` (CSS px on the
  page) and `FrameAt::device_clip` (device px), a rectangle with no
  radii (`frames.rs` 19-32).
  - A scroll frame's clip is its container's inner clip met with the
    outer frame's (`frames_update.rs` `work_out`).
  - A lane's rows share one template clip (`frames_lane_prime.rs`
    `lane_template`).
  - The `BoxClip`'s `rounded_by` is dropped there (`.rect`).
- **Painter.** `FrameDraw::clip` becomes the batch's scissor
  (`painter_runs.rs` `scissor`, `open_pass.rs` `set_scissor_rect`).
  - Clip entries are made only from the items' own clips
    (`painter_kept_entries.rs` `table_entries`).
  - `ClipEntry::parent` is always 0, so every chain is one link
    (`clip_table.rs` 44-52).
  - The shader already walks `parent` up to `MAX_CLIP_CHAIN` (8). Each
    link applies its own frame's transform (`clip.wgsl`
    `clip_coverage`).
  - `assert_agrees` holds the table and the scissor together, and
    `entry` refuses a clip with an `outer`.

## Proposal

A frame with a clip gets a clip entry of its own: its clip, with radii
when its container's clip is rounded, in its **outer** frame's
transform. Every item entry in that frame chains to it through
`parent`, and an unclipped item in a clipped frame takes the frame's
entry as its clip. The shader is unchanged, since it already walks
parents and applies each link's transform.

The scissor stays, as each frame clip's bounding rectangle: it culls,
and bounds the fragments drawn. The table cuts the exact shape. Images,
surfaces and meshes still draw under the scissor alone, so a rounded
frame clip over one stays refused until F3.1b moves them onto the table.

Rows of a lane share their template's one entry (item 3's sharing),
named by the lane. The row items' entries are in the row's transform,
and their parent in the container frame's.

## Slices

1. **Square frame entries, no visible change.**
   - Each clipped scroll frame and lane template gets an entry.
   - Item entries chain to it, and unclipped items take it.
   - The scissor stays as it is.
   - A check in every build holds each frame entry's rectangle, moved
     by its transform, to the scissor's rectangle. This replaces
     `assert_agrees`' frame half.
   - Tests: the chains on the table for a scroll frame, a nested one
     and a lane; on the GPU, a frame entry cutting an item with the
     scissor widened to the target. That is the break that shows the
     table, not the scissor, cuts.
   - Elements' chain count (`clip_chain_frames.rs`) and the painter's
     chains are held to one answer.
2. **Rounded frame clips.**
   - `FrameAt` carries radii, from its container's rounded `BoxClip`,
     and hits in a rounded frame are cut by its shape
     (`kept_hits_query.rs`).
   - `RoundedRefusal::Frame` is retired for table runs. Images and
     surfaces under a rounded frame clip are refused, as under an item
     clip.
   - A rounded scroller (`overflow: auto` with a radius) stays refused,
     deferred as Tom deferred it in F3.2b and outside the parity scope
     (entry 99, action 43).
   - A frame nested in a rounded frame is refused, naming the follow-up
     that chains frame entries to their outer frames', unless it is held
     clear of the rounded frame's corners, as an item clip is.
   - `assert_no_parent_in` returns early when nothing is given back, and
     its cost is measured on a large dashboard (the lead, after #639).
   - Tests: a framed lane and a scroller under a rounded clip from a box
     above them, painted and hit.
3. **Lane decline retired, F3.2c measured again.**
   - `LaneFrameRoundedClip` is removed, so a rounded list frames its
     rows.
   - The insert counts are measured again: expected near the square
     list's 6. A 4000-row rounded list updates.
   - Then F3.2c's branch rebases and ships.

## As built: slice 1

Answered (entry 99, action 43, the lead within Tom's approved work): the
scissor stays as the bounding rectangle, slice 1 lands alone, and the
repaint policy under a rounded item clip waits for a measurement.
Rounded scrollers stay outside the agreed scope.

Built (`crates/app/src/painter_clips.rs` `frame_clip`,
`painter_kept_entries.rs`) differently from the proposal above in one
respect: a frame's entry is its clip **on the page**, under transform
entry 0, the identity, not under its outer frame's transform.
- `FrameDraw::clip` is already the frame's clip met with its outer
  frames', so an entry needs no parent of its own. Every chain is at most
  two links, an item's clip and its frame's.
- The entry is held by its frame (a lane's rows are one frame,
  `BatchFrame::lane`, so they share it), not by its rectangle. A frame's
  clip does not move when its own content scrolls, so that scroll writes
  nothing. When an outer frame scrolls, a nested frame's entry is set
  again in place, one write per nested frame in view. Its index stays,
  so the entries under it and the kept bytes naming them stay. Holding
  it by its rectangle gave it a new index on every outer scroll, and
  vapors-perf's `gpu_f3_baselines` caught that: nested scrollers wrote
  18 batches where 2 are written, and code blocks 6 where 3 are.
- Elements' chain count (`clip_chain_frames.rs`), a link per enclosing
  frame, stays an upper bound on the painter's two. Holding the two to
  one answer is left to slice 2, where a rounded frame clip cannot be
  folded into its outer frames' rectangle and chains grow.

Tests (`crates/app/src/painter_frame_clips_tests.rs`, GPU tier): with
every scissor widened to the target (a `cfg(test)` field), each of these
ends at the list's bottom on the page, with the same pixels as under the
scissor:
- a clipping row crossing a scroll frame's edge (its entry under the
  frame's);
- a row with no clip (its entry the frame's);
- a list scrolled 2 px, and a list in a scroller scrolled 2 px (the
  frame's entry on the page, not under its own transform);
- a framed lane's clipping row. Its clip is moved into the lane
  template's coordinates (`framed_batches_rows.rs` `moved`) under the
  lane's transform entry, and its parent is the lane template's entry.
  No entry is row-relative.
A held entry's parent is held: release refuses to give back an entry a
held one is under (`painter_clips.rs` `assert_no_parent_in`).

Not built from the proposal's list:
- the CPU-side chain tests: the GPU tests above read the chains through
  the shader;
- a check holding each frame entry to the scissor. The two are made
  from one value, `FrameDraw::clip`: the scissor is `on_page(None)`, the
  entry its rectangle, and `assert_whole` holds it to whole pixels.
`assert_agrees` still checks each item entry against the scissor path;
it cuts by `FrameDraw::clip` rather than by walking `parent`, which
gives the same answer while the parent is made from that value.

`tests/gpu_clip_entries.rs` counts one entry more for the list's frame
clip, and a scroll still writes only the two row entries.

## As built: slice 2

The lead's go (2026-11-27, after #639): rounded frame clips, with rounded
scrollers deferred and a frame nested in a rounded frame refused unless
held clear of its corners. That applies Tom's nesting rule to frames.
- A framed lane whose container's clip is rounded, by its own box once
  slice 3 lets it frame or by a box above it, has a rounded clip:
  - `FrameAt::rounded` is a `FrameRound`, the rounding box's padding box
    on the page in CSS px, its padding radii and its slot.
  - It is set on the lane's template (`frames_lane_prime.rs`), so every
    row of the lane shares it.
- `FrameDraw::rounded` carries it in device pixels to the painter, which
  makes an entry for it under the frame's square entry
  (`Key::FrameRound`). The frame's item entries are under the rounded
  one.
- A hit in a rounded frame is cut by its shape too
  (`Frames::round_holds`).
- `RoundedRefusal::Frame` is now for scrollers only. A scroller's clip
  is not rounded while rounded scrollers are deferred.
- What a rounded frame refuses (`clip_refusals.rs`):
  - Decided with each box's clips, not its place (`in_round_frame`, kept
    by `Clips`, `kept_clips_round.rs`): a box in a rounded lane row is
    refused when it is an img, a gpu-surface or an svg drawing shapes
    (`Image`, `Surface`, `Svg`), which draw under the scissor.
  - A framed lane that does not clip in a rounded lane row is refused
    wherever it is (`UnclippedLane`, whose fix is to give it a clip).
  - Each box keeps the box rounding its lane frame (`Clips::round`); a
    change of it reopens what is below the box, so a layout change that
    rounds or squares the container (the card growing until its corners
    fit past its border) reaches the boxes in its rows, which their clips
    alone would not. `nested_frames` asserts each kept frame's rounding
    is there, and the debug check compares `nested` and `round`.
  - A scroller or clipping framed lane in a rounded lane row is checked
    after every paint against the frame's rounded clip on the page
    (`nested_frames`, one check per such frame), because rows move with
    none of their boxes painted again. One not held clear of the corners
    is refused (`NestedFrame`, naming the frame, the box that rounds the
    outer frame and this follow-up). One wider than the rounding box is
    refused too, though its square clip would draw right: that is more
    than needed, but loud.
  - The debug fresh paint holds the after-paint refusals to the
    update's. A box both refuse is reported once (`by_slot`).
- An svg drawing shapes under a rounded item clip is refused (`Svg`),
  which F3.2b missed. A path is tessellated into a mesh, and meshes draw
  under the scissor. Before this, the painter's panic was the only
  guard.
- Elements' chain count counts a rounded lane row's boundary as two
  links, the painter's rounded entry under its square one
  (`clip_chain.rs` `boundary`), so it stays an upper bound on the
  painter's chain. A box with a clip in a rounded lane row counts 3; a
  clipping framed lane held clear inside such a row gives its own boxes
  4, under `MAX_CLIP_CHAIN` (8). `assert_no_parent_in` also refuses
  giving back a frame's square entry while its rounded one is held.
- `assert_no_parent_in` returns early when nothing is given back.
  Measured over 1723 frames of a 2000-order dashboard scrolled down and
  back, every call took that return: at most 5 entries held, none given
  back, mean 70 ns, max 600 ns with the timer's own cost. Method: a
  release `vapors-bench` with a temporary timer around the call
  (`VAPORS_PARENT_TIMING`), not committed;
  ~/.cache/vapors-evidence/parent-check (NOTES.md, run.sh). The walk
  with entries given back was not reached, so it is not measured.

**Follow-up: frame entries chained to their outer frame's.** A frame
whose clip crosses a rounded frame's corners would need its entry under
the rounded frame's entry. A frame's entry is on the page and met with
its outer frames' rectangles, so the outer corners are lost. Trigger: the
first real page that needs a frame crossing a rounded corner. Until
then, `NestedFrame` refuses it.

## Questions for the lead and Tom

1. **Keep the scissor as the bounding rectangle** (recommended), or
   retire it in the same work (F3.1b whole, which needs images,
   surfaces and meshes on the table first)? Keeping it makes slices 1
   and 2 independent of F3.1b's image work.
2. **Slice 1 ships alone**, a PR with no visible change and the
   chains proven, or with slice 2? Recommended alone: it is where the
   kept-bytes and chain-length risks are, and it changes no pixels.
3. **Kept parts under a rounded item clip are painted again, never
   shifted** (F3.2b). With rows framed (slice 3) that stops mattering
   for lanes. Should shifting under a rounded clip be its own follow-up,
   or left until a measurement asks for it? Recommended: left.

## Risks

- **Chain length.** Item, then frame, then each nested frame. The
  corpus's longest is 2 today. Slice 1 measures again and cites it at
  `MAX_CLIP_CHAIN`.
- **Kept bytes.** A frame entry's index is in every item entry's
  `parent`. Plan item 2's rebuild on an index change applies; slice 1
  pins that a scroll moves no entry index.
- **Lane rows.** A lane's item clips are in the lane template's
  coordinates and its parent on the page. The shader applies each link's
  own transform. Slice 1's lane test draws an unscrolled lane, where
  both are the identity; a row-relative entry under a page one does not
  occur yet.
