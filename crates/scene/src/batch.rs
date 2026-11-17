//! Runs of the scene's items that one pipeline draws under one clip,
//! with the items that would change no visible pixel culled.

use std::borrow::Borrow;

use crate::display::{Item, Quad, Rect, Scene, Shadow};
use crate::shape::{Shape, ShapeKind};

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
    /// Consecutive closed-form shapes.
    Shapes(Vec<Shape>),
    /// Consecutive mesh shapes.
    Meshes(Vec<Shape>),
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
        batched.add(item.borrow(), clip, view);
    }
    batched
}

/// The rows of a lane one batch may hold, each item naming its row: a
/// batch's range in the painter's transform table holds this many
/// (vapors-renderer `instanced_bytes.rs` `assert_rows` refuses more),
/// and a longer run is split into batches.
pub const ROWS_PER_BATCH: usize = 128;

/// True when nothing `item` paints falls inside `view` cut to `clip`:
/// the rule [`batched`] culls by.
#[must_use]
pub fn culled(item: &Item, clip: Option<Rect>, view: Rect) -> bool {
    let region = clip.map_or(view, |clip| clip.intersection(view));
    region.is_empty() || item.bounds().is_disjoint(&region)
}

impl Batched {
    /// Adds `item` under `clip`, culled against `view` as [`batched`]
    /// culls: to the last batch when it can join it, else as a new one;
    /// counted culled, and kept only as a culled surface, when nothing of
    /// it falls in the view. Whether it was added to a batch. For items
    /// each culled in a view of its own (the rows of a lane, each in its
    /// own frame's coordinates).
    pub fn add(&mut self, item: &Item, clip: Option<Rect>, view: Rect) -> bool {
        self.add_culled(item, clip, culled(item, clip, view))
    }

    /// As [`Batched::add`], with whether `item` is culled worked out by the
    /// caller ([`culled`]), and batched under `clip`: for an item culled
    /// under its clip in its row's coordinates and batched under that clip
    /// moved into the lane's, so items of different rows under equal clips
    /// in their rows are not batched under one scissor; and so one test
    /// serves the caller's split and the batching.
    pub fn add_culled(&mut self, item: &Item, clip: Option<Rect>, is_culled: bool) -> bool {
        if !is_culled {
            self.push(item, clip, false);
            return true;
        }
        self.culled += 1;
        let surface = matches!(item, Item::Surface { .. });
        if surface {
            self.push(item, clip, true);
        }
        surface
    }

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
    /// How many items the run holds.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Run::Shadows(shadows) => shadows.len(),
            Run::Quads(quads) => quads.len(),
            Run::Shapes(shapes) | Run::Meshes(shapes) => shapes.len(),
            Run::Texts(items) | Run::Images(items) | Run::Icons(items) | Run::Surfaces(items) => {
                items.len()
            }
        }
    }

    /// Whether the run holds no item.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// An empty run of `item`'s kind.
    fn empty(item: &Item) -> Self {
        match item {
            Item::Shadow(_) => Run::Shadows(Vec::new()),
            Item::Quad(_) => Run::Quads(Vec::new()),
            Item::Text { .. } => Run::Texts(Vec::new()),
            Item::Image { .. } => Run::Images(Vec::new()),
            Item::Icon { .. } => Run::Icons(Vec::new()),
            Item::Surface { .. } => Run::Surfaces(Vec::new()),
            Item::Shape(Shape {
                kind: ShapeKind::ClosedForm,
                ..
            }) => Run::Shapes(Vec::new()),
            Item::Shape(Shape {
                kind: ShapeKind::Mesh,
                ..
            }) => Run::Meshes(Vec::new()),
        }
    }

    /// Appends `item` when it is of this run's kind; false when not.
    fn push(&mut self, item: &Item) -> bool {
        match (self, item) {
            (Run::Shadows(shadows), Item::Shadow(shadow)) => shadows.push(*shadow),
            (Run::Quads(quads), Item::Quad(quad)) => quads.push(*quad),
            (Run::Shapes(shapes), Item::Shape(shape)) if shape.kind == ShapeKind::ClosedForm => {
                shapes.push(*shape);
            }
            (Run::Meshes(meshes), Item::Shape(shape)) if shape.kind == ShapeKind::Mesh => {
                meshes.push(*shape);
            }
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

#[cfg(test)]
#[path = "batch_shape_tests.rs"]
mod shape_tests;
