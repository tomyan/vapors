//! A component's own events: `Dispatcher` dispatches an event at a node
//! and up through its ancestors at once, as the DOM's `dispatchEvent`
//! does, and answers whether no listener cancelled it, so the component
//! runs its default action only then. Listeners run with the tree not
//! borrowed, so they may use it; a dispatch that cannot run (the tree is
//! borrowed, as inside a `Dom` call) is refused, never dropped.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::{Dom, Event, Handler, NodeId};

/// The listeners an event at a node reaches: the node's, then each
/// ancestor's, with their node, cloned so none runs with the tree
/// borrowed.
pub trait Listeners {
    /// The listeners for `name` along `target`'s chain, target first.
    fn listeners_along(
        &self,
        target: NodeId,
        name: &str,
    ) -> Vec<(NodeId, Vec<Rc<RefCell<Handler>>>)>;
}

/// Dispatches a component's events into the tree it was mounted in.
#[derive(Clone)]
pub struct Dispatcher {
    dom: Rc<RefCell<dyn Dom>>,
    events: Rc<[String]>,
}

/// Why a dispatch did not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRefused(String);

impl fmt::Display for DispatchRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}

impl std::error::Error for DispatchRefused {}

impl Dispatcher {
    /// A dispatcher into `dom` of the events named in `events`, which it
    /// declares to the tree ([`Dom::declare_events`]) so listeners for
    /// them are taken. Call it at mount, not inside a `Dom` call.
    #[must_use]
    pub fn new(dom: Rc<RefCell<dyn Dom>>, events: &[&str]) -> Self {
        dom.borrow_mut().declare_events(events);
        let events = events.iter().map(|name| (*name).to_owned()).collect();
        Self { dom, events }
    }

    /// Dispatches `event` at `node`, bubbling: `Ok(true)` unless a
    /// listener prevented its default, as `dispatchEvent` answers.
    ///
    /// # Errors
    /// An event this dispatcher did not declare, or the tree borrowed
    /// (a dispatch from inside a `Dom` call).
    pub fn dispatch(&self, node: NodeId, event: Event) -> Result<bool, DispatchRefused> {
        let name = event.name().to_owned();
        if !self.events.contains(&name) {
            return Err(refused(
                &name,
                &format!(
                    "this dispatcher declares only {:?}; name it in `Dispatcher::new`",
                    self.events
                ),
            ));
        }
        let along = self
            .dom
            // a listener may borrow it mutably, so any borrow held now
            // would panic there; refuse here instead
            .try_borrow_mut()
            .map_err(|_| refused(&name, "the tree is borrowed: dispatch from a listener or after the call that borrows it, not inside it"))?
            .listeners_along(node, &name);
        let mut event = event.at(node);
        for (_, listeners) in along {
            for listener in listeners {
                (listener.borrow_mut())(&mut event);
                if event.take_capture().is_some() {
                    return Err(refused(
                        &name,
                        "a listener asked for pointer capture, which only a pointer event carries",
                    ));
                }
            }
            if event.propagation_stopped() {
                break;
            }
        }
        Ok(!event.default_prevented())
    }
}

impl Dispatcher {
    /// [`Dispatcher::dispatch`] for a component whose dispatches cannot
    /// be refused but by a bug in it: `true` unless a listener cancelled
    /// the event.
    ///
    /// # Panics
    /// When the dispatch is refused (a name not declared, or a dispatch
    /// from inside a `Dom` call), naming why: a bug in the component.
    #[must_use]
    pub fn emit(&self, node: NodeId, event: Event) -> bool {
        match self.dispatch(node, event) {
            Ok(went) => went,
            Err(why) => panic!("{why}"),
        }
    }
}

fn refused(name: &str, why: &str) -> DispatchRefused {
    DispatchRefused(format!("`{name}` was not dispatched: {why}"))
}
