//! A document's run-time shapes (one-scene S4a) and which elements use
//! them: each element's shapes are acquired when it paints and released
//! when it paints others or leaves the tree, so an entry lives while an
//! element paints it (the cache frees one with no users). The painter
//! takes the slots that changed through the document's `Painted`.

use vapors_dom::NodeId;
use vapors_scene::RuntimeRef;
use vapors_shapes::{CacheRef, IconPath, Paint, ShapeCache};

use crate::node_map::NodeMap;

/// One shape an element paints: where it was written, its path, its
/// paint, and the device pixels per unit it is drawn at.
pub(crate) type ShapeSpec<'a> = (&'a str, &'a IconPath, Paint, f32);

/// The cache and each element's uses of it.
#[derive(Debug)]
pub(crate) struct DocumentShapes {
    pub(crate) cache: ShapeCache,
    uses: NodeMap<Vec<CacheRef>>,
}

impl DocumentShapes {
    /// No shapes yet, for a new document.
    pub(crate) fn new() -> Self {
        Self {
            cache: ShapeCache::new(next_document()),
            uses: NodeMap::default(),
        }
    }

    /// `node` now paints `specs`: each acquired, then the shapes it
    /// painted before released, so a shape it keeps is never freed in
    /// between; each one's ref.
    ///
    /// # Errors
    /// What the cache refuses for a spec; `node` then keeps the shapes
    /// it had.
    pub(crate) fn use_for(
        &mut self,
        node: NodeId,
        specs: &[ShapeSpec<'_>],
    ) -> Result<Vec<RuntimeRef>, String> {
        let mut acquired = Vec::with_capacity(specs.len());
        for &(site, path, paint, px_per_unit) in specs {
            match self.cache.acquire(site, path, paint, px_per_unit) {
                Ok(at) => acquired.push(at),
                Err(reason) => {
                    for at in acquired {
                        self.cache.release(at);
                    }
                    return Err(reason);
                }
            }
        }
        let refs = acquired.iter().map(|&at| self.runtime_ref(at)).collect();
        let old = self.uses.insert(node, acquired).unwrap_or_default();
        for at in old {
            self.cache.release(at);
        }
        Ok(refs)
    }

    /// `node` left the tree: its shapes are released.
    pub(crate) fn release(&mut self, node: NodeId) {
        for at in self.uses.remove(&node).unwrap_or_default() {
            self.cache.release(at);
        }
    }

    /// The live entry `at` names, when it is this document's.
    pub(crate) fn get(
        &self,
        at: RuntimeRef,
    ) -> Option<&std::sync::Arc<vapors_shapes::RuntimeShape>> {
        let mine = at.document == self.cache.document();
        let at = CacheRef {
            index: at.index,
            generation: at.generation,
        };
        self.cache.get(at).filter(|_| mine)
    }

    fn runtime_ref(&self, at: CacheRef) -> RuntimeRef {
        RuntimeRef {
            document: self.cache.document(),
            index: at.index,
            generation: at.generation,
        }
    }
}

/// A number no other document in this process has.
fn next_document() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
#[path = "document_shapes_tests.rs"]
mod tests;
