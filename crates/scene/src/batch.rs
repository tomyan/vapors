//! Runs of the scene's items that one pipeline draws under one clip,
//! with the items that would change no visible pixel culled.

use std::borrow::Borrow;

use crate::display::{Item, Quad, Rect, Scene, Shadow};
use crate::shape::Shape;

/// Consecutive items of one kind under one clip: what a painter draws
/// with one pipeline before switching to the next.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    /// The clip, device pixels; `None` draws unclipped.
    pub clip: Option<Rect>,
    /// True for culled surfaces, kept so the painter can refuse them as
    /// it refuses clipped ones: nothing in the batch is drawn.
    pub culled: bool,
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
    /// Consecutive shapes.
    Shapes(Vec<Shape>),
}

/// A scene's batches for one target, and how many items were culled.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Batched {
    /// The batches, in painting order.
    pub batches: Vec<Batch>,
    /// Items left out, or (surfaces) kept in a batch marked culled,
    /// because their bounds miss the view under their clip.
    pub culled: u64,
}

impl Scene {
    /// The scene's batches for a target whose drawable area is `view`
    /// (device pixels), in order: [`batched`] over its items and clips.
    #[must_use]
    pub fn batches(&self, view: Rect) -> Batched {
        let clips = self.clips().iter().copied();
        batched(self.items().iter().zip(clips), view)
    }
}

/// The batches of `items`, each with its clip, for a target whose
/// drawable area is `view` (device pixels), in order. An item whose
/// [`Item::bounds`] miss `view` cut to its clip is culled: left out, so
/// the items either side of it can share a batch; a culled surface
/// instead keeps its place in a batch marked [`Batch::culled`], so the
/// painter refuses it as it refuses a clipped one.
pub fn batched<I: Borrow<Item>>(
    items: impl IntoIterator<Item = (I, Option<Rect>)>,
    view: Rect,
) -> Batched {
    let mut batched = Batched::default();
    for (item, clip) in items {
        let item = item.borrow();
        if !culled(item, clip, view) {
            batched.push(item, clip, false);
            continue;
        }
        batched.culled += 1;
        if let Item::Surface { .. } = item {
            batched.push(item, clip, true);
        }
    }
    batched
}

/// True when nothing `item` paints falls inside `view` cut to `clip`:
/// the rule [`batched`] culls by.
#[must_use]
pub fn culled(item: &Item, clip: Option<Rect>, view: Rect) -> bool {
    let region = clip.map_or(view, |clip| clip.intersection(view));
    region.is_empty() || item.bounds().is_disjoint(&region)
}

impl Batched {
    /// Adds `item` under `clip` to the last batch when it is of the same
    /// kind, clip and culling, else as a new batch.
    fn push(&mut self, item: &Item, clip: Option<Rect>, culled: bool) {
        if let Some(last) = self.batches.last_mut()
            && last.clip == clip
            && last.culled == culled
            && last.run.push(item)
        {
            return;
        }
        let mut run = Run::empty(item);
        run.push(item);
        self.batches.push(Batch { clip, culled, run });
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
            Item::Shape(_) => Run::Shapes(Vec::new()),
        }
    }

    /// Appends `item` when it is of this run's kind; false when not.
    fn push(&mut self, item: &Item) -> bool {
        match (self, item) {
            (Run::Shadows(shadows), Item::Shadow(shadow)) => shadows.push(*shadow),
            (Run::Quads(quads), Item::Quad(quad)) => quads.push(*quad),
            (Run::Shapes(shapes), Item::Shape(shape)) => shapes.push(*shape),
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
