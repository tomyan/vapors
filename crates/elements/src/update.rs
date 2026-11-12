//! An update: check the tree, compute styles, read them, lay out and
//! paint. Every refusal found on the way is returned together, and a
//! refused update paints nothing. Each of the three phases runs in a
//! `tracing` span named `update phase`, its `phase` field `style`,
//! `layout` or `paint`, for a reader of the spans (`crates/perf`).

use vapors_css::cascade::Environment;
use vapors_dom::NodeId;

use crate::changed_slots::ChangedSlots;
use crate::check::describe;
use crate::document::Document;
use crate::error::ElementError;
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
        let mut last = self.painted.take();
        self.counts = crate::UpdateCounts::default();
        self.texts.begin(
            last.as_ref()
                .map_or(0, |painted| painted.slabs.texts.live()),
        );
        let style = tracing::trace_span!("update phase", phase = "style").entered();
        self.styled(viewport)?;
        drop(style);
        let layout = tracing::trace_span!("update phase", phase = "layout").entered();
        let (slotted, rebuilt, written) = self.laid_out(viewport)?;
        let (stackings, patched) = self.stackings(&slotted.boxes, &rebuilt, last.as_mut());
        let mut changed = ChangedSlots::of(rebuilt, last.as_ref(), patched);
        let laid = (&slotted, &stackings);
        let from = self.painted_from(laid, viewport, (last.as_mut(), &written), &mut changed);
        drop(layout);
        let paint_span = tracing::trace_span!("update phase", phase = "paint").entered();
        let mut painted = self.paint_or_keep(last, (&slotted.boxes, &stackings), &from, &changed);
        drop(paint_span);
        self.boxes.keep(slotted, viewport.scale);
        (painted.from, painted.stackings) = (from, stackings);
        Ok(self.shown(painted))
    }

    /// Keeps `painted` as the last update's result, and moves the hover
    /// to what it shows.
    fn shown(&mut self, painted: Painted) -> &Painted {
        // Cleared before the rehover: a hover it moves is a change this
        // update did not restyle, so the a11y tree refuses until the next.
        self.changes.since_update = false;
        self.settle.due = true;
        self.rehover_in(&painted);
        self.painted.insert(painted)
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

    /// Checks the tree and restyles what changed, checking again only
    /// the elements changed since the last update (`style_checks`).
    fn styled(&mut self, viewport: Viewport) -> Result<(), Vec<ElementError>> {
        let touched = self.touched();
        self.recheck_tree(&touched);
        let mut errors = self.tree_refusals();
        errors.extend_from_slice(self.sheets.refused());
        let unparsed = self.style_attribute_refusals();
        if !unparsed.is_empty() {
            errors.extend(unparsed);
            return Err(errors);
        }
        if let Err(failed) = self.restyle(viewport) {
            errors.extend(failed);
            return Err(errors);
        }
        let rechecked = self.rechecked(touched);
        self.recheck_styled(&rechecked, viewport.scale);
        self.checks.checked();
        errors.extend(self.all_styled_refusals(viewport.scale));
        if errors.is_empty() {
            Ok(())
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
