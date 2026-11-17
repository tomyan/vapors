//! The longest clip chain a box may paint under (flat-scene F3,
//! `docs/plans/flat-scene-f3.md` item 6): vapors-elements refuses a box
//! past it at paint (`clip_chain.rs` there), and the renderer's clip
//! shader takes it as its loop bound (`clip_table.rs`), one number for
//! both.

/// The most links a box's clip chain may have. Measured 2026-11-17 at
/// each page's first update at one viewport (the corpus at 800x600, the
/// demos at 640x400): the longest chain in the corpus is 2 links
/// (`scrolling`) and in the demos 2 (`scroll`), each pinned by its
/// crate's `clip_chains.rs` against vapors-elements'
/// `MEASURED_CORPUS_CLIP_CHAIN` and `MEASURED_DEMOS_CLIP_CHAIN`. touche's
/// own pages are not in either set; it meets this limit as a refusal at
/// its next vapors pin bump. 8 leaves room for scroll containers nested
/// a few deep and for F3.2's rounded clips, each a link of its own. The
/// clip shader's loop bound is this number (vapors-renderer
/// `clip_table.rs`).
pub const MAX_CLIP_CHAIN: usize = 8;
