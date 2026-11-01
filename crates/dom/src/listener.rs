//! A listener's types: as given to the tree, and as the tree keeps it.

use crate::Event;

/// A listener as given to [`Dom::listen`](crate::Dom::listen): `Fn`, not
/// `FnMut`, so a listener that dispatches an event reaching itself again
/// runs again inside itself, as in the DOM; its state lives in signals or
/// cells, which `Fn` reaches.
pub type Handler = Box<dyn Fn(&mut Event)>;

/// A listener as the tree stores it: shared, so dispatch clones it out
/// and calls it with no borrow held.
///
/// Single-thread: `Rc`, as the reactive graph and the tree are (the
/// threading rule, ARCHITECTURE invariant 6); a tree that dispatches
/// across threads would need `Arc` and `Send` listeners, which retires
/// this.
pub type Listener = std::rc::Rc<dyn Fn(&mut Event)>;
