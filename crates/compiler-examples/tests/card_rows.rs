//! A `{for}` whose rows read the item's fields and render a snippet
//! (1-R1, R1b): `src/card_rows.wisp` titles a `Card` per layer with
//! `layer.get().1`, so the row builder needs the item type that only
//! the items closure tells the compiler, while the snippet the row
//! renders is cloned into it. Compiled by build.rs and driven by the
//! recording Dom.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::card_rows::{CardRows, CardRowsProps};
use vapors_dom::recording::RecordingDom;
use vapors_signals::Signal;

#[test]
fn rows_reading_the_items_fields_render_a_snippet_per_item() {
    // Given
    let layers = Signal::new(vec![
        (1, String::from("Curves")),
        (2, String::from("Levels")),
    ]);
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();

    // When
    let _rows = CardRows::mount(&dom, root, CardRowsProps { layers });

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<section><h2>Curves</h2><i>1</i><footer>end</footer></section><section><h2>Levels</h2><i>2</i><footer>end</footer></section>"
    );
}
