//! Specified to computed values: inheritance, initial values, `var()`
//! substitution, and resolution of relative units and colours.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use vapors_scene::Color as Rgba;

use super::collect::Winners;
use super::{ComputedStyle, Environment};
use crate::ast::ComponentValue;
use crate::property::{PROPERTIES, PropertyId, Specified, resolve_pending};
use crate::syntax::match_value;
use crate::value::{Color, Context, Length, Value, substitute_vars};

use super::compute_resolve::resolve;
use crate::{Error, Position, Result};

/// The initial value of every longhand, parsed once.
fn initial(id: PropertyId) -> &'static Value {
    static INITIALS: OnceLock<Vec<Value>> = OnceLock::new();
    &INITIALS.get_or_init(|| {
        PROPERTIES
            .iter()
            .map(|d| {
                let values = crate::parse::component::values_of_text(d.initial)
                    .expect("initial values tokenize");
                let id = PropertyId::from_name(d.name).expect("table entry");
                match_value(id.grammar(), &values)
                    .expect("initial values parse (property_table tests)")
            })
            .collect()
    })[id.index()]
}

/// Build the computed style from the winning declarations.
pub(crate) fn style(
    winners: &Winners,
    parent: Option<&ComputedStyle>,
    env: &Environment,
) -> Result<ComputedStyle> {
    let custom = custom_properties(winners, parent)?;
    let lookup = |name: &str| custom.get(name).map(Vec::as_slice);
    let mut specified = Vec::with_capacity(PROPERTIES.len());
    for id in PropertyId::all() {
        specified.push(specified_value(id, winners, parent, &lookup)?);
    }
    let font_size = font_size(&specified, parent, env)?;
    let root_font_size = parent.map_or(font_size, |p| p.root_font_size);
    let ctx = context(font_size, root_font_size, None, env);
    let color = color(&specified, parent, &ctx, env)?;
    let mut values = Vec::with_capacity(PROPERTIES.len());
    for (id, value) in PropertyId::all().zip(&specified) {
        let percent_base = (id.name() == "line-height").then_some(font_size);
        let ctx = Context {
            percent_base,
            ..ctx
        };
        values
            .push(resolve(value, &ctx, env.color_scheme, color).map_err(|e| at(&e, winners, id))?);
    }
    let font_size_id = PropertyId::from_name("font-size").expect("table");
    values[font_size_id.index()] = Value::Length(Length::px(font_size));
    Ok(ComputedStyle {
        values,
        custom,
        font_size,
        root_font_size,
        color,
    })
}

/// The computed font size: em and % against the parent's; rem on the
/// root refers to the initial font size (Values Level 4 section 6.1.2).
fn font_size(
    specified: &[Value],
    parent: Option<&ComputedStyle>,
    env: &Environment,
) -> Result<f64> {
    let parent_font_size = parent.map_or(env.root_font_size, |p| p.font_size);
    let rem = parent.map_or(env.root_font_size, |p| p.root_font_size);
    let ctx = context(parent_font_size, rem, Some(parent_font_size), env);
    let id = PropertyId::from_name("font-size").expect("table");
    match resolve(
        &specified[id.index()],
        &ctx,
        env.color_scheme,
        Rgba::from_rgba8(0, 0, 0, 255),
    )? {
        Value::Length(l) => Ok(l.value),
        other => Err(Error::new(
            Position::default(),
            format!("font-size computed to {}", other.to_css()),
        )),
    }
}

/// The computed colour; `currentcolor` here means the parent's.
fn color(
    specified: &[Value],
    parent: Option<&ComputedStyle>,
    ctx: &Context,
    env: &Environment,
) -> Result<Rgba> {
    let parent_color = parent.map_or(Rgba::from_rgba8(0, 0, 0, 255), |p| p.color);
    let id = PropertyId::from_name("color").expect("table");
    match resolve(&specified[id.index()], ctx, env.color_scheme, parent_color)? {
        Value::Color(Color::Rgba(c)) => Ok(c),
        other => Err(Error::new(
            Position::default(),
            format!("color computed to {}", other.to_css()),
        )),
    }
}

fn at(error: &Error, winners: &Winners, id: PropertyId) -> Error {
    let position = winners.longhands[id.index()]
        .as_ref()
        .map_or(error.position, |c| c.position);
    Error::new(position, format!("{}: {}", id.name(), error.message))
}

fn context(
    font_size: f64,
    root_font_size: f64,
    percent_base: Option<f64>,
    env: &Environment,
) -> Context {
    Context {
        font_size,
        root_font_size,
        viewport_width: env.viewport_width,
        viewport_height: env.viewport_height,
        zero_advance: font_size * env.zero_advance_em,
        percent_base,
    }
}

/// The custom properties in scope: inherited ones overlaid by this
/// element's winners, with `var()` substituted in the latter.
fn custom_properties(
    winners: &Winners,
    parent: Option<&ComputedStyle>,
) -> Result<BTreeMap<String, Vec<ComponentValue>>> {
    let mut raw: BTreeMap<String, Vec<ComponentValue>> =
        parent.map(|p| p.custom.clone()).unwrap_or_default();
    for (name, candidate) in &winners.custom {
        raw.insert(name.clone(), candidate.value.clone());
    }
    let mut out = raw.clone();
    for (name, candidate) in &winners.custom {
        let lookup = |n: &str| raw.get(n).map(Vec::as_slice);
        let value = substitute_vars(&candidate.value, &lookup)
            .map_err(|e| Error::new(candidate.position, format!("{name}: {e}")))?;
        out.insert(name.clone(), value);
    }
    Ok(out)
}

/// The specified value of one longhand: the winner, substituted and
/// parsed, or what inheritance and the initial value give.
fn specified_value<'a>(
    id: PropertyId,
    winners: &Winners,
    parent: Option<&ComputedStyle>,
    lookup: &dyn Fn(&str) -> Option<&'a [ComponentValue]>,
) -> Result<Value> {
    let inherited = || parent.map_or_else(|| initial(id).clone(), |p| p.values[id.index()].clone());
    let default = || {
        if id.inherited() {
            inherited()
        } else {
            initial(id).clone()
        }
    };
    let Some(candidate) = &winners.longhands[id.index()] else {
        return Ok(default());
    };
    Ok(match &candidate.value {
        Specified::Value(v) => v.clone(),
        Specified::Inherit => inherited(),
        Specified::Initial => initial(id).clone(),
        Specified::Unset => default(),
        Specified::Pending { property, tokens } => {
            match resolve_pending(property, tokens, lookup, id, candidate.position)? {
                Specified::Value(v) => v,
                Specified::Inherit => inherited(),
                Specified::Initial => initial(id).clone(),
                Specified::Unset => default(),
                Specified::Pending { .. } => unreachable!("substitution removes every var()"),
            }
        }
    })
}

/// Resolve a specified value of `id` as if declared on the element whose
/// computed style is `style`: the `@keyframes` path.
pub(crate) fn resolve_for(
    value: &Value,
    id: PropertyId,
    style: &ComputedStyle,
    env: &Environment,
) -> Result<Value> {
    let percent_base = (id.name() == "line-height").then_some(style.font_size);
    let ctx = context(style.font_size, style.root_font_size, percent_base, env);
    resolve(value, &ctx, env.color_scheme, style.color)
}
