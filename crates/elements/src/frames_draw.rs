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
    pub(crate) fn row_offsets(&self, container: usize, rows: &[usize]) -> Vec<[f32; 2]> {
        let base = self.lane_template_of(container).shift;
        let offset = |&row: &usize| {
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

    /// The offsets of the rows of a batch named `frame` with `rows`, from
    /// where the batch is drawn ([`Painted::frame_draw`]): none for a
    /// batch in one frame.
    ///
    /// # Panics
    ///
    /// As [`Painted::frame_draw`].
    #[must_use]
    pub fn row_offsets(&self, frame: BatchFrame, rows: &BatchRows) -> Vec<[f32; 2]> {
        match frame.0 {
            Framing::Frame(_) => Vec::new(),
            Framing::Lane(container) => self.frames.row_offsets(container, &rows.frames),
        }
    }
}
