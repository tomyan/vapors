//! The element-tree builder that generated components call.
//!
//! A `.wisp` component compiles to calls on the [`Dom`] trait: create
//! elements and text, set attributes, properties and text, place and
//! remove nodes, and listen for events. The elements crate implements
//! it over the real tree (F2c); [`recording::RecordingDom`] implements
//! it for tests and records every call, which is how a test proves
//! that a signal change patches exactly one property.
//!
//! [`blocks`] holds the runtime for `{if}` and `{for}`: the generated
//! code hands it closures that build a branch or a row, and it decides
//! what to create, move, update or remove.

pub mod blocks;
mod event;
pub mod recording;
mod roots;
mod snippet;

pub use event::{Capture, Event, Handler, Pointer};
pub use roots::{BlockNodes, Root, current_nodes};
pub use snippet::{DynDom, Snippet};

/// Identifies a node in the tree. The [`Dom`] implementation decides
/// what the number means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

/// A property value: what `set_property` and `bind:` carry.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// A string property such as an input's `value`.
    Text(String),
    /// A boolean property such as `checked` or `disabled`.
    Bool(bool),
    /// A numeric property such as a range input's `valueAsNumber`.
    Number(f64),
}

/// The attribute a component's `mount` sets on each element it creates,
/// to the scope its `<style>` was added under (see [`Dom::add_style`]).
pub const SCOPE_ATTRIBUTE: &str = "data-wisp";

/// The tree a component mounts into.
pub trait Dom {
    /// Creates an element with the given tag, attached nowhere yet.
    fn create_element(&mut self, tag: &str) -> NodeId;
    /// Creates a text node, attached nowhere yet.
    fn create_text(&mut self, text: &str) -> NodeId;
    /// Sets (or replaces) an attribute.
    fn set_attribute(&mut self, node: NodeId, name: &str, value: &str);
    /// Replaces a text node's text.
    fn set_text(&mut self, node: NodeId, text: &str);
    /// Sets a property, as distinct from an attribute: an input's live
    /// value rather than its initial one.
    fn set_property(&mut self, node: NodeId, name: &str, value: PropertyValue);
    /// Places `child` as the last child of `parent`.
    fn append(&mut self, parent: NodeId, child: NodeId);
    /// Places `child` in `parent` before `before`, or last when `None`.
    /// Moving a node that is already placed is allowed.
    fn insert_before(&mut self, parent: NodeId, child: NodeId, before: Option<NodeId>);
    /// Detaches a node and its subtree.
    fn remove(&mut self, node: NodeId);
    /// Calls `handler` for each `event` (such as `click`) on the node.
    fn listen(&mut self, node: NodeId, event: &str, handler: Handler);
    /// Adds a component's `<style>` under `scope`, the value its
    /// elements carry in [`SCOPE_ATTRIBUTE`]: its rules match
    /// only those elements. Each mount calls it; a scope already added
    /// is not added again.
    fn add_style(&mut self, scope: &str, css: &str);
}
