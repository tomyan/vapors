//! `border: <width> || <style> || <color>` for all four sides.

use super::PropertyId;
use super::declaration::{Longhand, Specified, parse_longhand, refusal};
use super::shorthand::parts;
use crate::ast::ComponentValue;
use crate::value::Value;
use crate::{Position, Result};

/// Expand `border`. Omitted parts take their initial values.
pub(super) fn expand(
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    let width_id = ids[0];
    let style_id = ids[4];
    let color_id = ids[8];
    let mut width = None;
    let mut style = None;
    let mut color = None;
    for part in parts(values) {
        if width.is_none()
            && let Ok(v) = parse_longhand(width_id, &part, at)
        {
            width = Some(v);
        } else if style.is_none()
            && let Ok(v) = parse_longhand(style_id, &part, at)
        {
            style = Some(v);
        } else if color.is_none()
            && let Ok(v) = parse_longhand(color_id, &part, at)
        {
            color = Some(v);
        } else {
            return Err(refusal(
                "border",
                "<line-width> || <line-style> || <color>",
                values,
                at,
            ));
        }
    }
    let initial = |id: PropertyId| -> Value {
        let text = id.def().initial;
        let values = crate::parse::component::values_of_text(text).expect("initial values parse");
        parse_longhand(id, &values, at).expect("initial values parse")
    };
    let width = width.unwrap_or_else(|| initial(width_id));
    let style = style.unwrap_or_else(|| initial(style_id));
    let color = color.unwrap_or_else(|| initial(color_id));
    let mut out = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let value = match i / 4 {
            0 => width.clone(),
            1 => style.clone(),
            _ => color.clone(),
        };
        out.push(Longhand {
            id: *id,
            value: Specified::Value(value),
        });
    }
    Ok(out)
}
