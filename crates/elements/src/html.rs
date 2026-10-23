//! The tree read back as HTML, for tests: the same shape as
//! `RecordingDom::to_html` (sorted attributes, no escaping), so a test
//! can hold the two `Dom` implementations to one answer.

use std::fmt::Write;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::node::Kind;

impl Document {
    /// The children of `node` as HTML: tags, sorted attributes and text,
    /// with no escaping (it is for assertions, not browsers).
    #[must_use]
    pub fn inner_html(&self, node: NodeId) -> String {
        let mut out = String::new();
        for child in self.arena.children(node) {
            self.write_html(&mut out, child);
        }
        out
    }

    fn write_html(&self, out: &mut String, node: NodeId) {
        match &self.arena.get(node, "inner_html").kind {
            Kind::Text(text) => out.push_str(text),
            Kind::Element(element) => {
                let _ = write!(out, "<{}", element.tag);
                for (name, value) in &element.attributes {
                    let _ = write!(out, " {name}=\"{value}\"");
                }
                out.push('>');
                for child in self.arena.children(node) {
                    self.write_html(out, child);
                }
                let _ = write!(out, "</{}>", element.tag);
            }
        }
    }
}
