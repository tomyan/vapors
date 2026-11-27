//! The clip each box paints and takes the pointer under (F2c S8): the
//! padding boxes of the `overflow: hidden` or `clip` boxes it is cut
//! off by, intersected. A box is cut off by what cuts off its
//! containing block's content: its parent's for a box in flow, the box
//! it is placed against for an absolute one, and nothing for a fixed
//! one, whose containing block is the viewport (box 0, the root, which
//! never clips).

use std::num::NonZeroU32;

use vapors_dom::NodeId;

use crate::clip_rounded::Met;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::PaintInput;

/// A box's clip, CSS pixels: a rectangle, rounded when `rounded_by`
/// names the box whose rounded padding box it is, whose radii it takes
/// (flat-scene F3.2b, `docs/plans/flat-scene-f3.md`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoxClip {
    /// The rectangle.
    pub(crate) rect: BoxRect,
    /// The slot of the box whose rounded padding box this is, plus 1, so
    /// a kept box's clip stays 24 bytes (`kept_box_size_tests.rs`).
    rounded_by: Option<NonZeroU32>,
}

impl BoxClip {
    /// `rect` with square corners.
    pub(crate) const fn square(rect: BoxRect) -> Self {
        Self {
            rect,
            rounded_by: None,
        }
    }

    /// `rect`, the padding box of box `slot`, rounded with its radii.
    ///
    /// # Panics
    ///
    /// When `slot` is past `u32::MAX - 1`.
    pub(crate) fn rounded(rect: BoxRect, slot: usize) -> Self {
        let named = u32::try_from(slot + 1).ok().and_then(NonZeroU32::new);
        let named = named.unwrap_or_else(|| {
            panic!("box slot {slot} is past what a clip can name (u32); reproduce with `cargo xtask test`")
        });
        Self {
            rect,
            rounded_by: Some(named),
        }
    }

    /// The slot of the box whose rounded padding box this is, if any.
    pub(crate) fn rounded_by(&self) -> Option<usize> {
        self.rounded_by.map(|slot| slot.get() as usize - 1)
    }

    /// The rectangle of a clip that must be square, as a scroll frame's
    /// clip is: a scissor cuts square.
    ///
    /// # Panics
    ///
    /// When the clip is rounded.
    pub(crate) fn square_rect(self) -> BoxRect {
        assert!(
            self.rounded_by.is_none(),
            "a rounded clip {self:?} reached a place only a square clip can go (a scroll \
             frame's clip, a hit's rectangle); reproduce with `cargo xtask test`"
        );
        self.rect
    }
}

/// The clip box `index` gives its content, from `own`, the clip it
/// paints under, CSS pixels, `None` for none, and whether the two
/// overlap as a rounded clip must not (`clip_rounded.rs`): when it cuts
/// off its content, its padding box, rounded by its own padding radii
/// ([`radii`]), or a scrolling text field's narrowed to its content box
/// on x ([`content_clip`]), met with `own`; and its own rounded padding
/// box, when it has one.
pub(crate) fn inner(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    own: Option<BoxClip>,
    input: &PaintInput<'_>,
) -> Inner {
    if !clipping(boxes, index, input) {
        return Inner {
            clip: own,
            overlaps: false,
            rounding: None,
        };
    }
    let radii_of = |slot| radii(boxes, rects, slot, input);
    let candidate = cut_to(boxes, rects, index, input);
    let rounding = candidate
        .rounded_by()
        .map(|slot| (candidate.rect, radii_of(slot)));
    let (clip, overlaps) = match crate::clip_rounded::meet(own, candidate, &radii_of) {
        Met::Clip(clip) => (clip, false),
        Met::Overlap(clip) => (clip, true),
    };
    Inner {
        clip: Some(clip),
        overlaps,
        rounding,
    }
}

/// What [`inner`] works out for a box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Inner {
    /// The clip it gives its content.
    pub(crate) clip: Option<BoxClip>,
    /// Whether that clip and the clip it is under overlap as a rounded
    /// clip must not.
    pub(crate) overlaps: bool,
    /// Its own padding box and padding radii, CSS pixels, when it rounds
    /// its content: hit testing cuts by them (`kept_hits_query.rs`).
    pub(crate) rounding: Option<(BoxRect, [f32; 4])>,
}

