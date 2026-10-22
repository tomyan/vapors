//! `transition` and `animation` shorthands: comma-separated items whose
//! tokens are classified by shape, as sumi did.

use super::PropertyId;
use super::declaration::{Longhand, Specified};
use super::shorthand::parts;
use crate::ast::ComponentValue;
use crate::parse::component::split_commas;
use crate::syntax::{Grammar, match_value, parse_grammar};
use crate::value::Value;
use crate::{Error, Position, Result};

/// The grammars of the slots of one shorthand item, in longhand order.
struct Slots {
    grammars: Vec<Grammar>,
    defaults: Vec<&'static str>,
    /// The order the grammars are tried in: a keyword valid for two slots
    /// goes to the one tried first.
    try_order: Vec<usize>,
}

impl Slots {
    fn transition() -> Self {
        Self {
            grammars: [
                "none | all | <custom-ident>",
                "<time [0,inf]>",
                "<easing-function>",
                "<time>",
            ]
            .map(parse_grammar)
            .to_vec(),
            defaults: vec!["all", "0s", "ease", "0s"],
            try_order: vec![1, 3, 2, 0],
        }
    }

    fn animation() -> Self {
        Self {
            grammars: [
                "none | <custom-ident>",
                "<time [0,inf]>",
                "<easing-function>",
                "<time>",
                "infinite | <number [0,inf]>",
                "normal | reverse | alternate | alternate-reverse",
                "none | forwards | backwards | both",
                "running | paused",
            ]
            .map(parse_grammar)
            .to_vec(),
            defaults: vec!["none", "0s", "ease", "0s", "1", "normal", "none", "running"],
            try_order: vec![1, 3, 2, 4, 5, 6, 7, 0],
        }
    }

    /// Classify one item's parts into slots; `None` for an absent slot.
    fn classify(
        &self,
        item: &[ComponentValue],
        what: &str,
        at: Position,
    ) -> Result<Vec<Option<Value>>> {
        let mut slots: Vec<Option<Value>> = vec![None; self.grammars.len()];
        for part in parts(item) {
            let found =
                self.try_order.iter().copied().find(|&i| {
                    slots[i].is_none() && match_value(&self.grammars[i], &part).is_some()
                });
            let Some(i) = found else {
                let text = crate::serialize::component_values(&part);
                return Err(Error::new(
                    at,
                    format!(
                        "{what}: {text:?} fits no free slot of the shorthand; an item has one name or property, one duration time, one easing function and one delay time{}",
                        if what == "animation" {
                            ", plus count, direction, fill mode and play state"
                        } else {
                            ""
                        }
                    ),
                ));
            };
            slots[i] = match_value(&self.grammars[i], &part);
        }
        Ok(slots)
    }

    fn default(&self, i: usize) -> Value {
        let values = crate::parse::component::values_of_text(self.defaults[i]).expect("literal");
        match_value(&self.grammars[i], &values).expect("defaults parse")
    }
}

/// Expand every item into one column per longhand.
fn expand(
    slots: &Slots,
    ids: &[PropertyId],
    values: &[ComponentValue],
    what: &str,
    at: Position,
) -> Result<Vec<Longhand>> {
    let mut columns: Vec<Vec<Value>> = vec![Vec::new(); slots.grammars.len()];
    for item in split_commas(values) {
        let mut found = slots.classify(&item, what, at)?;
        check_duration(&found, what, at)?;
        for (i, column) in columns.iter_mut().enumerate() {
            column.push(found[i].take().unwrap_or_else(|| slots.default(i)));
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

/// A named item needs a duration; `none` leaves the name at its default
/// and needs nothing.
fn check_duration(slots: &[Option<Value>], what: &str, at: Position) -> Result<()> {
    let named = slots[0]
        .as_ref()
        .and_then(Value::as_keyword)
        .is_some_and(|n| n != "none");
    if slots[1].is_none() && named {
        return Err(Error::new(
            at,
            format!("{what} needs a duration (sumi dropped such an item; vapors refuses it)"),
        ));
    }
    Ok(())
}

/// Per item: `[<property>] || <duration> || <easing> || <delay>`.
pub(super) fn transition(
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    let longhands = expand(&Slots::transition(), ids, values, "transition", at)?;
    if let Specified::Value(properties) = &longhands[0].value {
        super::declaration::check_transition_properties(properties, at)?;
    }
    Ok(longhands)
}

/// Per item: `<name> || <duration> || <easing> || <delay> || <count> ||
/// <direction> || <fill-mode> || <play-state>`.
pub(super) fn animation(
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    expand(&Slots::animation(), ids, values, "animation", at)
}
