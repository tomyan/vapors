//! What a [`SceneRenderer`](crate::SceneRenderer) reports about its own
//! lifetime, shared with whoever made it (as the racer's `TrackLog`):
//! that its drop ran, and for each kind of GPU resource it holds, how
//! many it made and how many it freed. Each resource carries a
//! [`Tracked`] record, dropped after the resource itself, so a resource
//! leaked (kept in an `Rc`, a static, or forgotten) is counted as made
//! and never as freed.

use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// A kind of GPU resource a renderer holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    /// The render pipeline and its bind group layouts.
    Pipeline,
    /// The camera's uniform buffer and bind group.
    Camera,
    /// The mesh uniform slots' buffer and bind group.
    Slots,
    /// The depth texture's view.
    Depth,
    /// One uploaded piece of geometry: its vertex and index buffers.
    Geometry,
}

impl Resource {
    /// Every kind.
    pub const ALL: [Self; 5] = [
        Self::Pipeline,
        Self::Camera,
        Self::Slots,
        Self::Depth,
        Self::Geometry,
    ];
}

/// Shared by a renderer and the application through
/// [`SceneRenderer::log`](crate::SceneRenderer::log).
#[derive(Debug, Default)]
pub struct RendererLog {
    dropped: Cell<bool>,
    made: [Cell<u64>; 5],
    freed: [Cell<u64>; 5],
}

impl RendererLog {
    /// Whether the renderer's drop has run: an unmounted surface's
    /// renderer's does, once the caller of `ElementsApp::unmount` drops
    /// it. What it freed is [`RendererLog::freed`].
    #[must_use]
    pub fn dropped(&self) -> bool {
        self.dropped.get()
    }

    /// How many resources of `kind` the renderer has made.
    #[must_use]
    pub fn made(&self, kind: Resource) -> u64 {
        self.made[kind as usize].get()
    }

    /// How many resources of `kind` have been freed.
    #[must_use]
    pub fn freed(&self, kind: Resource) -> u64 {
        self.freed[kind as usize].get()
    }

    pub(crate) fn drop_renderer(&self) {
        self.dropped.set(true);
    }

    fn count(cells: &[Cell<u64>; 5], kind: Resource) {
        let cell = &cells[kind as usize];
        cell.set(cell.get() + 1);
    }
}

/// A resource and its record in the log: made when wrapped, freed when
/// dropped, after the resource (fields drop in declaration order).
pub(crate) struct Tracked<T> {
    value: T,
    /// Held for its drop, which counts the resource freed.
    _record: Record,
}

struct Record {
    log: Rc<RendererLog>,
    kind: Resource,
}

impl<T> Tracked<T> {
    pub(crate) fn new(value: T, log: &Rc<RendererLog>, kind: Resource) -> Self {
        RendererLog::count(&log.made, kind);
        let record = Record {
            log: Rc::clone(log),
            kind,
        };
        Self {
            value,
            _record: record,
        }
    }
}

impl Drop for Record {
    fn drop(&mut self) {
        RendererLog::count(&self.log.freed, self.kind);
    }
}

impl<T> Deref for Tracked<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for Tracked<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
