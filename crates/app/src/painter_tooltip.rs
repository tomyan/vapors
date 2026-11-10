//! The tooltip over the scene: a quad and a text run drawn after the
//! document's runs, so above every item, `gpu-surface`s included, and
//! outside hit-testing and the accessibility tree (its text is already
//! the element's name or description there, docs/tooltips.md).

use vapors_elements::{Document, Point};
use vapors_renderer::Target;
use vapors_scene::{Run, Size};
use vapors_text::{BlendParams, Family, Layout, LayoutOptions, Style};

use super::ScenePainter;
use super::text::text_target;
use crate::Tooltip;
use crate::tooltip_box::{TooltipBox, anchor_of, tooltip_box, wrap_width};
use crate::tooltip_error::TooltipError;
use crate::tooltip_style::TooltipStyle;

impl ScenePainter {
    /// Draws `tooltip` over what this frame's [`ScenePainter::draw`] (or
    /// `draw_over`) drew of `document`, styled by the root's
    /// `--tooltip-*` properties and placed by [`crate::place_tooltip`]:
    /// at `pointer` (from [`crate::DocumentInput::pointer`]) for a
    /// pointer tooltip, at its element's border box for a keyboard one.
    /// Call it after the scene, each frame a tooltip is shown.
    ///
    /// # Errors
    ///
    /// [`TooltipError`], drawing nothing: a `--tooltip-*` value of the
    /// wrong kind, a pointer tooltip with no `pointer`, a keyboard
    /// tooltip whose element has no box, a document not yet updated, or
    /// a glyph that cannot be drawn.
    pub fn draw_tooltip(
        &mut self,
        target: &Target<'_>,
        document: &mut Document,
        tooltip: &Tooltip,
        pointer: Option<Point>,
    ) -> Result<(), TooltipError> {
        let style = TooltipStyle::of(document)?;
        let painted = document.painted().ok_or(TooltipError::NotUpdated)?;
        let (anchor, scale) = (anchor_of(tooltip, pointer, painted)?, painted.scale());
        let window = window_of(target.viewport, scale);
        let layout = lay_out(
            document,
            &tooltip.text,
            &style,
            wrap_width(window[0]),
            scale,
        );
        let text_size = [layout.width(), layout.height()].map(|v| v.ceil() / scale);
        let placed = tooltip_box(&style, text_size, anchor, window, scale)?;
        self.draw_box(target, &placed, &layout)
    }

    /// The box, then the text over it, each counted with its kind.
    fn draw_box(
        &mut self,
        target: &Target<'_>,
        placed: &TooltipBox,
        layout: &Layout,
    ) -> Result<(), TooltipError> {
        self.quads.draw(target, std::slice::from_ref(&placed.quad));
        self.count_run(&Run::Quads(vec![placed.quad]));
        let drawn = self.text.draw(
            &text_target(target),
            &mut self.atlas,
            layout,
            placed.text_origin,
            BlendParams::default(),
        );
        self.count_run(&Run::Texts(&[]));
        drawn.map_err(TooltipError::Atlas)
    }
}

/// `text` in the tooltip's style, wrapped at `width` CSS px.
fn lay_out(
    document: &mut Document,
    text: &str,
    style: &TooltipStyle,
    width: f32,
    scale: f32,
) -> Layout {
    let text_style = Style {
        color: style.color,
        ..Style::new(Family::Geist, style.font_size)
    };
    let options = LayoutOptions {
        width: Some(width),
        scale,
        ..LayoutOptions::default()
    };
    document.lay_out_text(text, &text_style, &options)
}

/// The target's size in CSS pixels.
#[allow(clippy::cast_precision_loss)] // window sizes are small
fn window_of(viewport: Size, scale: f32) -> [f32; 2] {
    [
        viewport.width as f32 / scale,
        viewport.height as f32 / scale,
    ]
}
