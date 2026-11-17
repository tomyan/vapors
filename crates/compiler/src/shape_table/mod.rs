//! The crate's shape table (docs/plans/one-scene.md 1.3, S5a): every
//! literal `<path>`, `<rect>` and `<circle>` of the crate's templates
//! is parsed, refused when it cannot be drawn, classified, and added
//! once to one table, however many components write it. `wisp.rs`
//! includes the emitted `shapes.rs` beside it. Stroke entries and each
//! element's index are S5a's part 2.

mod collect;
mod render;

use std::collections::HashMap;

use vapors_shapes::RuntimeShape;

use crate::CompileError;
use crate::ast::Component;

/// The entries of every component compiled so far, each shape once.
#[derive(Default)]
pub(crate) struct Table {
    entries: Vec<RuntimeShape>,
    by_bytes: HashMap<Vec<u8>, u32>,
}

impl Table {
    /// Adds the shapes of `component`'s literal shape elements that the
    /// table does not hold.
    ///
    /// # Errors
    /// The first literal shape that cannot be drawn, naming its file
    /// and line.
    pub(crate) fn add(&mut self, component: &Component) -> Result<(), CompileError> {
        collect::elements(component, self)
    }

    /// The index of `shape`, a new one unless an equal one is held.
    fn index_of(&mut self, shape: RuntimeShape) -> u32 {
        let next = u32::try_from(self.entries.len()).expect("fewer than 2^32 shapes");
        let bytes = shape.canonical().bytes().to_vec();
        let index = *self.by_bytes.entry(bytes).or_insert(next);
        if index == next {
            self.entries.push(shape);
        }
        index
    }

    /// The `shapes.rs` text, or `None` for a crate with no literal shape.
    pub(crate) fn render(&self) -> Option<String> {
        (!self.entries.is_empty()).then(|| render::table(&self.entries))
    }
}
