//! Two rows with one key: refused naming the key and the `{for}`'s
//! position in the `.wisp`, before any row is touched, and the block
//! and the runtime keep working afterwards.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use vapors_dom::blocks::keyed;
use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dom, NodeId, Root};
use vapors_signals::{Owner, Signal};

type Shared = Rc<RefCell<RecordingDom>>;

const SITE: &str = "src/list.wisp:4:5";

fn mounted(items: Signal<Vec<(u32, String)>>) -> Shared {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let anchor = dom.borrow_mut().create_text("");
    dom.borrow_mut().append(root, anchor);
    keyed(&dom, root, anchor, SITE, move || items.get(), row);
    dom
}

fn row(
    dom: &Shared,
    parent: NodeId,
    before: Option<NodeId>,
    _: Signal<usize>,
    item: Signal<String>,
) -> Vec<Root> {
    let li = dom.borrow_mut().create_element("li");
    dom.borrow_mut().insert_before(parent, li, before);
    let text = dom.borrow_mut().create_text(&item.get_untracked());
    dom.borrow_mut().append(li, text);
    vec![Root::Node(li)]
}

fn rows(entries: &[(u32, &str)]) -> Vec<(u32, String)> {
    entries
        .iter()
        .map(|(key, text)| (*key, (*text).to_string()))
        .collect()
}

fn panic_message(caught: &(dyn std::any::Any + Send)) -> String {
    caught
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| caught.downcast_ref::<&str>().map(ToString::to_string))
        .unwrap_or_default()
}

#[test]
fn a_duplicate_key_is_refused_naming_the_key_and_the_for() {
    // Given
    let owner = Owner::new();
    owner.set();
    let items = Signal::new(rows(&[(1, "a")]));
    let _dom = mounted(items);

    // When
    let caught = catch_unwind(AssertUnwindSafe(|| items.set(rows(&[(7, "a"), (7, "b")]))));

    // Then
    let message = panic_message(&*caught.expect_err("two rows share key 7"));
    assert!(message.contains(SITE), "{message}");
    assert!(message.contains("the key 7"), "{message}");
    assert!(message.contains("key="), "names the fix: {message}");
}

#[test]
fn after_a_refused_update_the_rows_are_unchanged_and_the_next_update_applies() {
    // Given
    let owner = Owner::new();
    owner.set();
    let items = Signal::new(rows(&[(1, "a"), (2, "b")]));
    let dom = mounted(items);

    // When
    let caught = catch_unwind(AssertUnwindSafe(|| items.set(rows(&[(3, "c"), (3, "d")]))));
    let after_refusal = dom.borrow().to_html();
    items.set(rows(&[(2, "b"), (4, "d")]));

    // Then
    assert!(caught.is_err());
    assert_eq!(after_refusal, "<li>a</li><li>b</li>");
    assert_eq!(dom.borrow().to_html(), "<li>b</li><li>d</li>");
}
