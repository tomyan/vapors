//! `{render expr()}` (1-R1, R1d): `src/render_expr.wisp` renders one of
//! two snippets chosen by a signal, with an argument from another. The
//! snippet shown follows the choice; the same snippet again, though the
//! choice was set, rebuilds nothing; the argument stays reactive.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::render_expr::{RenderExpr, RenderExprProps};
use vapors_dom::recording::{Call, RecordingDom};
use vapors_signals::Signal;

fn mounted(pick: Signal<bool>, word: Signal<String>) -> (Rc<RefCell<RecordingDom>>, RenderExpr) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let rendered = RenderExpr::mount(&dom, root, RenderExprProps { pick, word });
    (dom, rendered)
}

#[test]
fn the_snippet_shown_follows_the_expression() {
    // Given
    let pick = Signal::new(true);
    let (dom, _r) = mounted(pick, Signal::new(String::from("hi")));
    assert_eq!(dom.borrow().to_html(), "<p><b>hi</b></p>");

    // When
    pick.set(false);

    // Then
    assert_eq!(dom.borrow().to_html(), "<p><i>hi</i></p>");
}

#[test]
fn the_same_snippet_again_rebuilds_nothing() {
    // Given
    let pick = Signal::new(true);
    let (dom, _r) = mounted(pick, Signal::new(String::from("hi")));
    dom.borrow_mut().clear_calls();

    // When: set to the value it has; the expression yields a clone of `bold`
    pick.set(true);

    // Then
    assert_eq!(dom.borrow().calls(), &[] as &[Call]);
}

#[test]
fn the_argument_updates_the_snippet_in_place() {
    // Given
    let word = Signal::new(String::from("hi"));
    let (dom, _r) = mounted(Signal::new(true), word);
    dom.borrow_mut().clear_calls();

    // When
    word.set(String::from("yo"));

    // Then: one text set
    assert_eq!(dom.borrow().to_html(), "<p><b>yo</b></p>");
    assert_eq!(dom.borrow().calls().len(), 1, "{:?}", dom.borrow().calls());
}
