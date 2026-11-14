//! Specified to computed values: inheritance, initial values, `var()`
//! substitution, and resolution of relative units and colours. Each
//! longhand's value comes from one `Source`, copied once: a winner's or
//! the initial value resolved, or the parent's computed value as it is
//! (docs/plans/style-allocations.md, O2).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use vapors_scene::Color as Rgba;

use super::collect::Winners;
use super::{ComputedStyle, Environment};
use crate::ast::ComponentValue;
use crate::property::{PROPERTIES, PropertyId};
use crate::syntax::match_value;
use crate::value::{Color, Context, Length, Value, substitute_vars};

use super::compute_resolve::resolve;
use super::compute_source::{source, value_of};
use crate::{Error, Position, Result};

/// The initial value of every longhand, parsed once: the one table the
/// build-time analysis judges `initial`, `unset` and undeclared by.
pub(crate) fn initial(id: PropertyId) -> &'static Value {
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
    winners: &Winners<'_>,
    parent: Option<&ComputedStyle>,
    env: &Environment,
) -> Result<ComputedStyle> {
    let custom = custom_properties(winners, parent)?;
    let lookup = |name: &str| custom.get(name).map(Vec::as_slice);
    let (font_size_id, color_id) = (named("font-size"), named("color"));
    let font_size = font_size(font_size_id, winners, parent, env, &lookup)?;
    let root_font_size = parent.map_or(font_size, |p| p.root_font_size);
    let ctx = context(font_size, root_font_size, None, env);
    let color = color(color_id, winners, parent, &ctx, env, &lookup)?;
    let mut values = Vec::with_capacity(PROPERTIES.len());
    for id in PropertyId::all() {
        // the two read above: their sources are read once
        if id == font_size_id {
            values.push(Value::Length(Length::px(font_size)));
            continue;
        }
        if id == color_id {
            values.push(Value::Color(Color::Rgba(color)));
            continue;
        }
        let percent_base = (id.name() == "line-height").then_some(font_size);
        let ctx = Context {
            percent_base,
            ..ctx
        };
        let source = source(id, winners, parent, &lookup)?;
        let value = value_of(source, id, &ctx, env, color).map_err(|e| at(&e, winners, id))?;
        values.push(value);
    }
    super::font_weight::resolve_relative(&mut values, parent);
    let positions = winners
        .longhands
        .iter()
        .map(|w| w.as_ref().map(|c| (c.sheet, c.position)))
        .collect();
    Ok(ComputedStyle {
        values,
        custom,
        font_size,
        root_font_size,
        color,
        positions,
    })
}

/// A longhand the table is known to hold.
fn named(name: &str) -> PropertyId {
    PropertyId::from_name(name).expect("table")
}

/// The computed font size: em and % against the parent's; rem on the
/// root refers to the initial font size (Values Level 4 section 6.1.2).
fn font_size<'a>(
    id: PropertyId,
    winners: &Winners<'_>,
    parent: Option<&ComputedStyle>,
    env: &Environment,
    lookup: &dyn Fn(&str) -> Option<&'a [ComponentValue]>,
) -> Result<f64> {
    let parent_font_size = parent.map_or(env.root_font_size, |p| p.font_size);
    let rem = parent.map_or(env.root_font_size, |p| p.root_font_size);
    let ctx = context(parent_font_size, rem, Some(parent_font_size), env);
    let source = source(id, winners, parent, lookup)?;
    match value_of(source, id, &ctx, env, Rgba::from_rgba8(0, 0, 0, 255))? {
        Value::Length(l) => Ok(l.value),
        other => Err(Error::new(
            Position::default(),
            format!("font-size computed to {}", other.to_css()),
        )),
    }
}

/// The computed colour; `currentcolor` here means the parent's.
fn color<'a>(
    id: PropertyId,
    winners: &Winners<'_>,
    parent: Option<&ComputedStyle>,
    ctx: &Context,
    env: &Environment,
    lookup: &dyn Fn(&str) -> Option<&'a [ComponentValue]>,
) -> Result<Rgba> {
    let parent_color = parent.map_or(Rgba::from_rgba8(0, 0, 0, 255), |p| p.color);
    let source = source(id, winners, parent, lookup)?;
    match value_of(source, id, ctx, env, parent_color)? {
        Value::Color(Color::Rgba(c)) => Ok(c),
        other => Err(Error::new(
            Position::default(),
            format!("color computed to {}", other.to_css()),
        )),
    }
}

fn at(error: &Error, winners: &Winners<'_>, id: PropertyId) -> Error {
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
    winners: &Winners<'_>,
    parent: Option<&ComputedStyle>,
) -> Result<BTreeMap<String, Vec<ComponentValue>>> {
    let mut raw: BTreeMap<String, Vec<ComponentValue>> =
        parent.map(|p| p.custom.clone()).unwrap_or_default();
    for (name, candidate) in &winners.custom {
        raw.insert((*name).to_owned(), candidate.value.to_vec());
    }
    let mut out = raw.clone();
    for (name, candidate) in &winners.custom {
        let lookup = |n: &str| raw.get(n).map(Vec::as_slice);
        let value = substitute_vars(candidate.value, &lookup)
            .map_err(|e| Error::new(candidate.position, format!("{name}: {e}")))?;
        out.insert((*name).to_owned(), value);
    }
    Ok(out)
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
