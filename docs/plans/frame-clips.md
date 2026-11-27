# Frame clips on the clip table (before F3.2c)

Status: proposal, for the lead and Tom. Nothing here is built.

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
   - A rounded scroller (`overflow: auto` with a radius) is no longer
     refused: its frame clip rounds.
   - Tests: a framed lane and a scroller under a rounded clip, painted
     and hit, and a Chromium fixture of a rounded scroller.
3. **Lane decline retired, F3.2c measured again.**
   - `LaneFrameRoundedClip` is removed, so a rounded list frames its
     rows.
   - The insert counts are measured again: expected near the square
     list's 6. A 4000-row rounded list updates.
   - Then F3.2c's branch rebases and ships.

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
- **Lane rows.** Each row's entries are in the row's transform and their
  parent in the container's. The shader applies each link's own
  transform, which slice 1's lane test pins.
