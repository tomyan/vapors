//! A component tag inside a snippet body (virtual list V2b met its lack):
//! `src/snippet_component.wisp` mounts `RequiredLabel` from a snippet
//! passed to `LayerList` and from one rendered in place. A snippet's body
//! is built against `dyn Dom`, so the child's mount must be too.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::snippet_component::{SnippetComponent, SnippetComponentProps};
use vapors_dom::recording::RecordingDom;

#[test]
fn a_component_in_a_snippet_mounts_where_the_snippet_renders() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();

    // When
    let _host = SnippetComponent::mount(&dom, root, SnippetComponentProps::default());

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<ul><li><b>Curves 2</b></li></ul><b>badge 3</b>"
    );
}
