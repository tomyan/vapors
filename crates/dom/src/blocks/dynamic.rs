//! `{render expr()}`: the snippet an expression yields, built before an
//! anchor and rebuilt only when the expression yields a different one.
//! Snippets compare by identity, so one created on every read rebuilds
//! every time; create snippets once.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Effect, Owner};

use super::Fragment;
use crate::snippet::{Args, DynDom, Snippet};
use crate::{BlockNodes, Dom, NodeId};

/// The snippet shown, and its fragment.
type Shown<A> = Rc<RefCell<Option<(Snippet<A>, Fragment)>>>;

/// Keeps the snippet `callee` yields built before `anchor`, reading its
/// arguments from `args`; nothing while it yields `None`. Returns the
/// block's current nodes, for whatever owns its position.
pub fn dynamic<D, A, C, G>(
    dom: &Rc<RefCell<D>>,
    parent: NodeId,
    anchor: NodeId,
    callee: C,
    args: G,
) -> BlockNodes
where
    D: Dom + ?Sized + 'static,
    Rc<RefCell<D>>: DynDom,
    A: 'static,
    C: Fn() -> Option<Snippet<A>> + 'static,
    G: Fn() -> A + 'static,
{
    let dom = Rc::clone(dom);
    let args: Args<A> = Rc::new(args);
    let block_owner = Owner::new();
    let shown: Shown<A> = Rc::new(RefCell::new(None));
    let state = Rc::clone(&shown);
    let mut started = false;
    Effect::new(move || {
        let now = callee();
        let same = state.borrow().as_ref().map(|(was, _)| was) == now.as_ref();
        if started && same {
            return;
        }
        started = true;
        replace(&dom, &state, now, |dom, snippet| {
            block_owner.with(|| {
                Fragment::build(dom, parent, Some(anchor), |dom, parent, before| {
                    snippet.render_args(dom, parent, before, Rc::clone(&args))
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

/// Removes the snippet shown and builds `now`, holding no borrow of the
/// state while either runs.
fn replace<D: Dom + ?Sized, A>(
    dom: &Rc<RefCell<D>>,
    state: &Shown<A>,
    now: Option<Snippet<A>>,
    build: impl FnOnce(&Rc<RefCell<D>>, &Snippet<A>) -> Fragment,
) {
    let previous = state.borrow_mut().take();
    if let Some((_, fragment)) = previous {
        fragment.remove(dom);
    }
    if let Some(snippet) = now {
        let fragment = build(dom, &snippet);
        *state.borrow_mut() = Some((snippet, fragment));
    }
}
