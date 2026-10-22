//! The element trait the matcher walks.

use crate::tree::States;

/// One element of a tree, as a cheap handle: cloning it must not copy the
/// tree. Two handles are equal when they name the same element.
pub trait Element: Clone + PartialEq {
    /// The tag name; compared ASCII case-insensitively.
    fn tag_name(&self) -> &str;
    /// The id attribute.
    fn id(&self) -> Option<&str>;
    /// Whether `class` is in the class list.
    fn has_class(&self, class: &str) -> bool;
    /// An attribute's value.
    fn attribute(&self, name: &str) -> Option<&str>;
    /// The parent element; `None` for the root.
    fn parent(&self) -> Option<Self>;
    /// The previous element sibling.
    fn previous_sibling(&self) -> Option<Self>;
    /// The next element sibling.
    fn next_sibling(&self) -> Option<Self>;
    /// The first child element.
    fn first_child(&self) -> Option<Self>;
    /// Whether the element has no children and no text.
    fn is_empty(&self) -> bool;
    /// The state flags set on this element.
    fn states(&self) -> States;
    /// The nearest ancestor container's laid-out size, for `@container`.
    fn container_size(&self) -> Option<(f32, f32)>;

    /// The children in order.
    fn children(&self) -> Vec<Self> {
        let mut out = Vec::new();
        let mut next = self.first_child();
        while let Some(c) = next {
            next = c.next_sibling();
            out.push(c);
        }
        out
    }
}
