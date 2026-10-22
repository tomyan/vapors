//! `{if}`: one branch is live at a time, rebuilt only when the
//! condition changes value.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Effect, Owner};

use super::Fragment;
use crate::{Dom, NodeId};

/// Keeps the branch matching `condition` built before `anchor`. The
/// builders receive the dom, the parent and the node to insert before.
pub fn conditional<D, C, T, E>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    anchor: NodeId,
    condition: C,
    then: T,
    otherwise: E,
) where
    D: Dom + 'static,
    C: Fn() -> bool + 'static,
    T: Fn(&Rc<RefCell<D>>, NodeId, Option<NodeId>) -> Vec<NodeId> + 'static,
    E: Fn(&Rc<RefCell<D>>, NodeId, Option<NodeId>) -> Vec<NodeId> + 'static,
{
    let dom = Rc::clone(dom);
    let block_owner = Owner::new();
    let mut shown: Option<(bool, Fragment)> = None;
    Effect::new(move || {
        let now = condition();
        if shown.as_ref().is_some_and(|(was, _)| *was == now) {
            return;
        }
        if let Some((_, fragment)) = shown.take() {
            fragment.remove(&dom);
        }
        let fragment = block_owner.with(|| {
            Fragment::build(&dom, parent, Some(anchor), |dom, parent, before| {
                if now {
                    then(dom, parent, before)
                } else {
                    otherwise(dom, parent, before)
                }
            })
        });
        shown = Some((now, fragment));
    });
}
