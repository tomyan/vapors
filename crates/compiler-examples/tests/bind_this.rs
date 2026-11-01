//! `bind:this` (`src/refs.wisp`): an element's node in a signal while it
//! is mounted, set once the element is built, and `None` once it is
//! not; the signal never holds a removed node. Rows: `bind_this_rows.rs`.

#[path = "support/refs.rs"]
mod refs;

use std::cell::RefCell;
use std::rc::Rc;

use refs::{created, mounted, mounted_with_top};
use vapors_signals::{Effect, Signal};

#[test]
fn a_mounted_element_is_in_its_signal() {
    // Given
    let (dom, top) = (refs::new_dom(), Signal::new(None));

    // When
    let (refs, _) = mounted_with_top(&dom, &["a"], top);

    // Then
    assert_eq!(refs.first.get(), Some(created(&dom, "p")[0]));
    assert_eq!(top.get(), Some(created(&dom, "div")[0]));
}

#[test]
fn the_signal_is_set_once_the_element_has_its_children() {
    // Given: an effect that counts the bound div's children when set
    let top = Signal::new(None);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (into, tree) = (Rc::clone(&seen), refs::new_dom());
    let reader = Rc::clone(&tree);
    let _effect = Effect::new(move || {
        if let Some(node) = top.get() {
            into.borrow_mut().push(reader.borrow().children(node).len());
        }
    });

    // When: no batch runs at the top level, so the effect runs on the set
    let _refs = mounted_with_top(&tree, &["a"], top);

    // Then: it saw every child the div has now
    let div = created(&tree, "div")[0];
    let children = tree.borrow().children(div).len();
    assert!(children > 0);
    assert_eq!(*seen.borrow(), [children]);
}

#[test]
fn an_element_its_if_removes_leaves_none() {
    // Given
    let (_dom, refs, _) = mounted(&[]);

    // When
    refs.shown.set(false);

    // Then
    assert_eq!(refs.first.get(), None);
}

#[test]
fn an_effect_on_the_signal_never_sees_a_removed_node() {
    // Given: an effect that checks each node it sees is in the div
    let (dom, refs, _) = mounted(&[]);
    let div = created(&dom, "div")[0];
    let stale = Rc::new(RefCell::new(Vec::new()));
    let (into, reader, first) = (Rc::clone(&stale), Rc::clone(&dom), refs.first);
    let _effect = Effect::new(move || {
        if let Some(node) = first.get()
            && !reader.borrow().children(div).contains(&node)
        {
            into.borrow_mut().push(node);
        }
    });

    // When: the p goes, and comes back
    refs.shown.set(false);
    refs.shown.set(true);

    // Then
    assert!(stale.borrow().is_empty(), "{stale:?}");
    assert_eq!(refs.first.get(), Some(created(&dom, "p")[1]));
}

#[test]
fn unmounting_the_component_disposes_its_own_signals_without_a_panic() {
    // Given
    let (dom, refs, _) = mounted(&["a"]);
    let first = refs.first;

    // When: its cleanups run after its signals are disposed
    refs.unmount(&dom);

    // Then: the signal is gone, and nothing panicked clearing it
    assert!(first.try_get_untracked().is_err());
}
