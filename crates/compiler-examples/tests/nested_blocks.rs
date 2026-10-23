//! A block at the top of a branch, a row or a component: the enclosing
//! fragment owns every node the inner block inserts, so removing or
//! moving the fragment removes or moves them all.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::block_root::{BlockRoot, BlockRootProps};
use vapors_compiler_examples::nested_for::{NestedFor, NestedForProps};
use vapors_compiler_examples::nested_if::{NestedIf, NestedIfProps};
use vapors_dom::recording::RecordingDom;

fn dom() -> Rc<RefCell<RecordingDom>> {
    Rc::new(RefCell::new(RecordingDom::new()))
}

#[test]
fn removing_a_branch_removes_the_nodes_of_an_if_at_its_top() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let component = NestedIf::mount(&dom, root, NestedIfProps::default());
    assert_eq!(dom.borrow().to_html(), "<div><p>inner</p></div>");

    // When
    component.outer.set(false);

    // Then
    assert_eq!(dom.borrow().to_html(), "<div><span>else</span></div>");
}

#[test]
fn reordering_rows_moves_the_nodes_of_an_if_at_their_top() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let component = NestedFor::mount(&dom, root, NestedForProps::default());

    // When
    component.rows.set(vec![3, 1, 2]);

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<ul><li>3</li><li>1</li><li>2</li></ul>"
    );
}

#[test]
fn clearing_rows_removes_the_nodes_of_an_if_at_their_top() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let component = NestedFor::mount(&dom, root, NestedForProps::default());

    // When
    component.rows.set(Vec::new());

    // Then
    assert_eq!(dom.borrow().to_html(), "<ul></ul>");
}

#[test]
fn a_component_whose_root_is_a_block_lists_and_removes_its_nodes() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();
    let component = BlockRoot::mount(&dom, root, BlockRootProps::default());
    let paragraph = dom.borrow().children(root)[0];

    // When
    let roots = component.roots();
    component.unmount(&dom);

    // Then
    assert!(
        roots.contains(&paragraph),
        "roots include the block's nodes: {roots:?}"
    );
    assert_eq!(dom.borrow().to_html(), "");
    assert_eq!(dom.borrow().children(root), Vec::new());
}
