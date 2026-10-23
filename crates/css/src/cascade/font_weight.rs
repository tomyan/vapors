//! `bolder` and `lighter` as numbers, from the inherited weight (CSS
//! Fonts 4 section 2.2.1): the computed value of `font-weight` is a
//! number, so a child of a bolder element steps from that number.

use super::ComputedStyle;
use crate::property::PropertyId;
use crate::value::Value;

/// Replaces a computed `bolder` or `lighter` in `values` with the
/// number its parent's weight gives (400 without a parent).
pub(super) fn resolve_relative(values: &mut [Value], parent: Option<&ComputedStyle>) {
    let id = PropertyId::from_name("font-weight").expect("table");
    let relative = match &values[id.index()] {
        Value::Keyword(k) if k == "bolder" => bolder,
        Value::Keyword(k) if k == "lighter" => lighter,
        _ => return,
    };
    let inherited = parent.map_or(400.0, |p| weight(&p.values[id.index()]));
    values[id.index()] = Value::Number(relative(inherited));
}

/// A computed weight as a number: `normal` is 400 and `bold` 700.
fn weight(value: &Value) -> f64 {
    match value {
        Value::Number(n) => *n,
        Value::Integer(n) => i32::try_from(*n).map_or(400.0, f64::from),
        Value::Keyword(k) if k == "bold" => 700.0,
        _ => 400.0,
    }
}

fn bolder(inherited: f64) -> f64 {
    match inherited {
        w if w < 350.0 => 400.0,
        w if w < 550.0 => 700.0,
        w if w < 900.0 => 900.0,
        w => w,
    }
}

fn lighter(inherited: f64) -> f64 {
    match inherited {
        w if w < 100.0 => w,
        w if w < 550.0 => 100.0,
        w if w < 750.0 => 400.0,
        _ => 700.0,
    }
}
