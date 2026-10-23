//! An update: check the tree, compute styles, read them, lay out and
//! paint. Every refusal found on the way is returned together, and a
//! refused update paints nothing.

use std::collections::HashMap;

use vapors_css::cascade::Environment;
use vapors_dom::NodeId;

use crate::check::{check_tree, describe, elements, implemented};
use crate::document::Document;
use crate::error::ElementError;
use crate::layout::{build, lay_out};
use crate::paint::{PaintInput, Painted, paint};
use crate::style::{Refusal, Used, initial_style, used};

/// What the window or canvas gives the document each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Width in CSS pixels.
    pub width: f32,
    /// Height in CSS pixels.
    pub height: f32,
    /// Device pixels per CSS pixel.
    pub scale: f32,
}

impl Document {
    /// Styles, lays out and paints the document for `viewport`.
    ///
    /// # Errors
    ///
    /// Every refusal in the tree and its styles: unimplemented tags,
    /// listeners for events never fired, stylesheets that do not compute,
    /// and properties or values the tree does not implement yet. Nothing
    /// is painted then, and the previous result is dropped.
    pub fn update(&mut self, viewport: Viewport) -> Result<&Painted, Vec<ElementError>> {
        self.painted = None;
        let used = self.used_styles(viewport)?;
        let boxes = build(&self.arena, &used, self.root());
        let (boxes, rects) = lay_out(boxes, &mut self.fonts, viewport.scale, viewport.width);
        let mut input = PaintInput {
            used: &used,
            fonts: &mut self.fonts,
            scale: viewport.scale,
        };
        Ok(self.painted.insert(paint(&boxes, &rects, &mut input)))
    }

    /// The last successful update's result, if the document has not
    /// failed an update since.
    #[must_use]
    pub fn painted(&self) -> Option<&Painted> {
        self.painted.as_ref()
    }

    fn used_styles(&self, viewport: Viewport) -> Result<HashMap<NodeId, Used>, Vec<ElementError>> {
        let env = environment(viewport);
        let mut errors = check_tree(&self.arena, self.root());
        let styles = self
            .sheets
            .compute(&self.arena, self.root(), &env)
            .map_err(|e| vec![e])?;
        let initial = initial_style(&self.arena, self.root(), &env);
        let mut out = HashMap::new();
        for node in elements(&self.arena, self.root()) {
            if !implemented(&self.arena, node) {
                continue;
            }
            match used(styles.of(node), &initial) {
                Ok(style) => drop(out.insert(node, style)),
                Err(refusals) => errors.extend(self.unhonoured(node, refusals)),
            }
        }
        if errors.is_empty() {
            Ok(out)
        } else {
            Err(errors)
        }
    }
}

impl Document {
    fn unhonoured(
        &self,
        node: NodeId,
        refusals: Vec<Refusal>,
    ) -> impl Iterator<Item = ElementError> {
        let element = describe(&self.arena, node);
        refusals.into_iter().map(move |r| ElementError::Unhonoured {
            element: element.clone(),
            property: r.property,
            value: r.value,
            reason: r.reason,
        })
    }
}

fn environment(viewport: Viewport) -> Environment {
    Environment {
        viewport_width: f64::from(viewport.width),
        viewport_height: f64::from(viewport.height),
        resolution: f64::from(viewport.scale),
        ..Environment::default()
    }
}
