//! A local snippet passed as a prop by name (review-v66b, finding 2):
//! `src/snippet_as_prop.wisp` hands `b` to `LayerList` as its `row`
//! and renders `b` itself afterwards, which compiles because the
//! attribute takes a clone of the snippet.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::snippet_as_prop::{SnippetAsProp, SnippetAsPropProps};
use vapors_dom::recording::RecordingDom;

#[test]
fn a_snippet_passed_as_a_prop_still_renders_where_it_was_declared() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();

    // When
    let _host = SnippetAsProp::mount(&dom, root, SnippetAsPropProps::default());

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<ul><li><i>0a</i></li></ul><i>9z</i>"
    );
}
