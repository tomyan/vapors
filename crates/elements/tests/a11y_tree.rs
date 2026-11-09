//! The a11y tree a driver reads (R1): every rendered node with a role,
//! nested under its nearest ancestor with one, in document order, with
//! its name, value, states and border box; refused when the document
//! has changed since its last update.

mod support;

use support::roles_page::{STYLE, labelled, page};
use support::{document, viewport};
use vapors_dom::Dom;
use vapors_elements::{AxElement, AxError, BoxRect};

/// Each node as an indented line: role, name, value and the states set.
fn outline(nodes: &[AxElement], depth: usize, out: &mut Vec<String>) {
    for node in nodes {
        let value = node
            .value
            .as_deref()
            .map(|v| format!(" ={v}"))
            .unwrap_or_default();
        out.push(format!(
            "{}{} \"{}\"{value}{:?}",
            "  ".repeat(depth),
            node.role,
            node.name,
            node.states.set()
        ));
        outline(&node.children, depth + 1, out);
    }
}

fn lines(nodes: &[AxElement]) -> Vec<String> {
    let mut out = Vec::new();
    outline(nodes, 0, &mut out);
    out
}

#[test]
fn each_role_nests_under_its_nearest_ancestor_with_its_name_value_and_states() {
    // Given
    let doc = page();

    // When
    let tree = doc.ax_tree().expect("an updated document");

    // Then: the unnamed section is no region, so After sits in the dialog
    assert_eq!(
        lines(&tree),
        [
            "dialog \"Levels\"[]",
            "  group \"Tone\"[]",
            "    slider \"Amount\" =50[]",
            "    spinbutton \"Radius\" =2[]",
            "    textbox \"Note\" =hi[]",
            "  region \"Options\"[]",
            "    checkbox \"Mono\"[\"checked=false\"]",
            "    checkbox \"Lock\"[\"checked=true\", \"disabled=true\"]",
            "    button \"Pin\"[\"pressed=false\"]",
            "    button \"Apply\"[]",
            "    button \"Reset\"[\"disabled=true\"]",
            "  button \"After\"[]",
        ]
    );
}

#[test]
fn a_nodes_bounds_are_its_border_box_in_css_pixels() {
    // Given
    let doc = page();

    // When
    let tree = doc.ax_tree().expect("an updated document");

    // Then: the number field follows the 200 px range in the first row
    let radius = &tree[0].children[0].children[1];
    assert_eq!(
        radius.bounds,
        BoxRect {
            x: 200.0,
            y: 0.0,
            width: 80.0,
            height: 24.0
        }
    );
}

#[test]
fn a_node_not_drawn_or_hidden_from_the_tree_is_left_out_with_its_subtree() {
    // Given: a button under display: none, one under visibility: hidden,
    // one under aria-hidden, and one shown
    let mut doc = document();
    let body = doc.body();
    for (class, hidden) in [
        ("gone", false),
        ("unseen", false),
        ("", true),
        ("shown", false),
    ] {
        let row = labelled(&mut doc, body, "div", &[("class", &format!("row {class}"))]);
        if hidden {
            doc.set_attribute(row, "aria-hidden", "true");
        }
        support::roles_page::button(&mut doc, row, class, &[]);
    }
    let sheet =
        format!("{STYLE} .gone button {{ display: none; }} .unseen {{ visibility: hidden; }}");
    doc.add_stylesheet("hidden.css", &sheet).expect("parses");
    doc.update(viewport(640.0, 400.0)).expect("updates");

    // Then
    assert_eq!(
        lines(&doc.ax_tree().expect("updated")),
        ["button \"shown\"[]"]
    );
}

#[test]
fn a_document_changed_since_its_last_update_is_refused() {
    // Given: a name changed after the update
    let mut doc = page();
    let dialog = doc.children(doc.body())[0];
    doc.set_attribute(dialog, "aria-label", "Curves");

    // Then
    assert_eq!(doc.ax_tree().map(|_| ()), Err(AxError::NotUpdated));
}

#[test]
fn a_document_never_updated_is_refused() {
    assert_eq!(document().ax_tree().map(|_| ()), Err(AxError::NotUpdated));
}
