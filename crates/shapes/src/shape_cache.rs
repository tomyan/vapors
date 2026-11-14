//! A document's run-time shapes (docs/plans/one-scene.md 1.5, S4a):
//! each distinct shape parsed, classified and, when it is a path,
//! tessellated once, and named by its index in the cache, which a
//! `ShapeRef::runtime` carries. Kept per document (lead, 2026-11-14):
//! S5b's process registry issues table `ShapeId`s, and this run-time
//! range stays apart from them. On a hash hit the canonical bytes are
//! compared, and a hit with other bytes is a miss, one more entry,
//! never a defect: a run-time parse does not rely on bit equality
//! across targets (one-scene 1.3).

use std::collections::HashMap;
use std::sync::Arc;

use crate::canonical::{Canonical, Paint};
use crate::classify::{Classified, classify};
use crate::path::IconPath;
use crate::runtime_shape::{RuntimeShape, TOLERANCE_PX};
use crate::tessellate::tessellate;

/// The run-time shapes of one document.
#[derive(Debug, Default)]
pub struct ShapeCache {
    shapes: Vec<Arc<RuntimeShape>>,
    by_hash: HashMap<u64, Vec<usize>>,
    tessellated: usize,
}

impl ShapeCache {
    /// The index of `path` painted by `paint`, drawn at `px_per_unit`
    /// device pixels per viewBox unit at most: an entry with its
    /// canonical form whose mesh is fine enough for that scale, or a
    /// new one. `site` names the shape in a refusal and in the store's
    /// messages.
    ///
    /// # Errors
    /// A scale that is not a finite number above zero, a stroke width
    /// [`Canonical::of`] refuses, or a path the tessellator refuses;
    /// each names the site.
    pub fn entry(
        &mut self,
        site: &str,
        path: &IconPath,
        paint: Paint,
        px_per_unit: f32,
    ) -> Result<u32, String> {
        if !(px_per_unit.is_finite() && px_per_unit > 0.0) {
            return Err(format!(
                "the shape at {site} is drawn at {px_per_unit} device pixels per unit, which is not a finite number above zero; this is a fault in the caller's transform, so report it with the page"
            ));
        }
        let canonical = Canonical::of(site, path, paint)?;
        let tolerance = TOLERANCE_PX / px_per_unit;
        if let Some(index) = self.found(&canonical, tolerance) {
            return Ok(index);
        }
        let shape = self.made(site, path, paint, canonical, tolerance)?;
        Ok(self.kept(shape))
    }

    /// Every shape the cache holds, by index; a store takes those past
    /// the ones it holds.
    #[must_use]
    pub fn shapes(&self) -> &[Arc<RuntimeShape>] {
        &self.shapes
    }

    /// How many paths were tessellated since the last take; the count
    /// restarts.
    pub fn take_tessellated(&mut self) -> usize {
        std::mem::take(&mut self.tessellated)
    }

    /// The newest entry with `canonical`'s bytes flattened within
    /// `tolerance`.
    fn found(&self, canonical: &Canonical, tolerance: f32) -> Option<u32> {
        let indices = self.by_hash.get(&canonical.hash())?;
        let index = indices.iter().rev().copied().find(|&index| {
            let shape = &self.shapes[index];
            shape.canonical().bytes() == canonical.bytes() && shape.tolerance() <= tolerance
        })?;
        Some(index_of(index))
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
            self.tessellated += 1;
            let mesh = tessellate(site, path, paint, tolerance)?;
            Some((mesh.triangles, mesh.tolerance))
        } else {
            None
        };
        Ok(RuntimeShape::new(site, canonical, classified, mesh))
    }

    fn kept(&mut self, shape: RuntimeShape) -> u32 {
        let index = self.shapes.len();
        self.by_hash
            .entry(shape.canonical().hash())
            .or_default()
            .push(index);
        self.shapes.push(Arc::new(shape));
        index_of(index)
    }
}

fn index_of(index: usize) -> u32 {
    u32::try_from(index).expect("a document caches fewer than 2^32 shapes")
}

#[cfg(test)]
#[path = "shape_cache_tests.rs"]
mod tests;
