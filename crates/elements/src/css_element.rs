//! The tree as the CSS crate's selector matcher sees it: elements only,
//! text nodes skipped except where `:empty` counts them.

use vapors_css::selector::Element as CssElement;
use vapors_css::tree::States;
use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::node::{Element, Kind};

/// An element of the arena, as a cheap handle.
#[derive(Clone, Copy)]
pub(crate) struct ElementRef<'a> {
    pub(crate) arena: &'a Arena,
    pub(crate) id: NodeId,
}

impl PartialEq for ElementRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<'a> ElementRef<'a> {
    pub(crate) fn new(arena: &'a Arena, id: NodeId) -> Self {
        Self { arena, id }
    }

    fn data(&self) -> &'a Element {
        self.arena
            .get(self.id, "css")
            .element()
            .expect("an ElementRef is made only for elements")
    }

    /// The first element at or after `node` following `step`.
    fn element_from(
        &self,
        mut node: Option<NodeId>,
        step: fn(&Self, NodeId) -> Option<NodeId>,
    ) -> Option<Self> {
        while let Some(id) = node {
            if self.arena.get(id, "css").element().is_some() {
                return Some(Self::new(self.arena, id));
            }
            node = step(self, id);
        }
        None
    }

    fn next_of(&self, id: NodeId) -> Option<NodeId> {
        self.arena.get(id, "css").next
    }

    fn previous_of(&self, id: NodeId) -> Option<NodeId> {
        self.arena.get(id, "css").previous
    }
}

impl CssElement for ElementRef<'_> {
    fn tag_name(&self) -> &str {
        &self.data().tag
    }

    fn id(&self) -> Option<&str> {
        self.attribute("id")
    }

    fn has_class(&self, class: &str) -> bool {
        self.class_names().contains(&class)
    }

    fn class_names(&self) -> Vec<&str> {
        self.attribute("class")
            .map(|classes| classes.split_ascii_whitespace().collect())
            .unwrap_or_default()
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.data().attributes.get(name).map(String::as_str)
    }

    fn parent(&self) -> Option<Self> {
        let parent = self.arena.get(self.id, "css").parent?;
        Some(Self::new(self.arena, parent))
    }

    fn previous_sibling(&self) -> Option<Self> {
        self.element_from(self.previous_of(self.id), Self::previous_of)
    }

    fn next_sibling(&self) -> Option<Self> {
        self.element_from(self.next_of(self.id), Self::next_of)
    }

    fn first_child(&self) -> Option<Self> {
        self.element_from(self.arena.get(self.id, "css").first_child, Self::next_of)
    }

    /// No element children and no text with characters in it: the empty
    /// text nodes `{if}` and `{for}` use as anchors do not count.
    fn is_empty(&self) -> bool {
        self.arena.children(self.id).iter().all(|child| {
            matches!(&self.arena.get(*child, "css").kind, Kind::Text(text) if text.is_empty())
        })
    }

    /// The pointer's flags, and for a button or an input whether it is
    /// enabled (`:enabled` and `:disabled` are set on form controls
    /// only), and for a checkbox whether it is checked.
    fn states(&self) -> States {
        let data = self.data();
        if !matches!(data.tag.as_str(), "button" | "input") {
            return data.states;
        }
        let control = if data.attributes.contains_key("disabled") {
            States::DISABLED
        } else {
            States::ENABLED
        };
        let checked = match crate::checkbox::checked_of(data) {
            Some(true) => States::CHECKED,
            Some(false) | None => States::NONE,
        };
        data.states.union(control).union(checked)
    }

    fn container_size(&self) -> Option<(f32, f32)> {
        None
    }
}
