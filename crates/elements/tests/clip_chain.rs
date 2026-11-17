//! A box's clip chain: its own clip, and one link per enclosing scroll
//! container (flat-scene F3, `docs/plans/flat-scene-f3.md` item 6). A
//! chain past `MAX_CLIP_CHAIN` is refused once, naming the first box in
//! tree order that is past it.

use crate::support;

use support::{document, element, viewport};
use vapors_elements::{Document, MAX_CLIP_CHAIN};

/// `depth` scroll containers, each inside the last, and a leaf in the
/// innermost.
fn nested_scrollers(depth: usize) -> Document {
    nested_scrollers_holding(depth, &["leaf"])
}

/// `depth` scroll containers, each inside the last, and the innermost
/// holding a box of each class in `leaves`.
fn nested_scrollers_holding(depth: usize, leaves: &[&str]) -> Document {
    let mut doc = document();
    let mut parent = doc.body();
    for _ in 0..depth {
        parent = element(&mut doc, parent, "div", "scroller");
    }
    for class in leaves {
        element(&mut doc, parent, "div", class);
    }
    let css = ".scroller { overflow: auto; height: 50px; } .leaf, .second { height: 100px; }";
    doc.add_stylesheet("t.css", css).expect("parses");
    doc
}

#[test]
fn a_box_inside_three_scroll_containers_has_a_chain_of_three() {
    // When
    let mut doc = nested_scrollers(3);
    let painted = doc.update(viewport(640.0, 400.0)).expect("updates");

    // Then
    assert_eq!(painted.longest_clip_chain(), 3);
}

#[test]
fn a_chain_at_the_limit_updates() {
    // When
    let mut doc = nested_scrollers(MAX_CLIP_CHAIN);
    let updated = doc
        .update(viewport(640.0, 400.0))
        .map(vapors_elements::Painted::longest_clip_chain);

    // Then
    assert_eq!(updated.map_err(|e| format!("{e:?}")), Ok(MAX_CLIP_CHAIN));
}

#[test]
fn a_chain_past_the_limit_is_refused_once_naming_the_box() {
    // When
    let mut doc = nested_scrollers(MAX_CLIP_CHAIN + 1);
    let errors = doc.update(viewport(640.0, 400.0)).err().expect("refused");

    // Then
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].starts_with("<div class=\"leaf\"> (box slot "),
        "{}",
        messages[0]
    );
    assert!(
        messages[0].contains(&format!("has {} links", MAX_CLIP_CHAIN + 1)),
        "{}",
        messages[0]
    );
    assert!(messages[0].contains("reproduce: "), "{}", messages[0]);
}

/// Two hundred rows as the rows page's, under body (the lane's container),
/// with `clip` added, updated once: the longest clip chain, and whether
/// the lane was framed.
fn framed_rows(clip: &str) -> (usize, bool) {
    use support::rows_page::{row, rows_viewport};
    let mut doc = document();
    for index in 0..200 {
        row(&mut doc, index);
    }
    let css = format!(
        "body {{ margin: 0; }} .row {{ display: flex; height: 24px; gap: 8px; }} \
         .value {{ width: 80px; height: 20px; overflow: hidden; }} \
         button {{ width: 60px; height: 20px; }} {clip}"
    );
    doc.add_stylesheet("rows.css", &css).expect("parses");
    let longest = doc
        .update(rows_viewport())
        .expect("updates")
        .longest_clip_chain();
    (longest, doc.update_counts().lanes_framed > 0)
}

#[test]
fn a_lane_row_under_a_clip_adds_a_link_and_one_under_none_does_not() {
    // When: a value's text, under the value's own clip, in a framed row
    // under body, which clips (when the root's overflow is not visible,
    // clip.rs) or does not
    let clipped = framed_rows("html { overflow: hidden; } body { overflow: hidden; }");
    let unclipped = framed_rows("");

    // Then: the row's frame under body's clip is one more link
    assert_eq!((clipped, unclipped), ((2, true), (1, true)));
}

#[test]
fn two_boxes_past_the_limit_are_refused_once_naming_the_first_and_counting_the_other() {
    // Given: two leaves in the innermost of the nested scrollers
    let mut doc = nested_scrollers_holding(MAX_CLIP_CHAIN + 1, &["leaf", "second"]);

    // When
    let errors = doc.update(viewport(640.0, 400.0)).err().expect("refused");

    // Then
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].starts_with("<div class=\"leaf\"> (box slot "),
        "{}",
        messages[0]
    );
    assert!(
        messages[0].contains("; 1 other box is past it too;"),
        "{}",
        messages[0]
    );
}
