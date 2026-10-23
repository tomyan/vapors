//! The grid properties as Taffy 0.15's grid style: template and auto
//! tracks, auto-flow, item placement, and the justify properties.
//! Track lists come from vapors-css with no line names (it refuses
//! them) and with `repeat()` of a fixed count only.

use taffy::style_helpers::{line, span};
use taffy::{
    GridAutoFlow, GridPlacement, GridTemplateComponent, GridTemplateRepetition, Line,
    MaxTrackSizingFunction, MinTrackSizingFunction, RepetitionCount, Style, TrackSizingFunction,
};
use vapors_css::value::{LengthUnit, Value};

use super::box_align::{align_items, align_self};
use super::values::{Reader, keyword, shape, unknown};

/// Sets the grid fields of `style` from the element `reader` reads.
pub(crate) fn grid(reader: &mut Reader<'_>, style: &mut Style) {
    style.grid_template_columns = reader.read("grid-template-columns", template, Vec::new());
    style.grid_template_rows = reader.read("grid-template-rows", template, Vec::new());
    style.grid_auto_columns = reader.read("grid-auto-columns", auto_tracks, Vec::new());
    style.grid_auto_rows = reader.read("grid-auto-rows", auto_tracks, Vec::new());
    style.grid_auto_flow = reader.read("grid-auto-flow", auto_flow, GridAutoFlow::Row);
    style.grid_column = Line {
        start: reader.read("grid-column-start", placement, GridPlacement::Auto),
        end: reader.read("grid-column-end", placement, GridPlacement::Auto),
    };
    style.grid_row = Line {
        start: reader.read("grid-row-start", placement, GridPlacement::Auto),
        end: reader.read("grid-row-end", placement, GridPlacement::Auto),
    };
    style.justify_items = reader.read("justify-items", justify_items, taffy::AlignItems::NORMAL);
    style.justify_self = reader.read("justify-self", justify_self, None);
}

fn template(value: &Value) -> Result<Vec<GridTemplateComponent<String>>, String> {
    match value {
        Value::Keyword(k) if k == "none" => Ok(Vec::new()),
        Value::List(items) => items.iter().map(component).collect(),
        _ => Err(shape("a track list")),
    }
}

fn component(value: &Value) -> Result<GridTemplateComponent<String>, String> {
    match value {
        Value::Function(name, args) if name == "repeat" => {
            let [Value::Integer(count), Value::List(tracks)] = args.as_slice() else {
                return Err(shape("repeat(<count>, <tracks>)"));
            };
            let count = u16::try_from(*count)
                .map_err(|_| format!("repeats {count} times, more than a grid holds"))?;
            let tracks = tracks.iter().map(track).collect::<Result<Vec<_>, _>>()?;
            Ok(GridTemplateComponent::Repeat(GridTemplateRepetition {
                count: RepetitionCount::Count(count),
                tracks,
                line_names: Vec::new(),
            }))
        }
        other => track(other).map(GridTemplateComponent::Single),
    }
}

fn auto_tracks(value: &Value) -> Result<Vec<TrackSizingFunction>, String> {
    match value {
        Value::List(items) => items.iter().map(track).collect(),
        single => Ok(vec![track(single)?]),
    }
}

/// One track size: a single value stands for both bounds, except a
/// flex size, which is `minmax(auto, <fr>)` as CSS Grid defines it.
fn track(value: &Value) -> Result<TrackSizingFunction, String> {
    if let Value::Function(name, args) = value
        && name == "minmax"
    {
        let [low, high] = args.as_slice() else {
            return Err(shape("minmax(<min>, <max>)"));
        };
        return Ok(TrackSizingFunction {
            min: track_min(low)?,
            max: track_max(high)?,
        });
    }
    let min = match value {
        Value::Flex(_) => MinTrackSizingFunction::auto(),
        other => track_min(other)?,
    };
    Ok(TrackSizingFunction {
        min,
        max: track_max(value)?,
    })
}

