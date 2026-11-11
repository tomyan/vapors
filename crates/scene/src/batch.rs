//! Runs of the scene's items that one pipeline draws under one clip,
//! with the items that would change no visible pixel culled.

use crate::display::{Item, Quad, Rect, Scene, Shadow};

/// Consecutive items of one kind under one clip: what a painter draws
/// with one pipeline before switching to the next.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    /// The clip, device pixels; `None` draws unclipped.
    pub clip: Option<Rect>,
    /// The items.
    pub run: Run,
}

/// The items of a batch, by kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Run {
    /// Consecutive shadows.
    Shadows(Vec<Shadow>),
    /// Consecutive quads.
    Quads(Vec<Quad>),
    /// Consecutive text runs.
    Texts(Vec<Item>),
    /// Consecutive images.
    Images(Vec<Item>),
    /// Consecutive icons.
    Icons(Vec<Item>),
    /// Consecutive GPU surfaces.
    Surfaces(Vec<Item>),
}

/// A scene's batches for one target, and how many items were culled.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Batched {
    /// The batches, in painting order.
    pub batches: Vec<Batch>,
    /// Items left out, or (surfaces) kept under [`Rect::NOWHERE`],
    /// because their bounds miss the view under their clip.
    pub culled: u64,
}

impl Rect {
    /// A clip nothing passes: a culled surface's batch carries it, so the
    /// painter treats the surface as clipped out of view.
    pub const NOWHERE: Rect = Rect {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
}

impl Scene {
    /// The scene's batches for a target whose drawable area is `view`
    /// (device pixels), in order. An item whose [`Item::bounds`] miss
    /// `view` cut to its clip is culled: left out, so the items either
    /// side of it can share a batch; a culled surface instead keeps its
    /// place under [`Rect::NOWHERE`], so the painter refuses it as it
    /// would in view.
    #[must_use]
    pub fn batches(&self, view: Rect) -> Batched {
        let mut batched = Batched::default();
        for (item, &clip) in self.items().iter().zip(self.clips()) {
            if !culled(item, clip, view) {
                batched.push(item, clip);
                continue;
            }
            batched.culled += 1;
            if let Item::Surface { .. } = item {
                batched.push(item, Some(Rect::NOWHERE));
            }
        }
        batched
    }
}

/// True when nothing `item` paints falls inside `view` cut to `clip`.
fn culled(item: &Item, clip: Option<Rect>, view: Rect) -> bool {
    let region = clip.map_or(view, |clip| clip.intersection(view));
    region.is_empty() || item.bounds().is_disjoint(&region)
}

impl Batched {
    /// Adds `item` under `clip` to the last batch when it is of the same
    /// kind and clip, else as a new batch.
    fn push(&mut self, item: &Item, clip: Option<Rect>) {
        if let Some(last) = self.batches.last_mut()
            && last.clip == clip
            && last.run.push(item)
        {
            return;
        }
        let mut run = Run::empty(item);
        run.push(item);
        self.batches.push(Batch { clip, run });
    }
}

impl Run {
    /// An empty run of `item`'s kind.
    fn empty(item: &Item) -> Self {
        match item {
            Item::Shadow(_) => Run::Shadows(Vec::new()),
            Item::Quad(_) => Run::Quads(Vec::new()),
            Item::Text { .. } => Run::Texts(Vec::new()),
            Item::Image { .. } => Run::Images(Vec::new()),
            Item::Icon { .. } => Run::Icons(Vec::new()),
            Item::Surface { .. } => Run::Surfaces(Vec::new()),
        }
    }

    /// Appends `item` when it is of this run's kind; false when not.
    fn push(&mut self, item: &Item) -> bool {
        match (self, item) {
            (Run::Shadows(shadows), Item::Shadow(shadow)) => shadows.push(*shadow),
            (Run::Quads(quads), Item::Quad(quad)) => quads.push(*quad),
            (Run::Texts(items), Item::Text { .. })
            | (Run::Images(items), Item::Image { .. })
            | (Run::Icons(items), Item::Icon { .. })
            | (Run::Surfaces(items), Item::Surface { .. }) => items.push(*item),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "batch_cull_tests.rs"]
mod cull_tests;
