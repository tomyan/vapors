//! An icon: stroked [`IconPath`]s in a view box, rasterised by zeno at a
//! device size in whole pixels (docs/icons.md, Paint). Strokes are
//! zeno's defaults, which are SVG's: miter joins with limit 4, butt caps.

use std::fmt;
use std::sync::Arc;

use zeno::{Cap, Command, Fill, Format, Join, Mask, Origin, Stroke, Style, Transform};

use crate::icon_key::key_bits;
use crate::icon_miter::missing_wedges;
use crate::icon_path::IconPath;
use crate::raster::Coverage;

/// One `<path>`: its data and stroke width, in view box units.
#[derive(Clone, Debug, PartialEq)]
pub struct IconStroke {
    /// The path.
    pub path: IconPath,
    /// `stroke-width`.
    pub width: f32,
}

/// An `<svg>` of stroked paths, checked.
#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    view_box: [f32; 4],
    strokes: Vec<IconStroke>,
    bits: Vec<Arc<[u32]>>,
}

/// Why an icon was refused.
#[derive(Clone, Debug, PartialEq)]
pub enum IconError {
    /// No paths: nothing to draw.
    NoStrokes,
    /// A view box with a non-finite number, or a width or height not above 0.
    ViewBox([f32; 4]),
    /// A stroke width that is not a positive finite number.
    StrokeWidth {
        /// The stroke's position among the icon's paths.
        index: usize,
        /// Its width.
        width: f32,
    },
}

impl Icon {
    /// The icon `strokes` draw in `view_box` (`min-x min-y width height`).
    ///
    /// # Errors
    /// [`IconError`] for no strokes, a view box without area, or a stroke
    /// width that is not positive.
    pub fn new(view_box: [f32; 4], strokes: Vec<IconStroke>) -> Result<Self, IconError> {
        if strokes.is_empty() {
            return Err(IconError::NoStrokes);
        }
        let has_area = view_box[2] > 0.0 && view_box[3] > 0.0;
        if !has_area || !view_box.iter().all(|n| n.is_finite()) {
            return Err(IconError::ViewBox(view_box));
        }
        let positive = |width: f32| width > 0.0 && width.is_finite();
        if let Some(index) = strokes.iter().position(|s| !positive(s.width)) {
            let width = strokes[index].width;
            return Err(IconError::StrokeWidth { index, width });
        }
        let bits = strokes
            .iter()
            .map(|s| key_bits(view_box, s).into())
            .collect();
        Ok(Self {
            view_box,
            strokes,
            bits,
        })
    }

    /// Coverage of the strokes scaled from the view box to `size` device
    /// pixels, uniformly and centred as SVG's default `xMidYMid meet`, its
    /// top-left at the box's corner. Strokes overlap as two paths of one
    /// colour paint: `a + b(1 - a)`.
    /// That is what an opaque colour paints; a translucent one is drawn
    /// path by path ([`Icon::rasterize_stroke`]), as Chromium paints it.
    #[must_use]
    pub fn rasterize(&self, size: [u32; 2]) -> Coverage {
        let mut all = self.rasterize_stroke(0, size);
        for index in 1..self.strokes.len() {
            source_over(&mut all.data, &self.rasterize_stroke(index, size).data);
        }
        all
    }

    /// Coverage of stroke `index` alone, placed as [`Icon::rasterize`].
    ///
    /// # Panics
    ///
    /// If `index` is not below [`Icon::strokes`]; a caller bug.
    #[must_use]
    pub fn rasterize_stroke(&self, index: usize, size: [u32; 2]) -> Coverage {
        let [width, height] = size;
        let data = stroke_mask(&self.strokes[index], self.to_device(size), size);
        Coverage {
            left: 0,
            top: 0,
            width,
            height,
            data,
            notdef: false,
        }
    }

    /// How many strokes (paths) the icon has; at least one.
    #[must_use]
    pub fn strokes(&self) -> usize {
        self.strokes.len()
    }

    /// Stroke `index`'s path, width and the view box as bits: equal for strokes that
    /// rasterise alike, so the atlas can key on it.
    pub(crate) fn bits(&self, index: usize) -> &Arc<[u32]> {
        &self.bits[index]
    }

    /// The view box scaled by the smaller of the two axis ratios and
    /// centred in the box, so device-pixel snapping never stretches it.
    #[allow(clippy::cast_precision_loss)] // device sizes are far below 2^24
    fn to_device(&self, [width, height]: [u32; 2]) -> Transform {
        let [x, y, w, h] = self.view_box;
        let (width, height) = (width as f32, height as f32);
        let scale = (width / w).min(height / h);
        let left = (width - w * scale) / 2.0 - x * scale;
        let top = (height - h * scale) / 2.0 - y * scale;
        Transform::new(scale, 0.0, 0.0, scale, left, top)
    }
}

/// One stroke's coverage: miter joins with limit 4 and butt caps, SVG's
/// defaults, with the miters zeno bevels added (`icon_miter.rs`).
fn stroke_mask(stroke: &IconStroke, transform: Transform, [width, height]: [u32; 2]) -> Vec<u8> {
    let mut style = Stroke::new(stroke.width);
    style.join(Join::Miter).miter_limit(4.0).cap(Cap::Butt);
    let mut mask = render(
        stroke.path.commands(),
        style.into(),
        transform,
        [width, height],
    );
    let wedges = missing_wedges(stroke.path.commands(), stroke.width);
    if !wedges.is_empty() {
        let fill = Style::Fill(Fill::NonZero);
        source_over(
            &mut mask,
            &render(&wedges, fill, transform, [width, height]),
        );
    }
    mask
}

fn render(commands: &[Command], style: Style, transform: Transform, size: [u32; 2]) -> Vec<u8> {
    Mask::new(commands)
        .style(style)
        .transform(Some(transform))
        .format(Format::Alpha)
        .origin(Origin::TopLeft)
        .size(size[0], size[1])
        .render()
        .0
}

fn source_over(under: &mut [u8], over: &[u8]) {
    for (a, &b) in under.iter_mut().zip(over) {
        let sum = u32::from(*a) * 255 + u32::from(b) * (255 - u32::from(*a));
        *a = u8::try_from((sum + 127) / 255).expect("at most 255");
    }
}

impl fmt::Display for IconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStrokes => write!(
                f,
                "icon has no paths. Fix: give the <svg> at least one <path> child \
                 (docs/icons.md)."
            ),
            Self::ViewBox(view_box) => write!(
                f,
                "icon viewBox {view_box:?} has no area. Fix: give viewBox four finite numbers, \
                 min-x min-y width height, with width and height above 0 (docs/icons.md)."
            ),
            Self::StrokeWidth { index, width } => write!(
                f,
                "icon path {index} has stroke-width {width}. Fix: give stroke-width a positive \
                 number in viewBox units, or leave it out for 1 (docs/icons.md)."
            ),
        }
    }
}

impl std::error::Error for IconError {}

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
