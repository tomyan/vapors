//! The element tree: a [`Document`] implements [`vapors_dom::Dom`] over
//! HTML elements, computes their styles with `vapors-css`, lays them out
//! with Taffy and paints them to a [`vapors_scene::Scene`]. Nothing here
//! needs a GPU device. The plan and what each slice adds:
//! `docs/F2c.md`.

mod arena;
mod bidi;
mod chain;
mod check;
mod clip;
mod corners;
mod css_element;
mod document;
mod dom_impl;
mod error;
mod events;
mod focus;
mod html;
mod images;
mod img_checks;
mod inline_check;
mod keys;
mod layout;
mod node;
mod paint;
mod paint_inline;
mod paint_order;
mod paint_outline;
mod painted;
mod restyle;
mod restyle_pass;
mod scroll;
mod scroll_chains;
mod scrolling;
mod style;
mod tags;
mod thumb;
mod thumb_order;
mod tree;
mod update;

pub use document::Document;
pub use error::ElementError;
pub use events::Point;
pub use keys::Modifiers;
pub use layout::BoxRect;
pub use painted::Painted;
pub use style::{ALL_CURSORS, Cursor};
pub use update::Viewport;
