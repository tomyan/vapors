//! skrifa draws outlines into a pen; this one collects zeno path commands
//! and rasterises them, flipping y so rows go down from the baseline.

use skrifa::outline::OutlinePen;
use zeno::{Command, Format, Mask, Origin, Transform, Vector};

use crate::raster::Coverage;

/// The outline as zeno commands, in font units scaled to pixels, y up.
#[derive(Default)]
pub struct PathCommands(Vec<Command>);

impl OutlinePen for PathCommands {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::MoveTo(Vector::new(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::LineTo(Vector::new(x, y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0
            .push(Command::QuadTo(Vector::new(cx0, cy0), Vector::new(x, y)));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.push(Command::CurveTo(
            Vector::new(cx0, cy0),
            Vector::new(cx1, cy1),
            Vector::new(x, y),
        ));
    }

    fn close(&mut self) {
        self.0.push(Command::Close);
    }
}

impl PathCommands {
    /// Coverage of the path shifted right by `dx` pixels, y flipped so the
    /// bitmap's rows run down from above the baseline.
    pub fn rasterize(&self, dx: f32) -> Coverage {
        let flip = Transform::new(1.0, 0.0, 0.0, -1.0, dx, 0.0);
        let (data, placement) = Mask::new(&self.0)
            .format(Format::Alpha)
            .origin(Origin::TopLeft)
            .transform(Some(flip))
            .render();
        Coverage {
            left: placement.left,
            top: placement.top,
            width: placement.width,
            height: placement.height,
            data,
            notdef: false,
        }
    }
}
