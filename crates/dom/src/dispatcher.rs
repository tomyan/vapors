//! A component's own events: `Dispatcher` dispatches an event at a node
//! and up through its ancestors at once, as the DOM's `dispatchEvent`
//! does, and answers whether no listener cancelled it, so the component
//! runs its default action only then. Listeners run with the tree not
//! borrowed and are `Fn`, so a listener may use the tree and dispatch
//! again, the inner dispatch running to completion inside it, as in the
//! DOM. A dispatch that cannot run is refused, never dropped.
//!
//! Single-thread: listeners are `Rc` and the tree an `Rc<RefCell<_>>`,
//! as the reactive graph is (ARCHITECTURE invariant 6); a tree that
//! dispatches across threads, with `Arc` and `Send` listeners, retires
//! this.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::{Dom, Event, Listener, NodeId};

/// The listeners an event at a node reaches: the node's, then each
/// ancestor's, with their node, cloned so none runs with the tree
/// borrowed.
pub trait Listeners {
    /// The listeners for `name` along `target`'s chain, target first;
    /// `None` when `target` is not in the tree (removed).
    fn listeners_along(&self, target: NodeId, name: &str) -> Option<Vec<(NodeId, Vec<Listener>)>>;
}

/// Runs `event` through the listeners `along` (taken for the whole
/// path before any runs), node by node, until one stops propagation;
/// `after` sees each listener's node and the event after it runs. The
/// one bubbling loop: the elements document, the recording Dom and
/// `Dispatcher` all deliver through it. Returns how many listeners ran.
pub fn run_along(
    along: Vec<(NodeId, Vec<Listener>)>,
    event: &mut Event,
    mut after: impl FnMut(NodeId, &mut Event),
) -> usize {
    let mut ran = 0;
    for (node, listeners) in along {
        for listener in listeners {
            listener(event);
            ran += 1;
            after(node, event);
        }
        if event.propagation_stopped() {
            break;
        }
    }
    ran
}

/// Dispatches a component's events into the tree it was mounted in.
#[derive(Clone)]
pub struct Dispatcher {
    dom: Rc<RefCell<dyn Dom>>,
    events: Rc<[String]>,
}

/// Why a dispatch was refused, and, when its listeners ran, whether one
/// cancelled it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRefused {
    why: String,
    cancelled: Option<bool>,
}

impl DispatchRefused {
    /// Whether a listener prevented the default, when the listeners ran
    /// before the refusal; `None` when none ran.
    #[must_use]
    pub fn cancelled(&self) -> Option<bool> {
        self.cancelled
    }
}

impl fmt::Display for DispatchRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.why)
    }
}

impl std::error::Error for DispatchRefused {}

impl Dispatcher {
    /// A dispatcher into `dom` of the events named in `events`: the ones
    /// the component's type declares. It declares nothing to the tree;
    /// the generated `events()` of each component type is declared when
    /// that component mounts, its children's included.
    #[must_use]
    pub fn new(dom: Rc<RefCell<dyn Dom>>, events: &[&str]) -> Self {
        let events = events.iter().map(|name| (*name).to_owned()).collect();
        Self { dom, events }
    }

    /// Dispatches `event` at `node`, bubbling: `Ok(true)` unless a
    /// listener prevented its default, as `dispatchEvent` answers.
    ///
    /// # Errors
    /// An event this dispatcher does not declare, a node not in the tree,
    /// or the tree borrowed (a dispatch from inside a `Dom` call), before
    /// any listener runs; or a listener that asked for pointer capture,
    /// which only a pointer event carries, after every listener ran, with
    /// the cancel answer kept ([`DispatchRefused::cancelled`]).
    pub fn dispatch(&self, node: NodeId, event: Event) -> Result<bool, DispatchRefused> {
        let name = event.name().to_owned();
        let refused = |why: String| DispatchRefused {
            why: format!("`{name}` at node {node:?}: {why}"),
            cancelled: None,
        };
        if !self.events.contains(&name) {
            return Err(refused(format!(
                "this dispatcher declares only {:?}",
                self.events
            )));
        }
        let along = self
            .dom
            // a listener may borrow it mutably, so any borrow held now
            // would panic there; refuse here instead
            .try_borrow_mut()
            .map_err(|_| refused("the tree is borrowed: dispatch from a listener or after the call that borrows it, not inside it".into()))?
            .listeners_along(node, &name)
            .ok_or_else(|| refused("the node is not in the tree (removed)".into()))?;
        let mut event = event.at(node);
        let mut captured = false;
        let ran = run_along(along, &mut event, |_, event| {
            captured |= event.take_capture().is_some();
        });
        let went = !event.default_prevented();
        if captured {
            return Err(DispatchRefused {
                why: format!(
                    "`{name}` at node {node:?}: dispatched to {ran} listeners, then refused: a listener asked for pointer capture, which only a pointer event carries"
                ),
                cancelled: Some(!went),
            });
        }
        Ok(went)
    }

    /// [`Dispatcher::dispatch`] for a component whose dispatches cannot
    /// be refused but by a bug in it: `true` unless a listener cancelled
    /// the event.
    ///
    /// # Panics
    /// When the dispatch is refused, naming why: a bug in the component.
    #[must_use]
    pub fn emit(&self, node: NodeId, event: Event) -> bool {
        match self.dispatch(node, event) {
            Ok(went) => went,
            Err(why) => panic!("{why}"),
        }
    }
}
