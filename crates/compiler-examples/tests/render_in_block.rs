//! A render argument read inside a block (review-v66b, finding 1):
//! `src/render_in_block.wisp` passes a `String` prop to a snippet
//! rendered inside an `{if}` and reads the prop again after it, which
//! compiles only when the `{if}`'s builder takes a clone of the prop.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::render_in_block::{RenderInBlock, RenderInBlockProps};
use vapors_dom::recording::RecordingDom;

#[test]
fn a_prop_passed_to_a_render_in_an_if_is_still_there_after_it() {
    // Given
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();

    // When
    let _block = RenderInBlock::mount(&dom, root, RenderInBlockProps::default());

    // Then
    assert_eq!(dom.borrow().to_html(), "<b>kept</b><p>kept</p>");
}
