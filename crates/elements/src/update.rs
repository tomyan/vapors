//! An update: check the tree, compute styles, read them, lay out and
//! paint. Every refusal found on the way is returned together, and a
//! refused update paints nothing.

use vapors_css::cascade::Environment;
use vapors_dom::NodeId;

use crate::check::{check_tree, describe, elements, style_attribute_refusals};
use crate::document::Document;
use crate::error::ElementError;
use crate::layout::{Calcs, build, lay_out, placed};
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

/// Whether anything shown may have changed since the app last asked
/// ([`Document::take_changed`]) and since the last update; both, for a
/// new document.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Changes {
    pub(crate) since_asked: bool,
    pub(crate) since_update: bool,
}

impl Default for Changes {
    fn default() -> Self {
        Self {
            since_asked: true,
            since_update: true,
        }
    }
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
        self.counts = crate::UpdateCounts::default();
        self.texts.begin();
        let calcs = self.styled(viewport)?;
        let (root, body) = (self.root(), self.body());
        let text_marks = self.focused_text_marks();
        let kept = self.kept.as_ref().expect("styled keeps the styles");
        let used = &kept.used;
        let source = crate::layout::Source {
            arena: &self.arena,
            used,
            images: &self.images,
            text_marks,
        };
        let boxes = build(&source, root);
        self.counts.boxes_built = boxes.len();
        let size = (viewport.width, viewport.height);
        let texts = (&mut self.fonts, &mut self.texts);
        let (boxes, work) = lay_out(
            boxes,
            texts,
            (&self.boxes, &|node| kept.may_keep_layout(node)),
            viewport.scale,
            size,
            &calcs,
        )
        .map_err(|(failure, work)| {
            self.counts.layout_calls = work.calls;
            vec![ElementError::Layout(failure)]
        })?;
        let scrolls = |index: usize| match boxes[index].content {
            crate::layout::Content::Element(node) => used[&node].scrolls(),
            crate::layout::Content::Text(_) => false,
        };
        let offsets = self.scroll.settle(&boxes, &scrolls, viewport.scale);
        let rects = placed(&boxes, &offsets);
        let mut input = PaintInput {
            used,
            root,
            body,
            propagated: None,
            fonts: &mut self.fonts,
            cache: &mut self.texts,
            scale: viewport.scale,
            clip: None,
            scroll: &self.scroll,
            thumbs_shown: self.thumbs_shown,
        };
        let mut painted = paint(&boxes, &rects, &mut input);
        painted.scroll_chains = crate::scroll_chains::Chains::of(&boxes, &scrolls);
        self.boxes.keep(boxes, viewport.scale);
        (self.laid_out, self.counts.layout_calls) = (work.laid_out, work.calls);
        self.counts.note_paint(&painted);
        // Cleared before the rehover: a hover it moves is a change this
        // update did not restyle, so the a11y tree refuses until the next.
        self.changes.since_update = false;
        self.rehover_in(&painted);
        Ok(self.painted.insert(painted))
    }

    /// Notes a change to anything shown, for [`Document::take_changed`]
    /// and for what reads the last update (the a11y tree).
    pub(crate) fn note_change(&mut self) {
        self.changes = Changes::default();
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
        let mut errors = check_tree(&self.arena, self.root(), &self.declared_events);
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
        let kept = self.kept.as_ref().expect("restyle keeps the styles");
        let mut calcs = Calcs::default();
        for &node in &live {
            self.counts.elements_visited += 1;
            errors.extend(kept.refused.get(&node).into_iter().flatten().cloned());
            calcs.add(kept.styles.of(node), || describe(&self.arena, node));
            if kept.used.get(&node).is_some_and(|style| style.inline) {
                errors.extend(self.mixed_metrics(node, &kept.used, viewport.scale));
            }
        }
        errors.extend(crate::media_checks::refusals(
            self,
            &live,
            &kept.used,
            viewport.scale,
        ));
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
