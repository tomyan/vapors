//! Reading the recorded tree back as HTML, for assertions.

use std::fmt::Write;

use super::{Kind, RecordingDom};
use crate::NodeId;

impl RecordingDom {
    /// The root's children as HTML: tags, sorted attributes and text,
    /// with no escaping (it is for assertions, not browsers).
    #[must_use]
    pub fn to_html(&self) -> String {
        let mut out = String::new();
        for child in &self.record(self.root()).children {
            self.write_html(&mut out, *child);
        }
        out
    }

    fn write_html(&self, out: &mut String, node: NodeId) {
        let record = self.record(node);
        match &record.kind {
            Kind::Text(text) => out.push_str(text),
            Kind::Element {
                tag, attributes, ..
            } => {
                let _ = write!(out, "<{tag}");
                for (name, value) in attributes {
                    let _ = write!(out, " {name}=\"{value}\"");
                }
                out.push('>');
                for child in &record.children {
                    self.write_html(out, *child);
                }
                let _ = write!(out, "</{tag}>");
            }
        }
    }
}
