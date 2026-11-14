//! A document's run-time shapes (docs/plans/one-scene.md 1.5, S4a).
//! Each distinct shape is parsed, classified and, when it is a path,
//! tessellated once for its scale step: the scale rounded up to a
//! power of 2, so a zoom costs one tessellation per doubling, and
//! refused past [`MAX_PX_PER_UNIT`]. Kept per document (lead,
//! 2026-11-14): S5b's process registry issues table `ShapeId`s, and
//! this run-time range stays apart from them.
//!
//! Each entry counts its users (the elements that paint it). The last
//! release frees its slot for reuse, with the slot's generation raised
//! so a ref to the freed entry is refused. Live entries are therefore
//! at most the distinct (path, paint, stroke width, scale step) in
//! use. The slots changed since the painter last took them are kept
//! as a set, bounded by the slots, so a repaint takes only what
//! changed. On a hash hit the canonical bytes are compared, and other
//! bytes are a new entry, never a defect (one-scene 1.3).

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use crate::canonical::{Canonical, Paint};
use crate::classify::{Classified, classify};
use crate::path::IconPath;
use crate::runtime_shape::{RuntimeShape, TOLERANCE_PX};
use crate::tessellate::tessellate;

/// The largest scale step, device pixels per unit, a shape is
/// tessellated for. At 4096 an icon-sized circle (radius 8 units)
/// flattens within 0.1 px to about 1,300 segments, near 275 KB as a
/// stroke, and the WebGPU default `max_buffer_size` (256 MiB), which
/// the painter's entry buffer is held to, takes about 900 of those.
pub const MAX_PX_PER_UNIT: f32 = 4096.0;

/// An entry of a cache: its slot and the slot's generation, which goes
/// up each time the slot is freed, so a ref to a freed entry is told
/// apart from the entry that took its slot. The caller pairs it with
/// [`ShapeCache::document`] (`vapors_scene::RuntimeRef`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CacheRef {
    /// Its slot.
    pub index: u32,
    /// The slot's generation when it was made.
    pub generation: u32,
}

/// The run-time shapes of one document.
#[derive(Debug)]
pub struct ShapeCache {
    document: u32,
    slots: Vec<Slot>,
    free: Vec<u32>,
    by_hash: HashMap<u64, Vec<u32>>,
    changed: BTreeSet<u32>,
    tessellated: usize,
}

#[derive(Debug, Default)]
struct Slot {
    shape: Option<Arc<RuntimeShape>>,
    generation: u32,
    users: u32,
}

/// A slot's state for the painter's store: its entry and generation,
/// or `None` when it was freed.
pub type SlotChange = (u32, Option<(u32, Arc<RuntimeShape>)>);

impl ShapeCache {
    /// An empty cache for the document numbered `document`.
    #[must_use]
    pub fn new(document: u32) -> Self {
        Self {
            document,
            slots: Vec::new(),
            free: Vec::new(),
            by_hash: HashMap::new(),
            changed: BTreeSet::new(),
            tessellated: 0,
        }
    }

    /// The document this cache is.
    #[must_use]
    pub fn document(&self) -> u32 {
        self.document
    }

    /// One more use of `path` painted by `paint`, drawn at
    /// `px_per_unit` device pixels per unit at most: the live entry for
    /// its canonical form and scale step, or a new one. `site` names
    /// the shape in a refusal and in the store's messages.
    ///
    /// # Errors
    /// A scale that is not a finite number above zero, or one past
    /// [`MAX_PX_PER_UNIT`]; a stroke width [`Canonical::of`] refuses; a
    /// path the tessellator refuses. Each names the site. Nothing is
    /// used then.
    pub fn acquire(
        &mut self,
        site: &str,
        path: &IconPath,
        paint: Paint,
        px_per_unit: f32,
    ) -> Result<CacheRef, String> {
        let step = scale_step(site, px_per_unit)?;
        let canonical = Canonical::of(site, path, paint)?;
        let tolerance = TOLERANCE_PX / step;
        let index = if let Some(index) = self.found(&canonical, tolerance) {
            index
        } else {
            let shape = self.made(site, path, paint, canonical, tolerance)?;
            self.kept(shape)
        };
        self.slots[index as usize].users += 1;
        Ok(self.ref_of(index))
    }

    /// One use of `at` fewer; the last frees its slot.
    ///
    /// # Panics
    /// When `at` is not a live entry of this cache: a release without an
    /// acquire is a fault in the caller's use counts.
    pub fn release(&mut self, at: CacheRef) {
        let document = self.document;
        let Some(slot) = self.live_slot(at) else {
            panic!(
                "released {at:?}, which is not a live entry of document {document}; the caller's use counts are wrong"
            );
        };
        slot.users -= 1;
        if slot.users == 0 {
            self.freed(at.index);
        }
    }

