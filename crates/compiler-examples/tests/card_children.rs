//! Children inside a component tag (1-R1, R1a): `src/card_host.wisp`
//! passes `<p>` and `<b>` to `src/card.wisp`, which places them with
//! `{render children()}` between its title and its footer (an `{if}` among
//! them), and `<u>` to `src/frame.wisp`, which renders them at its top
//! level and forwards into a `Card` inside an `{if}`; compiled by
//! build.rs and driven by the recording Dom.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::card_host::{CardHost, CardHostProps};
use vapors_dom::recording::RecordingDom;

fn mounted() -> (Rc<RefCell<RecordingDom>>, CardHost) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let host = CardHost::mount(&dom, root, CardHostProps::default());
    (dom, host)
}

#[test]
fn children_render_where_the_child_renders_them() {
    // When
    let (dom, _host) = mounted();

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<section><h2>Curves</h2><p>count 0</p><b>static</b><footer>end</footer></section><u>framed 0</u><i>after</i><section><h2>inner</h2><u>framed 0</u><footer>end</footer></section>"
    );
}

#[test]
fn a_parent_signal_updates_inside_the_children() {
    // Given
    let (dom, host) = mounted();

    // When
    host.count.set(3);

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<section><h2>Curves</h2><p>count 3</p><em>many</em><b>static</b><footer>end</footer></section><u>framed 3</u><i>after</i><section><h2>inner</h2><u>framed 3</u><footer>end</footer></section>"
    );
}

#[test]
fn unmounting_the_parent_removes_the_children_with_the_children_rendered_at_the_top() {
    // Given
    let (dom, host) = mounted();

    // When
    host.unmount(&dom);

    // Then
    assert_eq!(dom.borrow().to_html(), "");
}
