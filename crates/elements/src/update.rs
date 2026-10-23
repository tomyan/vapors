//! An update: check the tree, compute styles, read them, lay out and
//! paint. Every refusal found on the way is returned together, and a
//! refused update paints nothing.

use std::collections::HashMap;

use vapors_css::cascade::Environment;
use vapors_dom::NodeId;

use crate::check::{check_tree, describe, elements, implemented, style_attribute_refusals};
use crate::document::Document;
use crate::error::ElementError;
use crate::layout::{Calcs, build, lay_out};
use crate::paint::{PaintInput, paint};
use crate::painted::Painted;
use crate::style::{Context, ElementStyles, Refusal, Used, initial_style, used};

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
        self.texts.begin();
        let (used, calcs) = self.used_styles(viewport)?;
        let boxes = build(&self.arena, &used, self.root());
        let size = (viewport.width, viewport.height);
        let texts = (&mut self.fonts, &mut self.texts);
        let (boxes, rects) = lay_out(boxes, texts, viewport.scale, size, &calcs)
            .map_err(|failure| vec![ElementError::Layout(failure)])?;
        let mut input = PaintInput {
            used: &used,
            root: self.root(),
            body: self.body(),
            propagated: None,
            fonts: &mut self.fonts,
            cache: &mut self.texts,
            scale: viewport.scale,
        };
        let painted = paint(&boxes, &rects, &mut input);
        self.rehover_in(&painted);
        Ok(self.painted.insert(painted))
    }

    /// How many Parley layouts the last update built rather than kept
    /// from the one before: what the incremental tests count, and what
    /// the `stress` demo shows.
    #[must_use]
    pub fn text_layouts_built(&self) -> usize {
        self.texts.built()
    }

    /// The last successful update's result, if the document has not
    /// failed an update since.
    #[must_use]
    pub fn painted(&self) -> Option<&Painted> {
        self.painted.as_ref()
    }

    /// Each element's used style, and the calcs layout will ask for,
    /// collected while the computed styles that hold them are alive.
    fn used_styles(
        &self,
        viewport: Viewport,
    ) -> Result<(HashMap<NodeId, Used>, Calcs), Vec<ElementError>> {
        let env = environment(viewport);
        let mut errors = check_tree(&self.arena, self.root());
        errors.extend_from_slice(self.sheets.refused());
        let unparsed = style_attribute_refusals(&self.arena, self.root());
        if !unparsed.is_empty() {
            errors.extend(unparsed);
            return Err(errors);
        }
        let styles = self
            .sheets
            .compute(&self.arena, self.root(), &env)
            .map_err(|e| vec![e])?;
        let initial = initial_style(&self.arena, self.root(), &env);
        let mut out = HashMap::new();
        let mut calcs = Calcs::default();
        for node in elements(&self.arena, self.root()) {
            if !implemented(&self.arena, node) {
                continue;
            }
            let parent_used: Option<&Used> = self.parent(node).and_then(|parent| out.get(&parent));
            let flex_item = parent_used.is_some_and(|parent| {
                matches!(
                    parent.layout.display,
                    taffy::Display::Flex | taffy::Display::Grid
                )
            });
            let in_inline = parent_used.is_some_and(|parent| parent.inline);
            let own = styles.of(node);
            calcs.add(own, &describe(&self.arena, node));
            let parent = self.parent(node).map(|parent| styles.of(parent));
            let context = Context {
                initial: &initial,
                flex_item,
                in_inline,
                scale: viewport.scale,
            };
            match used(ElementStyles { own, parent }, &context) {
                Ok(style) => drop(out.insert(node, style)),
                Err(refusals) => errors.extend(self.unhonoured(node, refusals)),
            }
        }
        for node in elements(&self.arena, self.root()) {
            if out.get(&node).is_some_and(|style| style.inline) {
                errors.extend(self.mixed_metrics(node, &out, viewport.scale));
            }
        }
        if errors.is_empty() {
            Ok((out, calcs))
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
            declared: r.declared.map(|(sheet, at)| {
                format!("{}:{}:{}", self.sheets.name(sheet), at.line, at.column)
            }),
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