    /// The live entry `at` names.
    #[must_use]
    pub fn get(&self, at: CacheRef) -> Option<&Arc<RuntimeShape>> {
        let slot = self.slots.get(at.index as usize)?;
        slot.shape
            .as_ref()
            .filter(|_| slot.generation == at.generation)
    }

    /// Each slot changed since the last take, with its entry now; the
    /// set restarts.
    pub fn take_changes(&mut self) -> Vec<SlotChange> {
        let changed = std::mem::take(&mut self.changed);
        changed
            .into_iter()
            .map(|index| {
                let slot = &self.slots[index as usize];
                let shape = slot
                    .shape
                    .as_ref()
                    .map(|s| (slot.generation, Arc::clone(s)));
                (index, shape)
            })
            .collect()
    }

    /// How many entries are live, and how many slots the cache holds.
    #[must_use]
    pub fn counts(&self) -> (usize, usize) {
        let live = self.slots.iter().filter(|s| s.shape.is_some()).count();
        (live, self.slots.len())
    }

    /// How many paths were tessellated since the last take; the count
    /// restarts.
    pub fn take_tessellated(&mut self) -> usize {
        std::mem::take(&mut self.tessellated)
    }

    /// The live entry with `canonical`'s bytes at `tolerance`.
    fn found(&self, canonical: &Canonical, tolerance: f32) -> Option<u32> {
        let indices = self.by_hash.get(&canonical.hash())?;
        indices.iter().copied().find(|&index| {
            let shape = self.slots[index as usize]
                .shape
                .as_ref()
                .expect("by_hash holds live slots");
            shape.canonical().bytes() == canonical.bytes() && same_step(shape, tolerance)
        })
    }

    fn made(
        &mut self,
        site: &str,
        path: &IconPath,
        paint: Paint,
        canonical: Canonical,
        tolerance: f32,
    ) -> Result<RuntimeShape, String> {
        let classified = classify(path, paint);
        let mesh = if classified == Classified::Path {
            let mesh = tessellate(site, path, paint, tolerance)?;
            self.tessellated += 1;
            Some((mesh.triangles, mesh.tolerance))
        } else {
            None
        };
        Ok(RuntimeShape::new(site, canonical, classified, mesh))
    }

    /// `shape` in a free slot, or a new one.
    fn kept(&mut self, shape: RuntimeShape) -> u32 {
        let index = self.free.pop().unwrap_or_else(|| {
            self.slots.push(Slot::default());
            index_of(self.slots.len() - 1)
        });
        self.by_hash
            .entry(shape.canonical().hash())
            .or_default()
            .push(index);
        self.slots[index as usize].shape = Some(Arc::new(shape));
        self.changed.insert(index);
        index
    }

    fn freed(&mut self, index: u32) {
        let slot = &mut self.slots[index as usize];
        let shape = slot.shape.take().expect("a live slot");
        slot.generation += 1;
        let hash = shape.canonical().hash();
        let filed = self.by_hash.get_mut(&hash).expect("a live slot is filed");
        filed.retain(|&filed| filed != index);
        if filed.is_empty() {
            self.by_hash.remove(&hash);
        }
        self.free.push(index);
        self.changed.insert(index);
    }

    fn live_slot(&mut self, at: CacheRef) -> Option<&mut Slot> {
        let slot = self.slots.get_mut(at.index as usize)?;
        (slot.generation == at.generation && slot.users > 0).then_some(slot)
    }

    fn ref_of(&self, index: u32) -> CacheRef {
        CacheRef {
            index,
            generation: self.slots[index as usize].generation,
        }
    }
}

/// `px_per_unit` rounded up to a power of 2.
///
/// # Errors
/// A scale that is not a finite number above zero, or past
/// [`MAX_PX_PER_UNIT`], naming `site`.
fn scale_step(site: &str, px_per_unit: f32) -> Result<f32, String> {
    if !(px_per_unit.is_finite() && px_per_unit > 0.0) {
        return Err(format!(
            "the shape at {site} is drawn at {px_per_unit} device pixels per unit, which is not a finite number above zero; this is a fault in the caller's transform, so report it with the page"
        ));
    }
    if px_per_unit > MAX_PX_PER_UNIT {
        return Err(format!(
            "the shape at {site} is drawn at {px_per_unit} device pixels per unit, past the {MAX_PX_PER_UNIT} a run-time shape is tessellated for; draw it smaller, or as an image"
        ));
    }
    Ok(2f32.powf(px_per_unit.log2().ceil()))
}

/// Whether `shape` serves the scale step `tolerance` is for: a closed
/// form, which is exact, serves every one.
#[allow(clippy::float_cmp)] // exact: both are TOLERANCE_PX over a power of 2
fn same_step(shape: &RuntimeShape, tolerance: f32) -> bool {
    shape.geometry().is_closed_form() || shape.tolerance() == tolerance
}

fn index_of(index: usize) -> u32 {
    u32::try_from(index).expect("a document caches fewer than 2^32 shapes")
}

#[cfg(test)]
#[path = "shape_cache_tests.rs"]
mod tests;
