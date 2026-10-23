//! `border: <width> || <style> || <color>` for all four sides, and
//! `border-top` (and each other side) for one.

use super::PropertyId;
use super::declaration::{Longhand, Specified, parse_longhand};
use super::declaration_check::refusal;
use super::shorthand::parts;
use crate::ast::ComponentValue;
use crate::value::Value;
use crate::{Position, Result};

/// Expand `border` (twelve longhands: four widths, four styles, four
/// colours) or one side's shorthand (its width, style and colour).
/// Omitted parts take their initial values.
pub(super) fn expand(
    name: &str,
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    let per_group = ids.len() / 3;
    let group_ids = [ids[0], ids[per_group], ids[2 * per_group]];
    let [width_id, style_id, color_id] = group_ids;
    let mut found: [Option<Value>; 3] = [None, None, None];
    for part in parts(values) {
        let slot = group_ids
            .iter()
            .enumerate()
            .find(|(i, id)| found[*i].is_none() && parse_longhand(**id, &part, at).is_ok());
        let Some((i, id)) = slot else {
            return Err(refusal(
                name,
                "<line-width> || <line-style> || <color>",
                values,
                at,
            ));
        };
        found[i] = parse_longhand(*id, &part, at).ok();
    }
    let width = found[0].take().unwrap_or_else(|| initial(width_id, at));
    let style = found[1].take().unwrap_or_else(|| initial(style_id, at));
    let color = found[2].take().unwrap_or_else(|| initial(color_id, at));
    let by_group = [width, style, color];
    Ok(ids
        .iter()
        .enumerate()
        .map(|(i, id)| Longhand {
            id: *id,
            value: Specified::Value(by_group[i / per_group].clone()),
        })
        .collect())
}

/// A longhand's initial value, parsed.
fn initial(id: PropertyId, at: Position) -> Value {
    let values =
        crate::parse::component::values_of_text(id.def().initial).expect("initial values parse");
    parse_longhand(id, &values, at).expect("initial values parse")
}
