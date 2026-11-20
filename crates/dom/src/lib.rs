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

mod appearance;
pub mod blocks;
mod class_names;
mod compiled_sheet;
mod dispatcher;
mod event;
mod event_key;
mod event_pointer;
mod key_winners;
mod listener;
mod listener_result;
mod modifiers;
pub mod recording;
mod required;
mod roots;
mod scene_hits;
mod snippet;
mod style_key;
mod style_key_refusals;
mod window_listeners;

pub use appearance::{Appearance, AppearanceSink};
pub use class_names::class_names;
pub use compiled_sheet::CompiledSheet;
pub use dispatcher::{DispatchRefused, Dispatcher, Listeners, run_along};
pub use event::Event;
pub use event_pointer::{Capture, Pointer, PointerKind};
pub use key_winners::{
    BaseWinner, DeclarationRef, KeyWinners, PropertyRef, RuleRef, SheetRef, Winners,
};
pub use listener::{Handler, Listener};
pub use listener_result::{ListenerResult, handle, handle_without_event, set_error_report};
pub use modifiers::Modifiers;
pub use required::Required;
pub use roots::{BlockNodes, Root, current_nodes};
pub use scene_hits::{HitAsk, HitSource};
pub use snippet::{Args, DynDom, Renderable, Snippet};
pub use style_key::{MAX_TOGGLES, StyleKey};
pub use style_key_refusals::PRESENTATION as PRESENTATION_ATTRIBUTES;
pub use style_key_refusals::SVG_TAGS;
pub use window_listeners::{WINDOW_EVENTS, WindowListener, WindowListeners};

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

/// Whether writing `new` over `current` changes nothing. Numbers compare
/// by `to_bits`, not `==`: `0.0 == -0.0` would skip a visible change, and
/// a NaN is only equal to the NaN with the same bits, which is not a
/// reason to skip the write either.
fn property_unchanged(current: &PropertyValue, new: &PropertyValue) -> bool {
    match (current, new) {
        (PropertyValue::Number(a), PropertyValue::Number(b)) => {
            !a.is_nan() && !b.is_nan() && a.to_bits() == b.to_bits()
        }
        _ => current == new,
    }
}

/// The attribute a component's `mount` sets on each element it creates,
/// to the scope its `<style>` was added under (see [`Dom::add_style`]).
pub const SCOPE_ATTRIBUTE: &str = "data-wisp";

