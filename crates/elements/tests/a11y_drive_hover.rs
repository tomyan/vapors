//! An action that happened is reported as done (R1), even when the
//! tree after it cannot be read: the reply says why the tree is
//! unavailable, so a driver does not retry the action.

mod support;

use support::drive::{log, node, shared, target};
use support::roles_page::page;
use vapors_elements::{AxError, Command, Document, Reply};

#[test]
fn a_press_whose_tree_hover_keeps_changing_is_done_without_a_tree() {
    // Given: hovering Mono moves it from under the pointer, and back
    let mut page = page();
    let shift = "input[type=checkbox]:hover { margin-left: 40px; }";
    page.add_stylesheet("hover.css", shift).expect("parses");
    page.update(support::viewport(640.0, 400.0))
        .expect("updates");
    let doc = shared(page);
    let mono = node(&doc, "checkbox", "Mono");
    let changes = log(&doc, mono, &["change"]);

    // When
    let reply = Document::drive(&doc, &Command::Press(target("checkbox", "Mono")));

    // Then: checked once, and the reply says so, with why there is no tree
    assert_eq!(*changes.borrow(), ["change"]);
    let Reply::Done {
        nodes: None,
        problem: Some(AxError::NotUpdated),
        ..
    } = reply
    else {
        panic!("done without a tree expected, got {reply:?}")
    };
}
