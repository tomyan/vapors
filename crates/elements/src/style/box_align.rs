//! The alignment keywords as Taffy 0.15's alignment values, where
//! `normal` is a value of its own.

use taffy::{AlignContent, AlignItems};
use vapors_css::value::Value;

use super::values::{keyword, unknown};

pub(super) fn align_items(value: &Value) -> Result<AlignItems, String> {
    Ok(match keyword(value)? {
        "stretch" => AlignItems::STRETCH,
        "center" => AlignItems::CENTER,
        "start" => AlignItems::START,
        "end" => AlignItems::END,
        "flex-start" => AlignItems::FLEX_START,
        "flex-end" => AlignItems::FLEX_END,
        "baseline" => AlignItems::BASELINE,
        "normal" => AlignItems::NORMAL,
        other => return Err(unknown(other)),
    })
}

pub(super) fn align_self(value: &Value) -> Result<Option<AlignItems>, String> {
    match keyword(value)? {
        "auto" => Ok(None),
        _ => align_items(value).map(Some),
    }
}

pub(super) fn align_content(value: &Value) -> Result<AlignContent, String> {
    Ok(match keyword(value)? {
        "stretch" => AlignContent::STRETCH,
        "center" => AlignContent::CENTER,
        "start" => AlignContent::START,
        "end" => AlignContent::END,
        "flex-start" => AlignContent::FLEX_START,
        "flex-end" => AlignContent::FLEX_END,
        "space-between" => AlignContent::SPACE_BETWEEN,
        "space-around" => AlignContent::SPACE_AROUND,
        "space-evenly" => AlignContent::SPACE_EVENLY,
        "normal" => AlignContent::NORMAL,
        other => return Err(unknown(other)),
    })
}
