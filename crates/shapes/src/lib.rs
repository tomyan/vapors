//! Shapes: the SVG path-data parser, in one place for every crate that
//! asks "is this path data valid" (docs/plans/one-scene.md, 1.1), and
//! the shape table a crate's build emits for its literal shapes (1.3).
//! The icon route (`vapors-text`) strokes what the parser parses; the
//! element tree refuses what it does not, so a path that passes the
//! check rasterises. No GPU and no text dependency.

mod canonical;
mod classify;
mod classify_near;
mod geometry;
mod mesh;
mod mesh_edges;
mod path;
mod path_lex;
mod path_verb;
mod segment;
mod shape_id;
mod table;
mod table_error;
mod tessellate;
mod tessellate_path;

pub use canonical::{Canonical, FillRule, Paint, hash_bytes};
pub use classify::{Classified, classify};
pub use geometry::{Geometry, check_geometry};
pub use mesh::{FAR_EDGE, MeshVertex, check_mesh};
pub use path::{IconPath, PathError};
pub use segment::{Pen, Segment, non_finite, to_zeno};
pub use table::{ShapeEntry, ShapeId, ShapeIndex, ShapeTable, TABLE_FORMAT, TableError};
pub use tessellate::{Mesh, tessellate};
