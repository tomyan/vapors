//! A line mixing font sizes or line heights is refused (F2c S6 review):
//! Chromium's line baseline is the maximum over the strut and every
//! inline box of its rounded ascent plus half-leading, which vapors
//! does not compute yet; it places each inline box on the block's
//! strut. An inline element whose content area or line height differs
//! from its block's is therefore refused, naming it, rather than drawn
//! on the wrong baseline. Weight and face alone change neither: the
//! bundled faces share their vertical metrics.

use std::collections::HashMap;

use vapors_dom::NodeId;
use vapors_text::{LineHeight, content_area};

use crate::check::describe;
use crate::document::Document;
use crate::error::ElementError;
use crate::style::{TextStyle, Used};

const REASON: &str = "differs from its block's, and a line mixing font sizes or line heights is not laid out as Chromium lays it out yet (a slice after F2c, docs/F2c.md); give the element its block's font-size and line-height";

impl Document {
    /// The refusal for `node`, an inline element, when its metrics
    /// differ from its block's.
    pub(crate) fn mixed_metrics(
        &self,
        node: NodeId,
        used: &HashMap<NodeId, Used>,
        scale: f32,
    ) -> Option<ElementError> {
        let own = &used.get(&node)?.text;
        let block = self
            .chain(node)
            .into_iter()
            .skip(1)
            .find_map(|n| used.get(&n).filter(|u| !u.inline))?;
        let (property, value) = difference(own, &block.text, scale)?;
        Some(ElementError::Unhonoured {
            element: describe(&self.arena, node),
            property,
            value,
            reason: REASON.to_owned(),
            declared: None,
        })
    }
}

/// The first of font-size and line-height in which `own` differs from
/// `block`, with `own`'s value as CSS.
fn difference(own: &TextStyle, block: &TextStyle, scale: f32) -> Option<(&'static str, String)> {
    let area = |style: &TextStyle| {
        let layout = style.for_layout();
        content_area(&layout.family, layout.size, scale)
    };
    if own.size.to_bits() != block.size.to_bits() || area(own) != area(block) {
        return Some(("font-size", format!("{}px", own.size)));
    }
    if own.line_height != block.line_height {
        return Some(("line-height", line_height_css(own.line_height)));
    }
    None
}

fn line_height_css(line_height: LineHeight) -> String {
    match line_height {
        LineHeight::Normal => "normal".to_owned(),
        LineHeight::FontSizeRelative(n) => n.to_string(),
        LineHeight::Px(px) => format!("{px}px"),
    }
}
