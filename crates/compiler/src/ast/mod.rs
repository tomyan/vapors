//! What a parsed `.wisp` file is made of.

mod attributes;
mod blocks;

pub use attributes::{AttrValue, Attribute, Attributes};
pub use blocks::{ForBlock, IfBlock, Render, Snippet};

/// A parsed component file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// The type name, from the file name (`counter.wisp` is `Counter`).
    pub name: String,
    /// The `<script>` section, if present.
    pub script: Option<Section>,
    /// The `<style>` section, if present, untouched.
    pub style: Option<Section>,
    /// Byte offset of the template in the file.
    pub template_offset: usize,
    /// The template.
    pub template: Document,
}

/// The text of a section and where it starts in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The content between the tags, untrimmed.
    pub text: String,
    /// Byte offset of the content in the file.
    pub offset: usize,
    pub(crate) close_len: usize,
}

impl Section {
    /// Byte offset just past the section's close tag.
    #[must_use]
    pub fn end(&self) -> usize {
        self.offset + self.text.len() + self.close_len
    }
}

/// The template: top-level elements and blocks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    /// Top-level nodes in source order.
    pub children: Vec<Node>,
}

/// A template node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// An HTML element.
    Element(Element),
    /// A component tag (capitalised, or a `path::Name`).
    Component(ComponentTag),
    /// A run of text with `{expr}` holes.
    Text(Text),
    /// `{if cond}...{else}...{/if}`.
    If(IfBlock),
    /// `{for pattern in iterable key=expr}...{/for}`.
    For(ForBlock),
    /// `{snippet name(params)}...{/snippet}`.
    Snippet(Snippet),
    /// `{render name(args)}`.
    Render(Render),
}

/// A Rust expression from the template and where it starts in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    /// The code, trimmed.
    pub code: String,
    /// Byte offset of the code in the file.
    pub offset: usize,
}

/// An HTML element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The tag name, lowercase.
    pub tag: String,
    /// Attributes in source order.
    pub attributes: Attributes,
    /// Body nodes; empty for a self-closing or void element.
    pub children: Vec<Node>,
    /// Byte offset of the `<`.
    pub offset: usize,
}

/// A component tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentTag {
    /// `Counter` or `widgets::TextInput`.
    pub name: String,
    /// Attributes in source order.
    pub attributes: Attributes,
    /// Body nodes (snippets and loose content).
    pub children: Vec<Node>,
    /// Byte offset of the `<`.
    pub offset: usize,
}

/// A run of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    /// Literal runs and expression holes in order.
    pub parts: Vec<Part>,
    /// Byte offset of the run.
    pub offset: usize,
}

/// A piece of a text run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    /// Literal characters.
    Literal(String),
    /// A `{expr}` hole.
    Expr(Expr),
}
