//! The cause of a refused paint (`paint_refusals.rs`): the first box in
//! tree order whose own layout length is past f32's range (a width, a
//! margin, an inset), which may place other boxes at infinity; else the
//! first box, in tree order, that drew a coordinate that is not finite,
//! with what drew it.

use taffy::{CompactLength, Style};

use crate::layout::Placed;
use crate::paint_refusals::{NotPlaced, PartKind};

/// The box a refusal names and why.
pub(crate) struct Cause {
    pub(crate) slot: usize,
    what: What,
}

enum What {
    /// One of its own lengths, by property name.
    Length(&'static str),
    /// A part it drew.
    Drew(PartKind),
}

/// The cause of `unplaced`; `None` when it is empty.
pub(crate) fn cause(unplaced: &[NotPlaced], placed: &Placed) -> Option<Cause> {
    if unplaced.is_empty() {
        return None;
    }
    let order = tree_order(placed);
    let by_length = order.iter().find_map(|&slot| {
        own_length(&placed.boxes[slot].style).map(|name| Cause {
            slot,
            what: What::Length(name),
        })
    });
    by_length.or_else(|| {
        let first = order
            .iter()
            .find_map(|&slot| unplaced.iter().find(|n| n.slot == slot))
            .unwrap_or(&unplaced[0]);
        Some(Cause {
            slot: first.slot,
            what: What::Drew(first.part),
        })
    })
}

impl Cause {
    /// The refusal's text for the box `element`, at `scale`, with
    /// `others` more boxes not placed.
    pub(crate) fn message(&self, element: &str, scale: f32, others: usize) -> String {
        let (held, property) = match self.what {
            What::Length(name) => (format!("its `{name}` is past f32's range, which"), name),
            What::Drew(part) => drew(part),
        };
        let also = match others {
            0 => String::new(),
            n => format!("; {n} other boxes it reaches are not placed either"),
        };
        format!(
            "{element} (box slot {}): {held} puts a coordinate that is not finite in device \
             pixels at scale {scale} into its paint; paint refuses the update rather than draw \
             it{also}; fix: keep `{property}` within f32's range at that scale (about 3.4e38 \
             device px); reproduce: update the document at scale {scale}, as `cargo test -p \
             vapors-elements --test tests paint_finite` does for each form",
            self.slot
        )
    }
}

fn drew(part: PartKind) -> (String, &'static str) {
    let (what, property) = match part {
        PartKind::BorderBox => ("its border box", "width"),
        PartKind::Outline => (
            "its outline ring (`outline-width` or `outline-offset`)",
            "outline-width",
        ),
        PartKind::Thumb => ("its scrollbar thumbs", "width"),
        PartKind::Own("shadow") => ("its shadow (`box-shadow`)", "box-shadow"),
        PartKind::Own("clip") => ("an ancestor's clip", "width"),
        PartKind::Own(kind) => return (format!("its {kind} item"), "width"),
    };
    (what.to_owned(), property)
}

/// Every box's slot, root first, children in order.
fn tree_order(placed: &Placed) -> Vec<usize> {
    let mut order = Vec::new();
    let mut stack = vec![0];
    while let Some(slot) = stack.pop() {
        order.push(slot);
        stack.extend(placed.boxes[slot].children.iter().rev());
    }
    order
}

/// The first of `style`'s own lengths that is past f32's range, by
/// property name: sizes, margins, padding, borders, insets, gaps and
/// the flex basis. A max size is left out: an infinite one limits
/// nothing.
fn own_length(style: &Style) -> Option<&'static str> {
    let rect = |r: taffy::Rect<CompactLength>, names: [&'static str; 4]| {
        [r.left, r.right, r.top, r.bottom].into_iter().zip(names)
    };
    let named = [
        (style.size.width.into_raw(), "width"),
        (style.size.height.into_raw(), "height"),
        (style.min_size.width.into_raw(), "min-width"),
        (style.min_size.height.into_raw(), "min-height"),
        (style.flex_basis.into_raw(), "flex-basis"),
        (style.gap.width.into_raw(), "column-gap"),
        (style.gap.height.into_raw(), "row-gap"),
    ]
    .into_iter()
    .chain(rect(
        style.margin.map(taffy::LengthPercentageAuto::into_raw),
        ["margin-left", "margin-right", "margin-top", "margin-bottom"],
    ))
    .chain(rect(
        style.padding.map(taffy::LengthPercentage::into_raw),
        [
            "padding-left",
            "padding-right",
            "padding-top",
            "padding-bottom",
        ],
    ))
    .chain(rect(
        style.border.map(taffy::LengthPercentage::into_raw),
        [
            "border-left-width",
            "border-right-width",
            "border-top-width",
            "border-bottom-width",
        ],
    ))
    .chain(rect(
        style.inset.map(taffy::LengthPercentageAuto::into_raw),
        ["left", "right", "top", "bottom"],
    ));
    named
        .filter(|(length, _)| length.is_length_or_percentage() && !length.value().is_finite())
        .map(|(_, name)| name)
        .next()
}

#[cfg(test)]
#[path = "paint_cause_tests.rs"]
mod tests;
