//! A snippet (1-R1): a piece of template, built where it is rendered.
//! Children inside a component tag are the child's `children` snippet,
//! and `{render children()}` builds them there. A snippet's nodes and
//! effects belong to the owner that renders it.
//!
//! A snippet builds through `dyn Dom`, not a type parameter: a
//! component's `mount` is generic over its Dom, but its props struct is
//! not, and a snippet is a prop. `Dom`'s methods take no type
//! parameters, so any `Rc<RefCell<D>>` becomes an `Rc<RefCell<dyn Dom>>`
//! at the render site ([`DynDom`]).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::{Dom, NodeId, Root};

/// A snippet's arguments as the render site reads them: called inside
/// the snippet's effects, so a snippet follows its arguments as they
/// change (Svelte's parameters are reactive the same way).
pub type Args<A> = Rc<dyn Fn() -> A>;

/// What a snippet builds with: the Dom, the parent, the node to place
/// before and its arguments; it returns its top-level entries.
type Build<A> = dyn Fn(&Rc<RefCell<dyn Dom>>, NodeId, Option<NodeId>, Args<A>) -> Vec<Root>;

/// A piece of template built where it is rendered, taking arguments of
/// type `A`: a tuple of its parameters, `()` for none.
pub struct Snippet<A = ()>(Rc<Build<A>>);

impl<A: 'static> Snippet<A> {
    /// A snippet that builds with `build`.
    pub fn new(
        build: impl Fn(&Rc<RefCell<dyn Dom>>, NodeId, Option<NodeId>, Args<A>) -> Vec<Root> + 'static,
    ) -> Snippet<A> {
        Snippet(Rc::new(build))
    }

    /// The snippet that builds nothing: a snippet prop's default.
    #[must_use]
    pub fn empty() -> Snippet<A> {
        Snippet::new(|_, _, _, _| Vec::new())
    }

    /// Builds the snippet in `parent`, before `before` (at the end when
    /// `None`), reading its arguments from `args`; its top-level entries.
    pub fn render(
        &self,
        dom: &impl DynDom,
        parent: NodeId,
        before: Option<NodeId>,
        args: impl Fn() -> A + 'static,
    ) -> Vec<Root> {
        self.render_args(dom, parent, before, Rc::new(args))
    }

    /// [`Snippet::render`] with the arguments already shared.
    pub fn render_args(
        &self,
        dom: &impl DynDom,
        parent: NodeId,
        before: Option<NodeId>,
        args: Args<A>,
    ) -> Vec<Root> {
        (self.0)(&dom.dyn_dom(), parent, before, args)
    }
}

/// Snippets are equal when they are the same snippet: clones of one
/// `Snippet::new`, not two built alike.
impl<A> PartialEq for Snippet<A> {
    fn eq(&self, other: &Snippet<A>) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<A> Clone for Snippet<A> {
    fn clone(&self) -> Snippet<A> {
        Snippet(Rc::clone(&self.0))
    }
}

impl<A: 'static> Default for Snippet<A> {
    fn default() -> Snippet<A> {
        Snippet::empty()
    }
}

impl<A> fmt::Debug for Snippet<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Snippet")
    }
}

/// What `{render expr()}` accepts as `expr`: a snippet, or none to
/// render nothing, owned or borrowed (`map.get(key)` gives an
/// `Option<&Snippet>`).
pub trait Renderable<A> {
    /// The snippet to render, if any.
    fn snippet(&self) -> Option<Snippet<A>>;
}

impl<A> Renderable<A> for Snippet<A> {
    fn snippet(&self) -> Option<Snippet<A>> {
        Some(self.clone())
    }
}

impl<A> Renderable<A> for Option<Snippet<A>> {
    fn snippet(&self) -> Option<Snippet<A>> {
        self.clone()
    }
}

impl<A> Renderable<A> for Option<&Snippet<A>> {
    fn snippet(&self) -> Option<Snippet<A>> {
        self.cloned()
    }
}

impl<A, T: Renderable<A> + ?Sized> Renderable<A> for &T {
    fn snippet(&self) -> Option<Snippet<A>> {
        (**self).snippet()
    }
}

/// A shared Dom as `dyn Dom`, whatever it is now: a concrete Dom at a
/// component's top level, or `dyn Dom` already inside a snippet.
pub trait DynDom {
    /// The same Dom, as `dyn Dom`.
    fn dyn_dom(&self) -> Rc<RefCell<dyn Dom>>;
}

impl<D: Dom + 'static> DynDom for Rc<RefCell<D>> {
    fn dyn_dom(&self) -> Rc<RefCell<dyn Dom>> {
        Rc::clone(self) as Rc<RefCell<dyn Dom>>
    }
}

impl DynDom for Rc<RefCell<dyn Dom>> {
    fn dyn_dom(&self) -> Rc<RefCell<dyn Dom>> {
        Rc::clone(self)
    }
}

#[cfg(test)]
#[path = "snippet_tests.rs"]
mod tests;