#[allow(clippy::cast_possible_truncation)] // CSS lengths are far inside f32
fn track_min(value: &Value) -> Result<MinTrackSizingFunction, String> {
    match value {
        Value::Keyword(k) if k == "auto" => Ok(MinTrackSizingFunction::auto()),
        Value::Keyword(k) if k == "min-content" => Ok(MinTrackSizingFunction::min_content()),
        Value::Keyword(k) if k == "max-content" => Ok(MinTrackSizingFunction::max_content()),
        Value::Length(l) if l.unit == LengthUnit::Px => {
            Ok(MinTrackSizingFunction::length(l.value as f32))
        }
        Value::Length(l) if l.unit == LengthUnit::Percent => {
            Ok(MinTrackSizingFunction::percent((l.value / 100.0) as f32))
        }
        Value::Flex(_) => {
            Err("is a flex size as a track's minimum, which CSS Grid does not allow".to_owned())
        }
        _ => Err(shape("a track size")),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn track_max(value: &Value) -> Result<MaxTrackSizingFunction, String> {
    match value {
        Value::Flex(f) => Ok(MaxTrackSizingFunction::fr(*f as f32)),
        Value::Keyword(k) if k == "auto" => Ok(MaxTrackSizingFunction::auto()),
        Value::Keyword(k) if k == "min-content" => Ok(MaxTrackSizingFunction::min_content()),
        Value::Keyword(k) if k == "max-content" => Ok(MaxTrackSizingFunction::max_content()),
        Value::Length(l) if l.unit == LengthUnit::Px => {
            Ok(MaxTrackSizingFunction::length(l.value as f32))
        }
        Value::Length(l) if l.unit == LengthUnit::Percent => {
            Ok(MaxTrackSizingFunction::percent((l.value / 100.0) as f32))
        }
        _ => Err(shape("a track size")),
    }
}

fn auto_flow(value: &Value) -> Result<GridAutoFlow, String> {
    let words: Vec<String> = match value {
        Value::List(items) => items.iter().map(Value::to_css).collect(),
        single => vec![single.to_css()],
    };
    match words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["row"] => Ok(GridAutoFlow::Row),
        ["column"] => Ok(GridAutoFlow::Column),
        ["row", "dense"] | ["dense"] => Ok(GridAutoFlow::RowDense),
        ["column", "dense"] => Ok(GridAutoFlow::ColumnDense),
        _ => Err(unknown(&value.to_css())),
    }
}

fn placement(value: &Value) -> Result<GridPlacement<String>, String> {
    match value {
        Value::Keyword(k) if k == "auto" => Ok(GridPlacement::Auto),
        Value::Integer(n) => {
            let index =
                i16::try_from(*n).map_err(|_| format!("is line {n}, past what a grid holds"))?;
            if index == 0 {
                return Err("is line 0, which CSS Grid does not have".to_owned());
            }
            Ok(line(index))
        }
        Value::List(parts) => match parts.as_slice() {
            [Value::Keyword(k), Value::Integer(n)] if k == "span" => {
                Ok(span(u16::try_from(*n).map_err(|_| {
                    format!("spans {n}, more than a grid holds")
                })?))
            }
            _ => Err(shape("a grid line")),
        },
        _ => Err(shape("a grid line")),
    }
}

fn justify_items(value: &Value) -> Result<taffy::AlignItems, String> {
    match keyword(value)? {
        "left" | "right" => {
            Err("`left` and `right` are not scheduled: use start or end".to_owned())
        }
        _ => align_items(value),
    }
}

fn justify_self(value: &Value) -> Result<Option<taffy::AlignItems>, String> {
    match keyword(value)? {
        "left" | "right" => {
            Err("`left` and `right` are not scheduled: use start or end".to_owned())
        }
        "normal" => Ok(Some(taffy::AlignItems::NORMAL)),
        _ => align_self(value),
    }
}