/// What clipping box `index` cuts its content to: its content clip,
/// rounded when it has padding radii and is not a text field scrolling
/// its text, whose clip on x is clear of its corners (`style/overflow.rs`
/// refuses one that is not).
fn cut_to(boxes: &[LayoutBox], rects: &[BoxRect], index: usize, input: &PaintInput<'_>) -> BoxClip {
    let layout_box = &boxes[index];
    let content = content_clip(layout_box, rects[index]);
    let scrolls_text = crate::scroll_reveal::scrolls_text(layout_box);
    if !scrolls_text && radii(boxes, rects, index, input).iter().any(|r| *r > 0.0) {
        BoxClip::rounded(content, index)
    } else {
        BoxClip::square(content)
    }
}

/// The padding radii of box `slot`, CSS pixels (`clip_rounded.rs`
/// `padding_radii`); all 0 for a box that is not an element.
///
/// # Panics
///
/// When the element has no used style: every element placed has one.
pub(crate) fn radii(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    slot: usize,
    input: &PaintInput<'_>,
) -> [f32; 4] {
    let Some(node) = boxes[slot].element_in(slot) else {
        return [0.0; 4];
    };
    let used = input.used.get(&node).unwrap_or_else(|| {
        panic!(
            "box slot {slot}'s element {node:?} has no used style, so its clip's radii are \
             unknown: every placed element is restyled first (restyle_used.rs); reproduce with \
             `cargo xtask test`"
        )
    });
    crate::clip_rounded::padding_radii(&boxes[slot], rects[slot], used.decoration.radii)
}

/// `rect` cut to `clip`; an empty rect where they do not meet.
pub(crate) fn within(clip: Option<BoxRect>, rect: BoxRect) -> BoxRect {
    let Some(clip) = clip else {
        return rect;
    };
    let x = rect.x.max(clip.x);
    let y = rect.y.max(clip.y);
    BoxRect {
        x,
        y,
        width: ((rect.x + rect.width).min(clip.x + clip.width) - x).max(0.0),
        height: ((rect.y + rect.height).min(clip.y + clip.height) - y).max(0.0),
    }
}

/// What a clipping box cuts its content off at, from its border box:
/// its padding box, or for a text field that scrolls on x its content
/// box on x, as Chromium cuts a field's text off where its inner editor
/// ends (entry 44); on y still its padding box.
pub(crate) fn content_clip(layout_box: &LayoutBox, border_box: BoxRect) -> BoxRect {
    let padding = padding_box(layout_box, border_box);
    if !crate::scroll_reveal::scrolls_text(layout_box) {
        return padding;
    }
    let content = content_box(layout_box, border_box);
    BoxRect {
        x: content.x,
        width: content.width,
        ..padding
    }
}

pub(crate) fn padding_box(layout_box: &LayoutBox, border_box: BoxRect) -> BoxRect {
    let border = layout_box.layout.border;
    BoxRect {
        x: border_box.x + border.left,
        y: border_box.y + border.top,
        width: (border_box.width - border.left - border.right).max(0.0),
        height: (border_box.height - border.top - border.bottom).max(0.0),
    }
}

/// The content box of `layout_box`, whose border box is `border_box`.
pub(crate) fn content_box(layout_box: &LayoutBox, border_box: BoxRect) -> BoxRect {
    let padding = padding_box(layout_box, border_box);
    let inner = layout_box.layout.padding;
    BoxRect {
        x: padding.x + inner.left,
        y: padding.y + inner.top,
        width: (padding.width - inner.left - inner.right).max(0.0),
        height: (padding.height - inner.top - inner.bottom).max(0.0),
    }
}

/// Whether box `index` cuts off its content. The root never does: its
/// `overflow` is the viewport's. Nor does the body unless the root's
/// `overflow` is not `visible`; otherwise the body's is the viewport's
/// (CSS Overflow 3 section 3.3).
pub(crate) fn clipping(boxes: &[LayoutBox], index: usize, input: &PaintInput<'_>) -> bool {
    let Some(node) = boxes[index].element_in(index) else {
        return false;
    };
    let clips = |node: NodeId| input.used.get(&node).is_some_and(|used| used.clips());
    let to_viewport =
        crate::style::to_viewport(node == input.root, node == input.body, !clips(input.root));
    clips(node) && !to_viewport
}
