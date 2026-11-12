//! The 3D scene in `.wisp` (`docs/plans/3d-2.md`, row 4 and decision
//! 1): `<Scene>`, `<Group>`, `<Mesh>` and `<Camera>`, Rust components
//! with typed props, mounted through the compiler's component-tag path
//! like any child component. `<Scene>` and `<Group>` are built;
//! `<Mesh>` and `<Camera>` come in PR 4-3.
//!
//! ```html
//! <gpu-surface name="view"></gpu-surface>
//! <Scene scene={Some(scene)} surface={Some(surface)} camera={Some(eye)}>
//!   <Camera handle={Some(eye)} projection={Some(lens)} transform={back} />
//!   <Group transform={spin}>
//!     <Mesh geometry={Some(cube)} colour={red} />
//!   </Group>
//! </Scene>
//! ```
//!
//! Each component keeps one anchor element, a `div` with
//! `display: none`: no box, no paint, and no place in the accessibility
//! tree. The anchor is the component's identity in the document: its
//! children, and the `{if}` and `{for}` blocks among them, are placed in
//! it, and a node component finds its parent scene node from the
//! anchor it is placed in, at mount and at any later rebuild of a block.
//!
//! Each node is added to the scene at mount. `transform` (and a mesh's
//! `colour`) is a signal: a change writes it to that node alone and
//! invalidates the surface, so the next frame writes that node's
//! uniform (80 bytes; a group's subtree's), uploads no geometry, and
//! counts the scene's draws in `FrameCounts::surfaces`.
//!
//! A required prop is an `Option`, refused at mount when `None`, since
//! a parent's props struct takes every unset prop from `Default`. Markup
//! that is not a node component, inside `<Scene>` or `<Group>`, is
//! refused: the anchor would hide it.
//!
//! Disposing a component's owner (an `{if}` hiding it, a `{for}`
//! dropping its row, `unmount`) removes its node and the subtree under
//! it, and invalidates the surface. Keeping identity across a keyed
//! reorder and releasing geometry are row 5.

mod children;
mod component;
mod group;
mod node;
mod placement;
mod scene;

pub use group::{Group, GroupProps};
pub use scene::{Scene, SceneProps};
