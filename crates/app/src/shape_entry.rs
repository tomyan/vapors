//! One entry of the painter's shape store (`shape_store.rs`): where it
//! came from (a crate's table or a document's run-time cache), the ref
//! items name it by, its stroke and its triangles on the GPU; and the
//! checks of an item against it.

use std::sync::Arc;

use vapors_renderer::MeshEntry;
use vapors_scene::{Shape, ShapeKind, ShapeRef};
use vapors_shapes::{Geometry, Paint, RuntimeShape, ShapeEntry};

use crate::shape_bounds::outside_bounds;
use crate::shape_error::ShapeError;

#[derive(Debug)]
pub(crate) struct Registered {
    pub(crate) source: Source,
    /// The ref items name it by.
    pub(crate) shape: ShapeRef,
    /// The stroke width its paint names; `None` fills it.
    pub(crate) stroke: Option<f32>,
    /// A mesh's triangles on the GPU, once uploaded.
    pub(crate) uploaded: Option<MeshEntry>,
}

/// Where an entry came from: a crate's table, or a document's cache.
#[derive(Debug)]
pub(crate) enum Source {
    Table(&'static ShapeEntry),
    Runtime(Arc<RuntimeShape>),
}

impl Source {
    /// Where it was written.
    pub(crate) fn site(&self) -> &str {
        match self {
            Self::Table(entry) => entry.site,
            Self::Runtime(shape) => shape.site(),
        }
    }

    /// What it draws.
    pub(crate) fn geometry(&self) -> Geometry<'_> {
        match self {
            Self::Table(entry) => entry.geometry,
            Self::Runtime(shape) => shape.geometry(),
        }
    }
}

/// An item drawing `entry`, registered as `shape`, with its kind from
/// the entry's geometry ([`kind_of`]): the way to build a shape item,
/// so its kind and its entry's cannot disagree.
#[must_use]
pub fn shape_item(
    entry: &ShapeEntry,
    shape: ShapeRef,
    transform: [f32; 6],
    colour: vapors_scene::Color,
    bounds: vapors_scene::Rect,
) -> Shape {
    Shape::new(shape, kind_of(&entry.geometry), transform, colour, bounds)
}

/// The kind `geometry` is drawn as: the one answer an item's kind is
/// held to.
#[must_use]
pub fn kind_of(geometry: &Geometry) -> ShapeKind {
    if geometry.is_closed_form() {
        ShapeKind::ClosedForm
    } else {
        ShapeKind::Mesh
    }
}

pub(crate) fn undrawable(
    item: &Shape,
    registered: &Registered,
    reason: vapors_renderer::ClosedFormError,
) -> ShapeError {
    ShapeError::Undrawable {
        shape: item.shape,
        site: registered.source.site().to_owned(),
        reason,
    }
}

/// Refuses `item` when what `registered` draws under it, stroked
/// `stroke` wide (a closed form's paint; a mesh holds its stroke),
/// lies outside its bounds, which culling reads.
pub(crate) fn check_bounds(
    registered: &Registered,
    stroke: Option<f32>,
    item: &Shape,
) -> Result<(), ShapeError> {
    let geometry = registered.source.geometry();
    match outside_bounds(&geometry, stroke, item) {
        None => Ok(()),
        Some(drawn) => Err(ShapeError::OutsideBounds {
            shape: item.shape,
            site: registered.source.site().to_owned(),
            drawn,
            bounds: item.bounds,
        }),
    }
}

/// The stroke width `paint` names; `None` fills.
pub(crate) fn stroke_of(paint: Paint) -> Option<f32> {
    match paint {
        Paint::Fill(_) => None,
        Paint::Stroke { width } => Some(width),
    }
}
