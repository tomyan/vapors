//! An update: check the tree, compute styles, read them, lay out and
//! paint. Every refusal found on the way is returned together, and a
//! refused update paints nothing.

use vapors_css::cascade::Environment;
use vapors_dom::NodeId;

use crate::check::{check_tree, describe, elements, style_attribute_refusals};
use crate::document::Document;
use crate::error::ElementError;
use crate::layout::{Calcs, build, lay_out};
use crate::paint::{PaintInput, paint};
use crate::painted::Painted;
use crate::style::Refusal;

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
    ///
    /// # Panics
    /// Only on a bug in this crate: a restyle that succeeds always keeps
    /// the styles layout reads.
    pub fn update(&mut self, viewport: Viewport) -> Result<&Painted, Vec<ElementError>> {
        self.painted = None;
        self.texts.begin();
        let calcs = self.styled(viewport)?;
        let (root, body) = (self.root(), self.body());
        let used = &self.kept.as_ref().expect("styled keeps the styles").used;
        let boxes = build(&self.arena, used, root);
        let size = (viewport.width, viewport.height);
        let texts = (&mut self.fonts, &mut self.texts);
        let (boxes, rects, laid_out) =
            lay_out(boxes, texts, &self.boxes, viewport.scale, size, &calcs)
                .map_err(|failure| vec![ElementError::Layout(failure)])?;
        let mut input = PaintInput {
            used,
            root,
            body,
            propagated: None,
            fonts: &mut self.fonts,
            cache: &mut self.texts,
            scale: viewport.scale,
        };
        let painted = paint(&boxes, &rects, &mut input);
        self.boxes.keep(boxes, viewport.scale);
        self.laid_out = laid_out;
        self.rehover_in(&painted);
        Ok(self.painted.insert(painted))
    }

    /// How many elements the last update restyled: what the incremental
    /// tests count, and what the `stress` demo shows.
    #[must_use]
    pub fn elements_restyled(&self) -> usize {
        self.kept.as_ref().map_or(0, |kept| kept.restyled)
    }

    /// How many times the last update's layout laid a box out rather
    /// than reading its cache: what the incremental tests count.
    #[must_use]
    pub fn boxes_laid_out(&self) -> usize {
        self.laid_out
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

    /// Restyles what changed and checks the tree; the calcs layout
    /// will ask for, collected from the kept computed styles.
    fn styled(&mut self, viewport: Viewport) -> Result<Calcs, Vec<ElementError>> {
        let mut errors = check_tree(&self.arena, self.root());
        errors.extend_from_slice(self.sheets.refused());
        let unparsed = style_attribute_refusals(&self.arena, self.root());
        if !unparsed.is_empty() {
            errors.extend(unparsed);
            return Err(errors);
        }
        if let Err(failed) = self.restyle(viewport) {
            errors.extend(failed);
            return Err(errors);
        }
        let live = elements(&self.arena, self.root());
        let kept = self.kept.as_mut().expect("restyle keeps the styles");
        let alive: std::collections::HashSet<NodeId> = live.iter().copied().collect();
        kept.used.retain(|node, _| alive.contains(node));
        kept.refused.retain(|node, _| alive.contains(node));
        let mut calcs = Calcs::default();
        for &node in &live {
            errors.extend(kept.refused.get(&node).into_iter().flatten().cloned());
            calcs.add(kept.styles.of(node), || describe(&self.arena, node));
        }
        for node in elements(&self.arena, self.root()) {
            if out.get(&node).is_some_and(|style| style.inline) {
                errors.extend(self.mixed_metrics(node, &out, viewport.scale));
            }
        }
        if errors.is_empty() {
            Ok(calcs)
        } else {
            Err(errors)
        }
    }
}

impl Document {
    pub(crate) fn unhonoured(
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

pub(crate) fn environment(viewport: Viewport) -> Environment {
    Environment {
        viewport_width: f64::from(viewport.width),
        viewport_height: f64::from(viewport.height),
        resolution: f64::from(viewport.scale),
        ..Environment::default()
    }
}
