//! The a11y projection's first slice (5-A1's start): a labelled (by
//! `aria-labelledby`)
//! combobox, its listbox and options, as Chromium's accessibility tree
//! writes them (the corpus's `combobox.events` compares the two): a
//! hidden listbox is left out with the combobox's `controls`, an open
//! one is there with the active option as `activedescendant`.

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxNode, Document};

/// "State": a combobox controlling a listbox of two options, the
/// listbox shown only while the combobox is expanded.
fn combobox(expanded: bool) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let label = element(&mut doc, body, "span", "");
    doc.set_attribute(label, "id", "label");
    text(&mut doc, label, "State");
    let input = element(&mut doc, body, "input", "");
    for (name, value) in [
        ("id", "cb"),
        ("role", "combobox"),
        ("aria-labelledby", "label"),
        ("aria-controls", "lb"),
        ("aria-expanded", if expanded { "true" } else { "false" }),
    ] {
        doc.set_attribute(input, name, value);
    }
    if expanded {
        doc.set_attribute(input, "aria-activedescendant", "o2");
    }
    let list = element(
        &mut doc,
        body,
        "div",
        if expanded { "list open" } else { "list" },
    );
    for (name, value) in [("id", "lb"), ("role", "listbox"), ("aria-label", "States")] {
        doc.set_attribute(list, name, value);
    }
    for (id, name, selected) in [("o1", "Alabama", false), ("o2", "Alaska", true)] {
        let option = element(&mut doc, list, "div", "");
        doc.set_attribute(option, "id", id);
        doc.set_attribute(option, "role", "option");
        if selected && expanded {
            doc.set_attribute(option, "aria-selected", "true");
        }
        text(&mut doc, option, name);
    }
    doc.add_stylesheet(
        "app.css",
        "body { display: flex; } input { width: 100px; } .list { display: none; } .list.open { display: block; }",
    )
    .expect("parses");
    doc.update(viewport(400.0, 300.0)).expect("updates");
    (doc, input)
}

fn node(role: &str, name: &str, properties: &[&str]) -> AxNode {
    AxNode {
        role: role.into(),
        name: name.into(),
        value: None,
        description: None,
        properties: properties.iter().map(|p| (*p).to_owned()).collect(),
    }
}

const ROLES: [&str; 3] = ["combobox", "listbox", "option"];

#[test]
fn a_closed_combobox_is_alone_without_controls() {
    // Given
    let (doc, _) = combobox(false);

    // Then: the listbox is not rendered, so neither it nor `controls` is there
    assert_eq!(
        doc.ax_projection(&ROLES),
        Ok(vec![node("combobox", "State", &["expanded=false"])])
    );
}

#[test]
fn an_open_combobox_names_its_active_option_and_listbox() {
    // Given: open, Alaska active, focused
    let (doc, input) = combobox(true);
    let doc = std::rc::Rc::new(std::cell::RefCell::new(doc));
    Document::focus(&doc, Some(input));
    doc.borrow_mut()
        .update(viewport(400.0, 300.0))
        .expect("updates");

    // Then
    assert_eq!(
        doc.borrow().ax_projection(&ROLES),
        Ok(vec![
            node(
                "combobox",
                "State",
                &[
                    "activedescendant=option \"Alaska\"",
                    "controls=listbox \"States\"",
                    "expanded=true",
                    "focused=true"
                ]
            ),
            node("listbox", "States", &[]),
            node("option", "Alabama", &[]),
            node("option", "Alaska", &["selected=true"]),
        ])
    );
}

#[test]
fn a_document_never_updated_is_refused() {
    // Given
    let doc = document();

    // Then
    assert!(doc.ax_projection(&ROLES).is_err());
}

#[test]
fn a_projection_asked_for_aria_img_is_refused_naming_chromiums_image() {
    // Given
    let (doc, _) = combobox(false);

    // When
    let refusal = doc.ax_projection(&["img"]).expect_err("refused");

    // Then
    assert_eq!(
        refusal.to_string(),
        "the projection asked for `img`, ARIA's name; fix: use `image`, Chromium's name for ARIA img"
    );
}
