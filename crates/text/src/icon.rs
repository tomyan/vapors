//! An icon: stroked [`IconPath`]s in a view box, rasterised by zeno at a
//! device size in whole pixels (docs/icons.md, Paint). Strokes are
//! zeno's defaults, which are SVG's: miter joins with limit 4, butt caps.

use std::fmt;
use std::sync::Arc;

use zeno::{Cap, Command, Format, Join, Mask, Origin, Stroke, Transform};

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
    bits: Arc<[u32]>,
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
        let bits = key_bits(view_box, &strokes).into();
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
    #[must_use]
    pub fn rasterize(&self, size: [u32; 2]) -> Coverage {
        let [width, height] = size;
        let mut data = vec![0; width as usize * height as usize];
        let transform = self.to_device(size);
        for stroke in &self.strokes {
            source_over(&mut data, &stroke_mask(stroke, transform, size));
        }
        Coverage {
            left: 0,
            top: 0,
            width,
            height,
            data,
            notdef: false,
        }
    }

    /// The view box, stroke widths and paths as bits: equal for icons that
    /// rasterise alike, so the atlas can key on it.
    pub(crate) fn bits(&self) -> &Arc<[u32]> {
        &self.bits
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
/// defaults, set here rather than left to zeno's.
fn stroke_mask(stroke: &IconStroke, transform: Transform, [width, height]: [u32; 2]) -> Vec<u8> {
    let mut style = Stroke::new(stroke.width);
    style.join(Join::Miter).miter_limit(4.0).cap(Cap::Butt);
    Mask::new(stroke.path.commands())
        .style(style)
        .transform(Some(transform))
        .format(Format::Alpha)
        .origin(Origin::TopLeft)
        .size(width, height)
        .render()
        .0
}

fn source_over(under: &mut [u8], over: &[u8]) {
    for (a, &b) in under.iter_mut().zip(over) {
        let sum = u32::from(*a) * 255 + u32::from(b) * (255 - u32::from(*a));
        *a = u8::try_from((sum + 127) / 255).expect("at most 255");
    }
}

fn key_bits(view_box: [f32; 4], strokes: &[IconStroke]) -> Vec<u32> {
    let mut bits: Vec<u32> = view_box.iter().map(|n| n.to_bits()).collect();
    for stroke in strokes {
        bits.push(u32::MAX);
        bits.push(stroke.width.to_bits());
        for command in stroke.path.commands() {
            command_bits(command, &mut bits);
        }
    }
    bits
}

/// A tag per command kind, then its points' coordinates.
fn command_bits(command: &Command, bits: &mut Vec<u32>) {
    let (tag, points) = match command {
        Command::MoveTo(p) => (0, vec![*p]),
        Command::LineTo(p) => (1, vec![*p]),
        Command::CurveTo(a, b, p) => (2, vec![*a, *b, *p]),
        Command::QuadTo(a, p) => (3, vec![*a, *p]),
        Command::Close => (4, vec![]),
    };
    bits.push(tag);
    bits.extend(points.iter().flat_map(|p| [p.x.to_bits(), p.y.to_bits()]));
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
