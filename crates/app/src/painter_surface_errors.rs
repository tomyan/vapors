//! What the surface host refuses: mounting, a frame's surfaces, and a
//! renderer's failures, each naming the surface and the fix.

use std::fmt;

use super::PaintError;
use crate::surface::{SurfaceError, SurfaceSize};

/// Why [`ScenePainter::mount`](crate::ScenePainter::mount),
/// `unmount` or `invalidate` did nothing.
#[derive(Debug, PartialEq, Eq)]
pub enum MountError {
    /// A renderer is already mounted with this name.
    AlreadyMounted(String),
    /// No renderer is mounted with this name.
    NotMounted(String),
}

impl fmt::Display for MountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyMounted(name) => write!(
                f,
                "a renderer is already mounted for the gpu-surface {name:?}; unmount it first, or give the second surface another name"
            ),
            Self::NotMounted(name) => write!(
                f,
                "no renderer is mounted for the gpu-surface {name:?}; mount one with ScenePainter::mount, or check the name against the element's"
            ),
        }
    }
}

impl std::error::Error for MountError {}

/// A mounted surface's refusal, before the host adds its name.
#[derive(Debug)]
pub(crate) enum Refusal {
    DuplicateName,
    TooLarge { size: SurfaceSize, limit: u32 },
    Renderer(SurfaceError),
}

impl Refusal {
    /// The painter's error for the surface named `name`.
    pub(crate) fn named(self, name: &str) -> PaintError {
        let surface = name.to_owned();
        match self {
            Self::DuplicateName => PaintError::DuplicateName(surface),
            Self::TooLarge { size, limit } => PaintError::TooLarge {
                surface,
                width: size.width,
                height: size.height,
                limit,
            },
            Self::Renderer(error) => PaintError::Renderer { surface, error },
        }
    }
}

/// The message for a [`PaintError`].
pub(super) fn describe(error: &PaintError, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match error {
        PaintError::Atlas(error) => fmt::Display::fmt(error, f),
        PaintError::Unmounted(name) => unmounted(f, name),
        PaintError::DuplicateName(name) => duplicate(f, name),
        PaintError::TooLarge {
            surface,
            width,
            height,
            limit,
        } => too_large(f, surface, [*width, *height], *limit),
        PaintError::Renderer { surface, error } => renderer(f, surface, error),
    }
}

fn unmounted(f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
    write!(
        f,
        "the gpu-surface {name:?} has no renderer; mount one with ScenePainter::mount({name:?}, renderer) before the frame that paints it"
    )
}

fn duplicate(f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
    write!(
        f,
        "two gpu-surfaces in one frame are named {name:?}, and one renderer draws one surface; give each a different name"
    )
}

fn too_large(
    f: &mut fmt::Formatter<'_>,
    surface: &str,
    [width, height]: [u32; 2],
    limit: u32,
) -> fmt::Result {
    write!(
        f,
        "the gpu-surface {surface:?} needs a {width} x {height} backing texture, over this device's {limit}-pixel limit; make it smaller (or the scale lower)"
    )
}

fn renderer(f: &mut fmt::Formatter<'_>, surface: &str, error: &SurfaceError) -> fmt::Result {
    write!(
        f,
        "the renderer of the gpu-surface {surface:?} failed: {error}; the frame is not shown with stale or blank content, fix the renderer"
    )
}