/// The tree a component mounts into; its listeners are reachable for a
/// component's own events ([`Dispatcher`]).
pub trait Dom: Listeners {
    /// Creates an element with the given tag, attached nowhere yet.
    fn create_element(&mut self, tag: &str) -> NodeId;
    /// Creates a template's element from its [`StyleKey`], attached
    /// nowhere yet: the key's tag, with the attributes of
    /// [`StyleKey::attributes`] set. From here on [`Dom::set_attribute`]
    /// refuses those attributes on it ([`StyleKey::refusal`]), so the
    /// key stays true of the element; a `class:` directive toggles
    /// through [`Dom::toggle_classes`]. A key with a
    /// [`StyleKey::shape_refusal`] is refused.
    ///
    /// A refusal here and in the calls below is recorded, naming the
    /// site and the fix, and changes nothing: `vapors_elements` returns
    /// it from every update while the element is in the tree, and
    /// `RecordingDom` records it as a call. Neither panics.
    fn create_keyed_element(&mut self, key: &'static StyleKey) -> NodeId;
    /// Turns each named `class:` directive of a keyed element on or
    /// off, and sets its `class` to [`StyleKey::class_attribute`].
    /// Refuses a name the key does not list, and an element created
    /// without a key, naming the fix.
    fn toggle_classes(&mut self, node: NodeId, toggles: &[(&str, bool)]);
    /// The key `node` was created from; `None` for an element made by
    /// [`Dom::create_element`] and for a text node.
    fn style_key(&self, node: NodeId) -> Option<&'static StyleKey>;
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
    /// Calls `handler` for each `keydown` or `keyup` the page gets
    /// (`<vapors:window>`), after the listeners on the event's target and
    /// its ancestors ran, unless one stopped propagation. Another event
    /// panics ([`WindowListeners::add`]).
    fn listen_window(&mut self, event: &str, handler: Handler) -> WindowListener;
    /// Removes a window listener, when its component unmounts.
    fn unlisten_window(&mut self, listener: WindowListener);
    /// Adds a component's `<style>` (`sheet`, which only generated code
    /// makes) under its scope, the value its elements carry in
    /// [`SCOPE_ATTRIBUTE`]: its rules match only those elements. Each
    /// mount calls it; a scope already added is not added again.
    fn add_style(&mut self, sheet: CompiledSheet<'_>);
    /// Adds a `Styles::Global` component's `<style>` as an author sheet
    /// named `sheet` (the component's crate and file), unscoped: its
    /// rules may match any element. Each mount calls it; a sheet already
    /// added is not added again. It stays after its component unmounts
    /// and still styles every element it matches, as Svelte's `:global`
    /// styles do; `compile_dir_global` is for the layout corpus alone,
    /// which mounts one fixture per document.
    fn add_global_style(&mut self, sheet: CompiledSheet<'_>);
    /// Names events a component dispatches itself ([`Dispatcher`]), so a
    /// tree that refuses listeners for events it never fires takes
    /// listeners for these.
    fn declare_events(&mut self, names: &[&str]);
    /// Calls `sink` with the appearance of `anchor`, a 3D scene
    /// component's `mesh` anchor: at the first update after the watch
    /// and at each update that changes it, until `anchor` leaves the
    /// tree ([`AppearanceSink`]). A sink's error refuses that update, and
    /// the next update reports the appearance again. Refused on any node
    /// but a `mesh`, and on a mesh already watched.
    fn watch_appearance(&mut self, anchor: NodeId, sink: AppearanceSink);
    /// Asks `source` for the anchor under the pointer whenever a hit
    /// test lands on the `gpu-surface` named `surface`, until `anchor`,
    /// a 3D scene component's `scene` anchor, leaves the tree
    /// ([`HitSource`]). Refused on any node but a `scene`, on a scene
    /// already watched, and for a surface another scene is watched on.
    fn watch_hits(&mut self, anchor: NodeId, surface: &str, source: HitSource);
    /// Says what `anchor`'s scene draws under its surface may have
    /// changed (a node moved, hid or changed geometry), so the next update
    /// tests the hover again when the pointer is over that surface.
    fn hits_moved(&mut self, anchor: NodeId);
    /// A text node's text; `None` for an element. The `_if_changed`
    /// setters read this: `None` means absent or the wrong kind, and
    /// they then call the plain setter, which refuses the wrong kind
    /// loudly.
    fn current_text(&self, node: NodeId) -> Option<&str>;
    /// An element's attribute; `None` when it has none of that name or
    /// `node` is a text node (absent or wrong kind: the `_if_changed`
    /// setter then calls the plain setter, which refuses the wrong kind
    /// loudly).
    fn current_attribute(&self, node: NodeId, name: &str) -> Option<&str>;
    /// An element's property as it stands now, including what the
    /// person's own edits wrote (a typed `value`, a toggled `checked`);
    /// `None` when it has none of that name or `node` is a text node
    /// (absent or wrong kind, handled as for [`Dom::current_attribute`]).
    fn current_property(&self, node: NodeId, name: &str) -> Option<&PropertyValue>;

    /// [`Dom::set_text`] unless the node already holds `text`. The
    /// bindings the compiler generates call this, so an equal signal
    /// write changes nothing; a direct `set_text` always writes.
    fn set_text_if_changed(&mut self, node: NodeId, text: &str) {
        if self.current_text(node) != Some(text) {
            self.set_text(node, text);
        }
    }
    /// [`Dom::set_attribute`] unless the element already holds `value`
    /// under `name`; called by the generated bindings. An attribute the
    /// element's key fixes always goes to `set_attribute`, which
    /// refuses it, so the refusal does not depend on the value.
    fn set_attribute_if_changed(&mut self, node: NodeId, name: &str, value: &str) {
        let fixed = (self.style_key(node)).is_some_and(|key| key.refusal(name).is_some());
        if fixed || self.current_attribute(node, name) != Some(value) {
            self.set_attribute(node, name, value);
        }
    }
    /// [`Dom::set_property`] unless the element's current property is
    /// `value`: numbers compare by bit pattern, so a NaN number and a
    /// change between 0.0 and -0.0 are written (a range formats them
    /// "NaN", "0" and "-0"); other kinds compare by `PartialEq`. Called
    /// by the generated bindings.
    fn set_property_if_changed(&mut self, node: NodeId, name: &str, value: PropertyValue) {
        if !self
            .current_property(node, name)
            .is_some_and(|current| property_unchanged(current, &value))
        {
            self.set_property(node, name, value);
        }
    }
}
