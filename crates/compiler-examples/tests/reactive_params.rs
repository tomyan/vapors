//! Snippet parameters are reactive (1-R1, R1b, review-v66): a snippet
//! reads its arguments as they change. `src/readout_rows.wisp` renders
//! a snippet at the top of each keyed row with the item's fields, and a
//! kept row whose item changes updates in place; `src/param_reads.wisp`
//! reads a `String` parameter twice, inside an `{if}` and as an
//! argument to a snippet declared after it. Compiled by build.rs and
//! driven by the recording Dom.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::param_places::{ParamPlaces, ParamPlacesProps};
use vapors_compiler_examples::param_reads::{ParamReads, ParamReadsProps};
use vapors_compiler_examples::readout_rows::{ReadoutRows, ReadoutRowsProps};
use vapors_dom::recording::{Call, RecordingDom};
use vapors_signals::Signal;

fn recording() -> Rc<RefCell<RecordingDom>> {
    Rc::new(RefCell::new(RecordingDom::new()))
}

#[test]
fn a_kept_row_whose_item_changes_updates_its_snippet_in_place() {
    // Given
    let dom = recording();
    let root = dom.borrow().root();
    let layers = Signal::new(vec![(1, 10), (2, 20)]);
    let _rows = ReadoutRows::mount(&dom, root, ReadoutRowsProps { layers });
    dom.borrow_mut().clear_calls();

    // When: the same keys, a new amount for the first
    layers.set(vec![(1, 15), (2, 20)]);

    // Then: one text set, nothing created
    assert_eq!(
        dom.borrow().to_html(),
        "<ul><li><b>1=15</b></li><li><b>2=20</b></li></ul>"
    );
    let calls = dom.borrow().calls().to_vec();
    assert!(
        calls
            .iter()
            .all(|call| matches!(call, Call::SetText { .. }))
            && calls.len() == 1,
        "{calls:?}"
    );
}

#[test]
fn a_string_parameter_reads_in_every_place_and_follows_its_argument() {
    // Given
    let dom = recording();
    let root = dom.borrow().root();
    let word = Signal::new(String::from("ab"));
    let _reads = ParamReads::mount(&dom, root, ParamReadsProps { word });

    // When
    word.set(String::from("abcd"));

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        "<b>abcd</b><i>abcd</i><u>abcd</u><s>abcd</s>"
    );
}

#[test]
fn a_parameter_read_in_a_for_an_attribute_and_an_if_updates_in_place() {
    // Given: `show(n)` and `show(3)`, from src/param_places.wisp
    let dom = recording();
    let root = dom.borrow().root();
    let n = Signal::new(1);
    let _places = ParamPlaces::mount(&dom, root, ParamPlacesProps { n });
    dom.borrow_mut().clear_calls();

    // When
    n.set(2);

    // Then: the first render follows `n`, its <p> kept; the second, at 3, is untouched
    assert_eq!(
        dom.borrow().to_html(),
        "<p title=\"2\"><i>0</i><i>1</i><b>many</b></p><p title=\"3\"><i>0</i><i>1</i><i>2</i><b>many</b></p>"
    );
    let created_p = dom
        .borrow()
        .calls()
        .iter()
        .any(|call| matches!(call, Call::CreateElement { tag, .. } if tag == "p"));
    assert!(!created_p, "{:?}", dom.borrow().calls());
}
