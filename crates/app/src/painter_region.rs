//! Where the painter may draw on a target: the area batching culls
//! against, and each batch's scissor inside it, from the batch's clip
//! in its scroll frame and the frame's shift and clip.

use vapors_elements::FrameDraw;
use vapors_renderer::Target;
use vapors_scene::{Rect, Scissor, Size};

/// Where a batch may draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Region {
    /// The whole target.
    Whole,
    /// Inside this scissor.
    Within(Scissor),
    /// Nowhere: the batch is not drawn.
    Nowhere,
}

/// Where a batch under `clip` may draw on a `viewport`-sized target
/// that has its own scissor `outer`: inside both.
fn region(outer: Option<Scissor>, clip: Option<Rect>, viewport: Size) -> Region {
    let inner = match clip.map(|clip| clip.scissor(viewport)) {
        None => return outer.map_or(Region::Whole, Region::Within),
        Some(None) => return Region::Nowhere,
        Some(Some(inner)) => inner,
    };
    match outer.map_or(Some(inner), |outer| outer.intersect(inner)) {
        Some(both) => Region::Within(both),
        None => Region::Nowhere,
    }
}

/// Where `target` can be drawn: its scissor, else the whole viewport.
#[allow(clippy::cast_precision_loss)] // target sizes are far below 2^24
pub(super) fn view(target: &Target<'_>) -> Rect {
    let Size { width, height } = target.viewport;
    target.scissor.map_or_else(
        || Rect::new(0.0, 0.0, width as f32, height as f32),
        |s| {
            Rect::new(
                s.x as f32,
                s.y as f32,
                (s.x + s.width) as f32,
                (s.y + s.height) as f32,
            )
        },
    )
}

/// A batch's clip on the page: `clip`, its clip inside `frame`, moved by
/// the frame's shift and cut to the frame's clip; `None` for none.
pub(super) fn on_page(clip: Option<Rect>, frame: FrameDraw) -> Option<Rect> {
    let clip = clip.map(|clip| clip.offset(frame.shift));
    match (clip, frame.clip) {
        (Some(clip), Some(outer)) => Some(clip.intersection(outer)),
        (clip, outer) => clip.or(outer),
    }
}

/// `target` cut to `clip`; `None` when nothing of it is left.
pub(super) fn clipped<'a>(target: &Target<'a>, clip: Option<Rect>) -> Option<Target<'a>> {
    let scissor = match region(target.scissor, clip, target.viewport) {
        Region::Whole => None,
        Region::Within(scissor) => Some(scissor),
        Region::Nowhere => return None,
    };
    Some(Target { scissor, ..*target })
}

#[cfg(test)]
#[path = "painter_region_tests.rs"]
mod tests;
