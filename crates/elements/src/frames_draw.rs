//! What the painter needs of each scroll frame (IU4 4c,
//! `docs/plans/iu4.md` section 3): a batch's items are in its frame's
//! coordinates and its clip is the inner one, so the painter adds the
//! frame's shift at draw and cuts the scissor to the frame's clip. And
//! where in a frame its items can show, which culling tests them in.

use vapors_scene::Rect;

use crate::framed_batches::BatchRows;
use crate::frames::Frames;
use crate::painted::Painted;

/// Names where a batch's items are: one frame, or the rows of a framed
/// lane, each item in its own row's frame (`framed_batches.rs`, 4f).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BatchFrame(pub(crate) Framing);

/// What a [`BatchFrame`] names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Framing {
    /// A scroll frame, a lane frame alone, or the page's.
    Frame(usize),
    /// The rows of the framed lane of this container.
    Lane(usize),
}

impl BatchFrame {
    /// Frame `frame`.
    pub(crate) fn frame(frame: usize) -> Self {
        Self(Framing::Frame(frame))
    }

    /// The rows of the framed lane of `container`.
    pub(crate) fn lane(container: usize) -> Self {
        Self(Framing::Lane(container))
    }
}

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

impl FrameDraw {
    /// A batch's clip on the page, the painter's scissor: `clip`, its
    /// clip in the frame (or a lane's template, `framed_batches_rows.rs`),
    /// moved by the shift and cut to the frame's clip; `None` for none.
    /// One answer for the painter and the debug check that places the
    /// kept batches on the page (`kept_batches_check_page.rs`). Unmoved,
    /// to the bit, by a zero shift: a key tells -0.0 from 0.0.
    #[must_use]
    // Exact: a zero shift is exactly zero.
    #[allow(clippy::float_cmp)]
    pub fn on_page(&self, clip: Option<Rect>) -> Option<Rect> {
        let moved = |clip: Rect| {
            if self.shift == [0.0, 0.0] {
                clip
            } else {
                clip.offset(self.shift)
            }
        };
        match (clip.map(moved), self.clip) {
            (Some(clip), Some(outer)) => Some(clip.intersection(outer)),
            (clip, outer) => clip.or(outer),
        }
    }

    /// A batch item's clip on the page, as [`Frames::item`] puts it there:
    /// moved by the shift and cut to the frame's clip
    /// ([`vapors_scene::Clip::cut`]), so a rounded clip keeps its corners.
    /// For the debug check that places the kept batches on the page.
    #[must_use]
    // Exact: a zero shift is exactly zero.
    #[allow(clippy::float_cmp)]
    pub fn clip_on_page(&self, clip: Option<vapors_scene::Clip>) -> Option<vapors_scene::Clip> {
        let moved = |clip: vapors_scene::Clip| {
            if self.shift == [0.0, 0.0] {
                clip
            } else {
                clip.offset(self.shift)
            }
        };
        match (clip.map(moved), self.clip) {
            (Some(clip), Some(outer)) => Some(clip.cut(outer)),
            (clip, outer) => clip.or(outer.map(vapors_scene::Clip::square)),
        }
    }
}

impl Frames {
    /// `view`, device pixels on the page, in frame `frame`'s coordinates
    /// and cut to its clip: where the frame's items can show.
    pub(crate) fn view_in(&self, frame: usize, view: Rect) -> Rect {
        view_at(self.at(frame), view)
    }

    /// Where a batch named `frame` is drawn: its frame's shift and clip,
    /// or a lane's (`Frames::lane_template_of`), each row then moved by
    /// its offset (`row_offsets`).
    pub(crate) fn draw(&self, frame: BatchFrame) -> FrameDraw {
        let at = match frame.0 {
            Framing::Frame(frame) => self.at(frame),
            Framing::Lane(container) => self.lane_template_of(container),
        };
        FrameDraw {
            shift: at.shift,
            clip: at.device_clip,
        }
    }

    /// Each of `rows`' frames' offset from lane `container`'s template,
    /// device pixels, whole: what the shader adds to the batch's shift.
    /// A slot no row holds (`NO_ROW`) is at no offset; no item names it.
    pub(crate) fn row_offsets(&self, container: usize, rows: &[usize]) -> Vec<[f32; 2]> {
        let base = self.lane_template_of(container).shift;
        let offset = |&row: &usize| {
            if row == crate::framed_batches::NO_ROW {
                return [0.0, 0.0];
            }
            let shift = self.at(row).shift;
            [shift[0] - base[0], shift[1] - base[1]]
        };
        rows.iter().map(offset).collect()
    }
}

/// `view`, device pixels on the page, in the coordinates of the frame
/// at `at` and cut to its clip.
pub(crate) fn view_at(at: crate::frames::FrameAt, view: Rect) -> Rect {
    let view = at.device_clip.map_or(view, |clip| clip.intersection(view));
    view.offset(at.shift.map(|v| -v))
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
        self.frames.draw(frame)
    }

    /// Which paint's frames these are: another after every paint, the
    /// same while nothing was painted. While it, the view and the batches
    /// stay, where each batch is drawn ([`Painted::frame_draw`],
    /// [`Painted::row_offsets`]) stays too.
    #[must_use]
    pub fn frames_epoch(&self) -> u64 {
        self.frames.epoch()
    }

    /// The offsets of the rows of a batch named `frame` with `rows`, from
    /// where the batch is drawn ([`Painted::frame_draw`]): none for a
    /// batch in one frame.
    ///
    /// # Panics
    ///
    /// As [`Painted::frame_draw`].
    #[must_use]
    pub fn row_offsets(&self, frame: BatchFrame, rows: BatchRows<'_>) -> Vec<[f32; 2]> {
        match frame.0 {
            Framing::Frame(_) => Vec::new(),
            Framing::Lane(container) => self.frames.row_offsets(container, rows.frames),
        }
    }
}

#[cfg(test)]
#[path = "frames_draw_tests.rs"]
mod tests;
