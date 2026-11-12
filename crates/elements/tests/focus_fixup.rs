//! The focus fixup: a focused text field that loses its box (itself,
//! an ancestor or html `display: none`, or `visibility: hidden`) or is
//! disabled loses focus at the settle after the update that shows it.
//! The field commits (`change`), then `blur` and `focusout` fire, and
//! typing afterwards goes nowhere.
//!
//! Chromium 155.0.8059.79 in each case, after the mutation and within
//! 300 ms: `change`, `blur`, `focusout` on the field, still connected,
//! then a typed key reaches the body and the value stays. During
//! `change` focus is already on the body, in Chromium and here.
//!
//! A key sent straight after the mutation differs: Chromium dispatches
//! its `keydown` to the still-focused field and inserts nothing, the
//! field being uneditable without a box. Here a key routed before the
//! next update edits the field, since whether it has a box is known
//! only at the update; matching needs a style and layout pass before
//! each key is routed, so it is recorded, not matched.
//!
//! Removal differs from Chromium: removing the focused field (or an
//! ancestor) clears focus here with no events (`drop_focus_within`),
//! while Chromium 155 fires `change`, `blur`, `focusout` on it during
//! the `remove()` call, before it is detached (`isConnected` true), so
//! capture listeners on the document see them. Matching it needs the
//! remove path to dispatch, which `Dom::remove` (`&mut self`) cannot.
//!
//! Rerun Chromium's side: `node scripts/chromium-focus-fixup.mjs`.

mod support;

use support::fixup::{typed_in, update_and_settle};
use support::text_page::{Page, type_text, value};
use support::viewport;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// The ways the field stops being focusable, each by name.
const LOSSES: [(&str, Loss); 5] = [
    ("self display: none", |page| hide(page, page.name)),
    ("ancestor display: none", |page| {
        let body = page.doc.borrow().body();
        hide(page, body);
    }),
    ("html display: none", |page| {
        let root = page.doc.borrow().root();
        hide(page, root);
    }),
    ("visibility: hidden", |page| {
        let mut doc = page.doc.borrow_mut();
        doc.set_attribute(page.name, "style", "visibility: hidden");
    }),
    ("disabled", |page| {
        page.doc
            .borrow_mut()
            .set_attribute(page.name, "disabled", "");
    }),
];

type Loss = fn(&Page);

fn hide(page: &Page, node: NodeId) {
    let mut doc = page.doc.borrow_mut();
    doc.set_attribute(node, "style", "display: none");
}

#[test]
fn a_field_that_loses_its_box_or_is_disabled_commits_then_blurs() {
    for (name, lose) in LOSSES {
        // Given
        let page = typed_in();

        // When
        lose(&page);
        update_and_settle(&page);

        // Then: change, blur, focusout once each, and the body has focus
        let log = page.log.borrow().clone();
        assert_eq!(log, ["change \"erWarm\"", "blur", "focusout"], "{name}");
        assert_eq!(page.doc.borrow().active_element(), None, "{name}");
    }
}

#[test]
fn typing_after_the_fixup_goes_nowhere() {
    for (name, lose) in LOSSES {
        // Given
        let page = typed_in();
        lose(&page);
        update_and_settle(&page);
        page.log.borrow_mut().clear();

        // When
        type_text(&page, "xy");

        // Then: no edit, and the platform has no text input to show
        assert_eq!(value(&page, page.name), "erWarm", "{name}");
        assert!(page.log.borrow().is_empty(), "{name}: {:?}", page.log);
        assert_eq!(page.doc.borrow_mut().text_input(), None, "{name}");
    }
}

#[test]
fn a_field_still_shown_and_enabled_keeps_focus_and_fires_nothing() {
    // Given
    let page = typed_in();
    let mut doc = page.doc.borrow_mut();
    doc.set_attribute(page.name, "style", "color: #ff0000");
    drop(doc);

    // When
    update_and_settle(&page);

    // Then
    assert!(page.log.borrow().is_empty(), "{:?}", page.log);
    assert_eq!(page.doc.borrow().active_element(), Some(page.name));
}

#[test]
fn a_refused_update_takes_no_focus_away() {
    // Given: the field hidden in an update the tree refuses, so
    // whether it has a box is unknown
    let page = typed_in();
    let mut doc = page.doc.borrow_mut();
    doc.set_attribute(page.name, "style", "display: none; float: left");
    let refused = doc.update(viewport(400.0, 100.0)).is_err();
    drop(doc);

    // When
    Document::settle_focus(&page.doc);

    // Then
    assert!(refused, "the float was expected to be refused");
    assert!(page.log.borrow().is_empty(), "{:?}", page.log);
    assert_eq!(page.doc.borrow().active_element(), Some(page.name));
}

#[test]
fn focus_and_focus_within_styles_clear_at_the_update_after_the_fixup() {
    // Given: :focus and :focus-within coloured, the field focused
    let page = typed_in();
    let sheet = "input:focus { background-color: #00ff00; } \
                 body:focus-within input { border-color: #0000ff; }";
    let mut doc = page.doc.borrow_mut();
    doc.add_stylesheet("states.css", sheet).expect("parses");
    drop(doc);
    update_and_settle(&page);
    let focused = colours(&page);

    // When: disabled, so the field stays drawn
    page.doc
        .borrow_mut()
        .set_attribute(page.name, "disabled", "");
    update_and_settle(&page);
    update_and_settle(&page);

    // Then
    assert!(focused.contains(&[0, 255, 0, 255]), "{focused:?}");
    let blurred = colours(&page);
    assert!(!blurred.contains(&[0, 255, 0, 255]), "{blurred:?}");
    assert!(!blurred.contains(&[0, 0, 255, 255]), "{blurred:?}");
}

/// Every quad's background and border colours in the last update.
fn colours(page: &Page) -> Vec<[u8; 4]> {
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("painted");
    painted
        .scene()
        .to_scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            vapors_scene::Item::Quad(quad) => Some(quad),
            _ => None,
        })
        .flat_map(|quad| [quad.background.to_rgba8(), quad.border_colors[0].to_rgba8()])
        .collect()
}
