//! Runs of the scene's items that one pipeline draws under one clip.

use crate::display::{Item, Quad, Rect, Scene, Shadow};

/// Consecutive items of one kind under one clip: what a painter draws
/// with one pipeline before switching to the next.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch<'a> {
    /// The clip, device pixels; `None` draws unclipped.
    pub clip: Option<Rect>,
    /// The items.
    pub run: Run<'a>,
}

/// The items of a batch, by kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Run<'a> {
    /// Consecutive shadows.
    Shadows(Vec<Shadow>),
    /// Consecutive quads.
    Quads(Vec<Quad>),
    /// Consecutive text runs.
    Texts(&'a [Item]),
}

impl Scene {
    /// The scene's batches, in order.
    #[must_use]
    pub fn batches(&self) -> Vec<Batch<'_>> {
        let (items, clips) = (self.items(), self.clips());
        let mut batches = Vec::new();
        let mut start = 0;
        for end in 1..=items.len() {
            let same =
                |a: usize, b: usize| kind(&items[a]) == kind(&items[b]) && clips[a] == clips[b];
            if end == items.len() || !same(start, end) {
                batches.push(Batch {
                    clip: clips[start],
                    run: Run::of(&items[start..end]),
                });
                start = end;
            }
        }
        batches
    }
}

impl<'a> Run<'a> {
    fn of(items: &'a [Item]) -> Self {
        match items[0] {
            Item::Shadow(_) => Run::Shadows(
                items
                    .iter()
                    .filter_map(|item| match item {
                        Item::Shadow(shadow) => Some(*shadow),
                        _ => None,
                    })
                    .collect(),
            ),
            Item::Quad(_) => Run::Quads(
                items
                    .iter()
                    .filter_map(|item| match item {
                        Item::Quad(quad) => Some(*quad),
                        _ => None,
                    })
                    .collect(),
            ),
            Item::Text { .. } => Run::Texts(items),
        }
    }
}

fn kind(item: &Item) -> u8 {
    match item {
        Item::Shadow(_) => 0,
        Item::Quad(_) => 1,
        Item::Text { .. } => 2,
    }
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
