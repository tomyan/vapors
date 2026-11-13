//! Shapes: the SVG path-data parser, in one place for every crate that
//! asks "is this path data valid" (docs/plans/one-scene.md, 1.1), and
//! the shape table a crate's build emits for its literal shapes (1.3).
//! The icon route (`vapors-text`) strokes what the parser parses; the
//! element tree refuses what it does not, so a path that passes the
//! check rasterises. No GPU and no text dependency.

mod geometry;
mod path;
mod path_lex;
mod shape_id;
mod table;
mod table_error;

pub use geometry::{Geometry, MeshVertex, check_geometry};
pub use path::{IconPath, PathError};
pub use table::{ShapeEntry, ShapeId, ShapeIndex, ShapeTable, TABLE_FORMAT, TableError};
