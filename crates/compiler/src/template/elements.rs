//! Elements and component tags: the tag name decides which, the
//! attributes follow, then a self-closing `/>`, a void tag's `>`, or a
//! body up to the close tag.

use super::Parser;
use super::children::{Closer, is_tag_char};
use super::vocabulary::{SLOT_REMOVED, is_html_tag, is_void};
use crate::CompileError;
use crate::ast::{Attributes, ComponentTag, Element, Node};

impl Parser<'_> {
    /// Parses the element or component tag at the cursor (`<`).
    pub fn parse_element(&mut self) -> Result<Node, CompileError> {
        let offset = self.pos;
        if self.starts_with("<!--") {
            return Err(self.error(
                offset,
                "there is no comment syntax in a template: put the note in <script> as a `//` comment",
            ));
        }
        self.bump(1);
        let name = self.read_while(is_tag_char).to_string();
        if name.is_empty() {
            return Err(self.error(offset, "expected a tag name after `<`"));
        }
        if is_component_name(&name) {
            return self.parse_component(name, offset);
        }
        self.refuse_unknown_tag(&name, offset)?;
        let attributes = self.parse_attributes(&name)?;
        let children = self.parse_body(&name, offset)?;
        Ok(Node::Element(Element {
            tag: name,
            attributes,
            children,
            offset,
        }))
    }

    fn refuse_unknown_tag(&self, name: &str, offset: usize) -> Result<(), CompileError> {
        let message = match name {
            _ if is_html_tag(name) => return Ok(()),
            "box" => "the <box> tag was removed; use <div> (or another HTML container element)".into(),
            "text" => "the <text> tag was removed; use <span> (or another HTML text element)".into(),
            "title" => "<title> is not an element here: the window title is a platform concern, set from Rust".into(),
            "script" | "style" => format!("the <{name}> section goes before the template, not inside it"),
            _ if name.starts_with("slot") => SLOT_REMOVED.into(),
            _ => format!(
                "unknown element `{name}`: HTML elements are lowercase (div, span, button, ...); a component tag is capitalised, like `<{}>`",
                capitalise(name)
            ),
        };
        Err(self.error(offset, message))
    }

    /// Nothing for `/>` or a void tag; otherwise the nodes up to the
    /// close tag.
    fn parse_body(&mut self, name: &str, opened: usize) -> Result<Vec<Node>, CompileError> {
        if self.starts_with("/>") {
            self.bump(2);
            return Ok(Vec::new());
        }
        self.expect_tag_end(name, opened)?;
        if is_void(name) {
            return Ok(Vec::new());
        }
        let closer = Closer::Element {
            tag: name.to_string(),
            opened,
        };
        Ok(self.parse_children(&closer)?.0)
    }

    fn expect_tag_end(&mut self, name: &str, opened: usize) -> Result<(), CompileError> {
        if !self.starts_with(">") {
            return Err(self.error(
                self.pos.max(opened),
                format!("expected `>` to finish the `<{name}>` tag"),
            ));
        }
        self.bump(1);
        Ok(())
    }

    fn parse_component(&mut self, name: String, offset: usize) -> Result<Node, CompileError> {
        let attributes = self.parse_attributes(&name)?;
        let children = if self.starts_with("/>") {
            self.bump(2);
            Vec::new()
        } else {
            self.expect_tag_end(&name, offset)?;
            let closer = Closer::Element {
                tag: name.clone(),
                opened: offset,
            };
            self.parse_children(&closer)?.0
        };
        Ok(Node::Component(ComponentTag {
            name,
            attributes,
            children,
            offset,
        }))
    }
}

/// A capitalised name, or a `path::Name`.
fn is_component_name(name: &str) -> bool {
    let last = name.rsplit("::").next().unwrap_or(name);
    last.chars().next().is_some_and(char::is_uppercase)
}

fn capitalise(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

impl Attributes {
    pub(crate) fn push(&mut self, attribute: crate::ast::Attribute) {
        self.0.push(attribute);
    }
}
