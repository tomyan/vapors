//! What the painter needs of each scroll frame (IU4 4c,
//! `docs/plans/iu4.md` section 3): a batch's items are in its frame's
//! coordinates and its clip is the inner one, so the painter adds the
//! frame's shift at draw and cuts the scissor to the frame's clip. And
//! where in a frame its items can show, which culling tests them in.

use vapors_scene::Rect;

use crate::frames::Frames;
use crate::painted::Painted;

/// Names the scroll frame a batch's items are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BatchFrame(pub(crate) usize);

/// Where a frame's batches are drawn on the target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameDraw {
    /// Added to every position of the frame's batches, device pixels,
    /// whole.
    pub shift: [f32; 2],
    /// The clip every batch of the frame is under, device pixels on the
    /// page; `None` for none.
    pub clip: Option<Rect>,
}

impl Frames {
    /// `view`, device pixels on the page, in frame `frame`'s coordinates
    /// and cut to its clip: where the frame's items can show.
    pub(crate) fn view_in(&self, frame: usize, view: Rect) -> Rect {
        let at = self.at(frame);
        let view = at.device_clip.map_or(view, |clip| clip.intersection(view));
        view.offset(at.shift.map(|v| -v))
    }

    /// Where frame `frame`'s batches are drawn.
    pub(crate) fn draw(&self, frame: usize) -> FrameDraw {
        let at = self.at(frame);
        FrameDraw {
            shift: at.shift,
            clip: at.device_clip,
        }
    }
}

impl Painted {
    /// Where the batches of `frame` are drawn this update: their shift
    /// and the frame's clip.
    ///
    /// # Panics
    /// When `frame` names a frame this paint does not hold: a batch
    /// kept from another paint.
    #[must_use]
    pub fn frame_draw(&self, frame: BatchFrame) -> FrameDraw {
        self.frames.draw(frame.0)
    }
}
