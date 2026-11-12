//! The a11y update a mirror takes (R6, `docs/plans/r6.md`): what changed
//! in a document's accessibility tree since the last take, as records
//! of the nodes new or changed, the nodes removed, placements for the
//! nodes new or moved, and the focused node when it moved. The types
//! alone, keyed by `NodeId`, so the platform's mirrors (the web page's
//! DOM, R7's native adapters) take them without the document behind
//! them; `vapors-elements` builds them from its tree and re-exports them.

use serde::{Deserialize, Serialize};
pub use vapors_dom::NodeId;

/// What changed since the last take.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AxUpdate {
    /// The first take, or every box was built: the mirror is rebuilt
    /// from `nodes`, which then holds every node and `placed` nothing.
    pub every: bool,
    /// Nodes no longer in the tree.
    pub removed: Vec<NodeId>,
    /// Nodes new, or whose record changed.
    pub nodes: Vec<AxMirrorNode>,
    /// Nodes new or moved, in an order a mirror applies one by one:
    /// a placement's `before` is already in place when it is reached.
    pub placed: Vec<AxPlace>,
    /// The focused node, when focus moved; `Some(None)` when it left.
    pub focus: Option<Option<NodeId>>,
}

impl AxUpdate {
    /// Whether the take found nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.every
            && self.removed.is_empty()
            && self.nodes.is_empty()
            && self.placed.is_empty()
            && self.focus.is_none()
    }
}

/// One node as a mirror shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct AxMirrorNode {
    /// The element.
    pub node: NodeId,
    /// The nearest node above it in the tree; `None` for a top node.
    pub parent: Option<NodeId>,
    /// Its role, the ARIA token.
    pub role: String,
    /// Its accessible name; empty for none.
    pub name: String,
    /// Its value, for a control that shows one, when not empty.
    pub value: Option<String>,
    /// Its accessible description, when not empty.
    pub description: Option<String>,
    /// Its states.
    pub states: AxStates,
    /// A slider's or spinbutton's range.
    pub range: Option<AxRange>,
    /// Whether it can take focus now (`focus_candidate`).
    pub focusable: bool,
    /// Its border box in CSS pixels from the border box of its parent
    /// node (the padding box of the parent's mirror element), plus the
    /// parent's scroll offset; from the viewport for a top node. A
    /// node's record does not change when a container above it
    /// scrolls.
    pub bounds: AxRect,
    /// Its scroll, when it is a scroll container: the mirror's element
    /// scrolls, and the nodes under it move with one write.
    pub scroll: Option<AxScroll>,
}

/// A scroll container's offset and extent, in CSS pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AxScroll {
    /// How far its content is scrolled, as applied: `[x, y]`.
    pub offset: [f32; 2],
    /// How far its content reaches from its padding box's corner,
    /// `[width, height]`: the box's size plus how far it can scroll, so
    /// the mirror's element scrolls as far.
    pub extent: [f32; 2],
}

/// A node's states; `None` where Chromium leaves the state out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxStates {
    /// It has focus.
    pub focused: bool,
    /// A disabled control, or `aria-disabled="true"`.
    pub disabled: bool,
    /// A checkbox's state, or `aria-checked`.
    pub checked: Option<bool>,
    /// `aria-expanded`.
    pub expanded: Option<bool>,
    /// `aria-selected="true"`.
    pub selected: bool,
    /// `aria-pressed`.
    pub pressed: Option<bool>,
}

impl AxStates {
    /// The states Chromium writes, as `name=value`, sorted.
    #[must_use]
    pub fn set(&self) -> Vec<String> {
        let mut set = Vec::new();
        for (name, state) in [
            ("checked", self.checked),
            ("expanded", self.expanded),
            ("pressed", self.pressed),
        ] {
            if let Some(state) = state {
                set.push(format!("{name}={state}"));
            }
        }
        for (name, state) in [
            ("disabled", self.disabled),
            ("focused", self.focused),
            ("selected", self.selected),
        ] {
            if state {
                set.push(format!("{name}=true"));
            }
        }
        set.sort();
        set
    }
}

/// A ranged control's bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxRange {
    /// `aria-valuemin`.
    pub min: f64,
    /// `aria-valuemax`.
    pub max: f64,
}

/// A border box in CSS pixels, from the origin its record names.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AxRect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// Where a node goes: under `parent`, before `before`, or last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxPlace {
    /// The node.
    pub node: NodeId,
    /// Its parent in the tree; `None` for a top node.
    pub parent: Option<NodeId>,
    /// The sibling it goes before, which the mirror holds by then;
    /// `None` to go last.
    pub before: Option<NodeId>,
}
