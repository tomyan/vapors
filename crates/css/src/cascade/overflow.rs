//! `overflow-x` and `overflow-y` computed together (CSS Overflow 3
//! section 3.1): when one axis is neither `visible` nor `clip`, a
//! `visible` on the other computes to `auto` and a `clip` to `hidden`.

use crate::property::PropertyId;
use crate::value::Value;

/// The computed pair for the specified `x` and `y` keywords: the one
/// table both the cascade and its tests read.
#[must_use]
pub fn computed_pair<'a>(x: &'a str, y: &'a str) -> (&'a str, &'a str) {
    if no_scroller(x) && no_scroller(y) {
        return (x, y);
    }
    (scrolling_axis(x), scrolling_axis(y))
}

/// `visible` and `clip` never make a scroll container.
fn no_scroller(keyword: &str) -> bool {
    matches!(keyword, "visible" | "clip")
}

fn scrolling_axis(keyword: &str) -> &str {
    match keyword {
        "visible" => "auto",
        "clip" => "hidden",
        other => other,
    }
}

/// Replaces the computed pair in `values` with [`computed_pair`]'s.
pub(super) fn resolve_pair(values: &mut [Value]) {
    let ids = ["overflow-x", "overflow-y"].map(|n| PropertyId::from_name(n).expect("table"));
    let [Some(x), Some(y)] = ids.map(|id| values[id.index()].as_keyword().map(str::to_owned))
    else {
        return;
    };
    let (cx, cy) = computed_pair(&x, &y);
    values[ids[0].index()] = Value::Keyword(cx.to_owned());
    values[ids[1].index()] = Value::Keyword(cy.to_owned());
}
