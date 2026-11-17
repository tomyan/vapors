//! The longest clip chain a box may paint under (flat-scene F3,
//! `docs/plans/flat-scene-f3.md` item 6): vapors-elements refuses a box
//! past it at paint (`clip_chain.rs` there), and the renderer's clip
//! shader takes it as its loop bound (`clip_table.rs`), one number for
//! both.

/// The most links a box's clip chain may have. Measured 2026-11-17: the
/// longest chain in the corpus is 2 links (`scrolling`, elements-corpus
/// `tests/clip_chains.rs`) and in the demos 2 (`scroll`, demos
/// `tests/clip_chains.rs`); 8 leaves room for
/// scroll containers nested a few deep and for F3.2's rounded clips,
/// each a link of its own. The clip shader's loop bound is this number
/// (vapors-renderer `clip_table.rs`).
pub const MAX_CLIP_CHAIN: usize = 8;
