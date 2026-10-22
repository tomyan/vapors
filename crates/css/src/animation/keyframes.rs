//! `@keyframes` stops resolved against one element: `var()` from its
//! custom-property scope, units against its font size, colours under the
//! environment's scheme. Missing 0% and 100% values come from the
//! element's own computed style.

use super::sampler::Animation;
use super::spec::AnimationSpec;
use super::timing::TimingFunction;
use crate::cascade::{Cascade, ComputedStyle, Environment};
use crate::property::{PropertyId, Specified};
use crate::value::Value;
use crate::{Error, Result};

/// One resolved stop.
#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    /// 0.0 to 1.0.
    pub offset: f64,
    /// The computed values this stop sets.
    pub values: Vec<(PropertyId, Value)>,
    /// `animation-timing-function` declared in this stop, for the
    /// segment that starts here.
    pub timing: Option<TimingFunction>,
}

/// Stops sorted by offset, every animated property present at 0 and 1.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedKeyframes {
    /// The stops.
    pub stops: Vec<Stop>,
}

impl Cascade {
    /// The animation `spec` names, with its keyframes resolved for the
    /// element whose computed style is `style`.
    ///
    /// # Errors
    /// No `@keyframes` of that name; a `var()` in a stop that does not
    /// resolve in the element's scope; a value that fails its grammar.
    pub fn animation(
        &self,
        spec: &AnimationSpec,
        style: &ComputedStyle,
        env: &Environment,
    ) -> Result<Animation> {
        let keyframes = self.keyframes(&spec.name).ok_or_else(|| {
            Error::new(
                PropertyId::from_name("animation-name")
                    .and_then(|id| style.declared_at(id))
                    .unwrap_or_default(),
                format!(
                    "animation {:?}: no @keyframes block of that name",
                    spec.name
                ),
            )
        })?;
        let mut stops = Vec::new();
        for stop in &keyframes.stops {
            stops.push(resolve_stop(stop, style, env)?);
        }
        stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        fill_ends(&mut stops, style);
        Ok(Animation::new(spec.clone(), ResolvedKeyframes { stops }))
    }
}

fn resolve_stop(
    stop: &crate::ast::KeyframeStop,
    style: &ComputedStyle,
    env: &Environment,
) -> Result<Stop> {
    let timing_id = PropertyId::from_name("animation-timing-function").expect("table");
    let mut values = Vec::new();
    let mut timing = None;
    for decl in &stop.declarations {
        for longhand in &decl.longhands {
            let specified = stop_value(decl, longhand, style)?;
            if longhand.id == timing_id {
                timing = TimingFunction::from_value(&specified);
                continue;
            }
            let computed = crate::cascade::resolve_for(&specified, longhand.id, style, env)
                .map_err(|e| {
                    Error::new(
                        decl.position,
                        format!("{}: {}", longhand.id.name(), e.message),
                    )
                })?;
            values.push((longhand.id, computed));
        }
    }
    Ok(Stop {
        offset: stop.offset,
        values,
        timing,
    })
}

/// One longhand's specified value in a stop; `var()` resolves in the
/// element's scope and CSS-wide keywords are refused.
fn stop_value(
    decl: &crate::ast::Declaration,
    longhand: &crate::property::Longhand,
    style: &ComputedStyle,
) -> Result<Value> {
    let wide = || {
        Error::new(
            decl.position,
            format!(
                "{}: CSS-wide keywords are not allowed in @keyframes",
                decl.name
            ),
        )
    };
    let resolved = match &longhand.value {
        Specified::Pending { property, tokens } => {
            let lookup = |name: &str| style.custom(name);
            crate::property::resolve_pending(property, tokens, &lookup, longhand.id, decl.position)?
        }
        other => other.clone(),
    };
    match resolved {
        Specified::Value(v) => Ok(v),
        _ => Err(wide()),
    }
}

/// Give every animated property a value at 0 and at 1 from `style`.
fn fill_ends(stops: &mut Vec<Stop>, style: &ComputedStyle) {
    let mut ids: Vec<PropertyId> = stops
        .iter()
        .flat_map(|s| s.values.iter().map(|(id, _)| *id))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    for offset in [0.0, 1.0] {
        let index = if let Some(i) = stops
            .iter()
            .position(|s| (s.offset - offset).abs() < f64::EPSILON)
        {
            i
        } else {
            let at = if offset == 0.0 { 0 } else { stops.len() };
            stops.insert(
                at,
                Stop {
                    offset,
                    values: Vec::new(),
                    timing: None,
                },
            );
            at
        };
        for id in &ids {
            if !stops[index].values.iter().any(|(i, _)| i == id) {
                stops[index].values.push((*id, style.get(*id).clone()));
            }
        }
    }
}
