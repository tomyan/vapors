//! Named snippets with parameters (1-R1, R1b): `src/layer_host.wisp`
//! declares `badge(label)` and renders it before a list, inside an
//! `{if}` and after it, and
//! declares `row(name, index)` inside the `<LayerList>` tag, which
//! makes it `LayerList`'s `row` prop; `src/layer_list.wisp` renders it
//! once per item of a `{for}`. Compiled by build.rs and driven by the
//! recording Dom.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::layer_host::{LayerHost, LayerHostProps};
use vapors_dom::recording::RecordingDom;

fn mounted() -> (Rc<RefCell<RecordingDom>>, LayerHost) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let host = LayerHost::mount(&dom, root, LayerHostProps::default());
    (dom, host)
}

#[test]
fn a_snippet_renders_with_its_arguments_wherever_it_is_rendered() {
    // When
    let (dom, _host) = mounted();

    // Then: badge before its declaration's neighbours too, and row per item
    assert_eq!(
        dom.borrow().to_html(),
        "<em>first</em><ul><li><b>0: Curves at 0</b></li><li><b>1: Levels at 0</b></li></ul><em>last</em>"
    );
}

#[test]
fn a_parent_signal_read_in_a_passed_snippet_updates_every_row() {
    // Given
    let (dom, host) = mounted();

    // When
    host.amount.set(5);

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<em>first</em><ul><li><b>0: Curves at 5</b></li><li><b>1: Levels at 5</b></li></ul><em>more</em><em>last</em>"
    );
}

#[test]
fn unmounting_removes_every_rendered_snippet() {
    // Given
    let (dom, host) = mounted();

    // When
    host.unmount(&dom);

    // Then
    assert_eq!(dom.borrow().to_html(), "");
}
