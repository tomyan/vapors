//! Shapes: the SVG path-data parser, in one place for every crate that
//! asks "is this path data valid" (docs/plans/one-scene.md, 1.1). The
//! icon route (`vapors-text`) strokes what it parses; the element tree
//! refuses what it does not, so a path that passes the check rasterises.
//! No GPU and no text dependency.

mod path;
mod path_lex;

pub use path::{IconPath, PathError};
