//! `transition` and `animation` shorthands: comma-separated items whose
//! tokens are classified by shape, as sumi did.

use super::PropertyId;
use super::declaration::{Longhand, Specified};
use super::shorthand::parts;
use crate::ast::ComponentValue;
use crate::parse::component::split_commas;
use crate::syntax::{match_value, parse_grammar};
use crate::value::Value;
use crate::{Error, Position, Result};

fn grammar(text: &str) -> crate::syntax::Grammar {
    parse_grammar(text)
}

/// Per item: `[<property>] || <duration> || <easing> || <delay>`.
pub(super) fn transition(
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    let time = grammar("<time [0,inf]>");
    let easing = grammar("<easing-function>");
    let property = grammar("none | all | <custom-ident>");
    let mut columns: Vec<Vec<Value>> = vec![Vec::new(); 4];
    for item in split_commas(values) {
        let mut slots: [Option<Value>; 4] = [None, None, None, None];
        for part in parts(&item) {
            let slot = if let Some(t) = match_value(&time, &part) {
                if slots[1].is_none() {
                    (1, t)
                } else if slots[3].is_none() {
                    (3, t)
                } else {
                    return Err(Error::new(
                        at,
                        "transition: a third time; an item has one duration and one delay",
                    ));
                }
            } else if let Some(e) = match_value(&easing, &part).filter(|_| slots[2].is_none()) {
                (2, e)
            } else if let Some(p) = match_value(&property, &part).filter(|_| slots[0].is_none()) {
                (0, p)
            } else {
                return Err(Error::new(
                    at,
                    format!(
                        "transition: {:?} is not a property, time or easing function",
                        crate::serialize::component_values(&part)
                    ),
                ));
            };
            slots[slot.0] = Some(slot.1);
        }
        let is_none = slots[0].as_ref().and_then(Value::as_keyword) == Some("none");
        if slots[1].is_none() && !is_none {
            return Err(Error::new(
                at,
                "transition needs a duration (sumi dropped such an item; vapors refuses it)",
            ));
        }
        let defaults = [
            Value::Keyword("all".to_owned()),
            zero_s(),
            Value::Keyword("ease".to_owned()),
            zero_s(),
        ];
        for (column, (slot, default)) in columns.iter_mut().zip(slots.into_iter().zip(defaults)) {
            column.push(slot.unwrap_or(default));
        }
    }
    Ok(ids
        .iter()
        .zip(columns)
        .map(|(id, column)| Longhand {
            id: *id,
            value: Specified::Value(Value::CommaList(column)),
        })
        .collect())
}

fn zero_s() -> Value {
    Value::Time(crate::value::Time {
        value: 0.0,
        unit: crate::value::TimeUnit::S,
    })
}

/// Per item: `<name> || <duration> || <easing> || <delay> || <count> ||
/// <direction> || <fill-mode> || <play-state>`.
pub(super) fn animation(
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    let grammars = [
        grammar("none | <custom-ident>"),
        grammar("<time [0,inf]>"),
        grammar("<easing-function>"),
        grammar("<time>"),
        grammar("infinite | <number [0,inf]>"),
        grammar("normal | reverse | alternate | alternate-reverse"),
        grammar("none | forwards | backwards | both"),
        grammar("running | paused"),
    ];
    let defaults = ["none", "0s", "ease", "0s", "1", "normal", "none", "running"];
    let mut columns: Vec<Vec<Value>> = vec![Vec::new(); 8];
    for item in split_commas(values) {
        let mut slots: Vec<Option<Value>> = vec![None; 8];
        for part in parts(&item) {
            // Keywords that are also valid names are tried as their own
            // slot first: order of the grammars below puts the name last.
            let order = [1, 2, 3, 4, 5, 6, 7, 0];
            let found = order
                .into_iter()
                .find(|&i| slots[i].is_none() && match_value(&grammars[i], &part).is_some());
            match found {
                Some(i) => slots[i] = match_value(&grammars[i], &part),
                None => {
                    return Err(Error::new(
                        at,
                        format!(
                            "animation: {:?} does not fit any part of the shorthand",
                            crate::serialize::component_values(&part)
                        ),
                    ));
                }
            }
        }
        if slots[1].is_none() {
            return Err(Error::new(
                at,
                "animation needs a duration (sumi dropped such an item; vapors refuses it)",
            ));
        }
        for (i, column) in columns.iter_mut().enumerate() {
            let default =
                match_value(&grammars[i], &text_values(defaults[i])).expect("defaults parse");
            column.push(slots[i].take().unwrap_or(default));
        }
    }
    Ok(ids
        .iter()
        .zip(columns)
        .map(|(id, column)| Longhand {
            id: *id,
            value: Specified::Value(Value::CommaList(column)),
        })
        .collect())
}

fn text_values(text: &str) -> Vec<ComponentValue> {
    crate::parse::component::values_of_text(text).expect("literal")
}
