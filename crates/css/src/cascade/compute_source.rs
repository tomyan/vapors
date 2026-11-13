//! Where one longhand's computed value comes from, and the value from
//! it: a declared or substituted value resolved, the initial value
//! resolved, or the parent's computed value copied as it is
//! (docs/plans/style-allocations.md, O2).

use std::borrow::Cow;

use vapors_scene::Color as Rgba;

use super::collect::Winners;
use super::compute::initial;
use super::compute_resolve::resolve;
use super::{ComputedStyle, Environment};
use crate::Result;
use crate::ast::ComponentValue;
use crate::property::{PropertyId, Specified, resolve_pending};
use crate::value::{Context, Value};

/// Where one longhand's computed value comes from.
pub(super) enum Source<'a> {
    /// A declared value, resolved in the element's own context.
    Declared(&'a Value),
    /// A `var()` winner substituted, resolved as a declared value.
    Substituted(Value),
    /// The initial value, resolved.
    Initial,
    /// The parent's computed value, copied as it is: computed values
    /// are absolute (lengths px, colours rgba, calcs with absolute
    /// lengths), which `copied` checks in debug builds.
    Parent,
}

/// The source of one longhand's computed value: its winner, substituted
/// when it holds `var()`, or what inheritance and the initial value give.
pub(super) fn source<'w, 'c>(
    id: PropertyId,
    winners: &Winners<'w>,
    parent: Option<&ComputedStyle>,
    lookup: &dyn Fn(&str) -> Option<&'c [ComponentValue]>,
) -> Result<Source<'w>> {
    let Some(candidate) = &winners.longhands[id.index()] else {
        return Ok(unset(id, parent));
    };
    let specified = match candidate.value {
        Specified::Pending { property, tokens } => Cow::Owned(resolve_pending(
            property,
            tokens,
            lookup,
            id,
            candidate.position,
        )?),
        other => Cow::Borrowed(other),
    };
    Ok(match specified {
        Cow::Borrowed(Specified::Value(v)) => Source::Declared(v),
        Cow::Owned(Specified::Value(v)) => Source::Substituted(v),
        Cow::Borrowed(Specified::Inherit) | Cow::Owned(Specified::Inherit) => inherited(parent),
        Cow::Borrowed(Specified::Initial) | Cow::Owned(Specified::Initial) => Source::Initial,
        Cow::Borrowed(Specified::Unset) | Cow::Owned(Specified::Unset) => unset(id, parent),
        Cow::Borrowed(Specified::Pending { .. }) | Cow::Owned(Specified::Pending { .. }) => {
            unreachable!("substitution removes every var()")
        }
    })
}

/// What `inherit` gives: the parent's computed value, or the initial
/// value on the root.
fn inherited(parent: Option<&ComputedStyle>) -> Source<'static> {
    match parent {
        Some(_) => Source::Parent,
        None => Source::Initial,
    }
}

/// What no declaration, or `unset`, gives: `inherit` for an inherited
/// property, `initial` for the rest.
fn unset(id: PropertyId, parent: Option<&ComputedStyle>) -> Source<'static> {
    if id.inherited() {
        inherited(parent)
    } else {
        Source::Initial
    }
}

/// The computed value from its source.
pub(super) fn value_of(
    source: Source<'_>,
    id: PropertyId,
    parent: Option<&ComputedStyle>,
    ctx: &Context,
    env: &Environment,
    current: Rgba,
) -> Result<Value> {
    match source {
        Source::Declared(v) => resolve(v, ctx, env.color_scheme, current),
        Source::Substituted(v) => resolve(&v, ctx, env.color_scheme, current),
        Source::Initial => resolve(initial(id), ctx, env.color_scheme, current),
        Source::Parent => {
            let parent = parent.expect("Source::Parent is only given with a parent");
            Ok(copied(id, parent, ctx, env, current))
        }
    }
}

/// The parent's computed value of `id`, as it is. Debug builds check
/// that it resolves to itself in this element's context, which is what
/// lets it be copied without resolving.
fn copied(
    id: PropertyId,
    parent: &ComputedStyle,
    ctx: &Context,
    env: &Environment,
    current: Rgba,
) -> Value {
    let value = parent.values[id.index()].clone();
    debug_assert!(
        resolve(&value, ctx, env.color_scheme, current).as_ref() == Ok(&value),
        "{}: the parent's computed value `{}` does not resolve to itself, so a child cannot copy it (docs/plans/style-allocations.md, O2: a computed value is absolute); reproduce with the failing test",
        id.name(),
        value.to_css()
    );
    value
}
