//! The 3D scene in `.wisp` (`docs/plans/3d-2.md`, row 4 and decision
//! 1): `<Scene>`, `<Group>`, `<Mesh>` and `<Camera>`, Rust components
//! with typed props, mounted through the compiler's component-tag path
//! like any child component.
//!
//! ```html
//! <gpu-surface name="view"></gpu-surface>
//! <Scene scene={Some(scene)} surface={Some(surface)} camera={Some(eye)}>
//!   <Camera handle={Some(eye)} projection={Some(lens)} transform={back} />
//!   <Group transform={spin}>
//!     <Mesh geometry={Some(cube)} class="hull" class:lit={lit} />
//!   </Group>
//! </Scene>
//! ```
//!
//! Each component keeps one anchor element, tagged `scene`, `group`,
//! `mesh` or `camera` and made from a `StyleKey`: the one the compiler
//! passes as the `anchor` prop for a tag carrying `class`, `class:` or
//! `style` (`docs/plans/css-for-3d.md`, C1), or else the component's
//! own (`own_keys.rs`). The user-agent sheet gives those tags
//! `display: none`, so an anchor has no box, no paint and no place in
//! the accessibility tree. Each anchor's used style is a `SceneUsed`
//! (vapors-elements; C2), and a declaration its tag does not read is
//! refused at the update. A mesh's colour and visibility are its
//! anchor's computed `color` and `visibility`, inherited from the HTML
//! around the scene where no rule declares them (C3). The anchor is the component's identity in the document: its
//! children, and the `{if}` and `{for}` blocks among them, are placed in
//! it, and a node component finds its parent scene node from the
//! anchor it is placed in, at mount and at any later rebuild of a block.
//!
//! Each node is added to the scene at mount. `transform` is a signal,
//! and a mesh's appearance follows its anchor's restyle: a change
//! writes that node alone and invalidates the surface, so the next
//! frame writes that node's uniform (80 bytes; a transform: its
//! subtree's; a visibility change alone: none), uploads no geometry,
//! and counts the scene's draws in `FrameCounts::surfaces`.
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

mod camera;
mod children;
mod component;
mod group;
mod mesh;
mod node;
mod own_keys;
mod placement;
mod scene;

pub use camera::{Camera, CameraProps};
pub use group::{Group, GroupProps};
pub use mesh::{Mesh, MeshProps};
pub use scene::{Scene, SceneProps};
