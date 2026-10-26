//! `{if}`: one branch is live at a time, rebuilt only when the
//! condition changes value.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Effect, Owner};

use super::Fragment;
use crate::{BlockNodes, Dom, NodeId, Root};

/// The branch shown, and which one it is.
type Shown = Rc<RefCell<Option<(bool, Fragment)>>>;

/// Keeps the branch matching `condition` built before `anchor`. The
/// builders receive the dom, the parent and the node to insert before.
/// Returns the block's current nodes, for whatever owns its position.
pub fn conditional<D, C, T, E>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    anchor: NodeId,
    condition: C,
    then: T,
    otherwise: E,
) -> BlockNodes
where
    D: Dom + ?Sized + 'static,
    C: Fn() -> bool + 'static,
    T: Fn(&Rc<RefCell<D>>, NodeId, Option<NodeId>) -> Vec<Root> + 'static,
    E: Fn(&Rc<RefCell<D>>, NodeId, Option<NodeId>) -> Vec<Root> + 'static,
{
    let dom = Rc::clone(dom);
    let block_owner = Owner::new();
    let shown: Shown = Rc::new(RefCell::new(None));
    let state = Rc::clone(&shown);
    Effect::new(move || {
        let now = condition();
        if state.borrow().as_ref().is_some_and(|(was, _)| *was == now) {
            return;
        }
        replace_branch(&dom, &state, now, |dom| {
            block_owner.with(|| {
                Fragment::build(dom, parent, Some(anchor), |dom, parent, before| {
                    if now {
                        then(dom, parent, before)
                    } else {
                        otherwise(dom, parent, before)
                    }
                })
            })
        });
    });
    BlockNodes::new(move || {
        let mut nodes = shown
            .borrow()
            .as_ref()
            .map(|(_, fragment)| fragment.nodes())
            .unwrap_or_default();
        nodes.push(anchor);
        nodes
    })
}

/// Removes the branch shown and builds the other, holding no borrow
/// of the state while either runs.
fn replace_branch<D: Dom + ?Sized>(
    dom: &Rc<RefCell<D>>,
    state: &Shown,
    now: bool,
    build: impl FnOnce(&Rc<RefCell<D>>) -> Fragment,
) {
    let previous = state.borrow_mut().take();
    if let Some((_, fragment)) = previous {
        fragment.remove(dom);
    }
    let fragment = build(dom);
    *state.borrow_mut() = Some((now, fragment));
}
