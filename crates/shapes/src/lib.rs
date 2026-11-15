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
mod geometry_box;
mod mesh;
mod mesh_check;
mod mesh_edges;
mod mesh_normals;
mod mesh_slivers;
mod mesh_wedge;
mod path;
mod path_lex;
mod path_verb;
mod runtime_shape;
mod segment;
mod shape_cache;
mod shape_id;
mod table;
mod table_error;
mod tessellate;
mod tessellate_path;

pub use canonical::{Canonical, FillRule, Paint, hash_bytes, paint_of};
pub use classify::{Classified, classify};
pub use geometry::{Geometry, MIN_RADIUS, check_geometry};
pub use geometry_box::transformed_box;
pub use mesh::{CONVEX, INSIDE, MeshCorner, MeshTriangle, REFLEX};
pub use mesh_check::check_mesh;
pub use mesh_edges::mesh_triangles;
pub use path::{IconPath, PathError};
pub use runtime_shape::{RuntimeShape, TOLERANCE_PX};
pub use segment::{Pen, Segment, non_finite, to_zeno};
pub use shape_cache::{CacheRef, MAX_PX_PER_UNIT, ShapeCache, SlotChange};
pub use table::{ShapeEntry, ShapeId, ShapeIndex, ShapeTable, TABLE_FORMAT, TableError};
pub use tessellate::{Mesh, tessellate};
