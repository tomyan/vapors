//! An element's computed style as Taffy's `Style`: the box model, the
//! display type and the flex properties. Every field Taffy reads is set
//! from a computed value or from the CSS initial value, never left at
//! Taffy's own default, which differs from CSS's (`display: flex`,
//! `box-sizing: border-box`).

use taffy::{
    AlignContent, AlignItems, BoxSizing, Dimension, Display, FlexDirection, FlexWrap,
    LengthPercentage, LengthPercentageAuto, Rect, Size, Style,
};
use vapors_css::value::Value;

use super::box_align::{align_content, align_items, align_self};
use super::sizes::{dimension, flex_basis, min_size, none_or_amount};
use super::values::{Amount, Reader, amount, keyword, later, shape, unknown};

/// The Taffy style of the element `reader` reads; `flex_item` when its
/// parent is a flex container, which blockifies it (CSS Display 3,
/// 2.7): an inline-level display lays out as its block-level twin.
/// `border` is the used border widths (top, right, bottom, left).
pub(crate) fn box_style(reader: &mut Reader<'_>, flex_item: bool, border: [f32; 4]) -> Style {
    let display = if flex_item { blockified } else { display };
    let mut style = Style {
        display: reader.read("display", display, Display::Block),
        box_sizing: reader.read("box-sizing", box_sizing, BoxSizing::ContentBox),
        size: Size {
            width: reader.read("width", dimension, Dimension::auto()),
            height: reader.read("height", dimension, Dimension::auto()),
        },
        min_size: Size {
            width: reader.read("min-width", min_size, LengthPercentageAuto::auto()),
            height: reader.read("min-height", min_size, LengthPercentageAuto::auto()),
        },
        max_size: Size {
            width: reader.read("max-width", none_or_amount, LengthPercentageAuto::auto()),
            height: reader.read("max-height", none_or_amount, LengthPercentageAuto::auto()),
        },
        aspect_ratio: reader.read("aspect-ratio", aspect_ratio, None),
        margin: margins(reader),
        padding: paddings(reader),
        border: Rect {
            top: LengthPercentage::length(border[0]),
            right: LengthPercentage::length(border[1]),
            bottom: LengthPercentage::length(border[2]),
            left: LengthPercentage::length(border[3]),
        },
        ..Style::default()
    };
    flex(reader, &mut style);
    super::grid_style::grid(reader, &mut style);
    style
}

fn flex(reader: &mut Reader<'_>, style: &mut Style) {
    style.flex_direction = reader.read("flex-direction", flex_direction, FlexDirection::Row);
    style.flex_wrap = reader.read("flex-wrap", flex_wrap, FlexWrap::NoWrap);
    style.flex_grow = reader.read("flex-grow", number, 0.0);
    style.flex_shrink = reader.read("flex-shrink", number, 1.0);
    style.flex_basis = reader.read("flex-basis", flex_basis, Dimension::auto());
    style.align_items = reader.read("align-items", align_items, AlignItems::NORMAL);
    style.align_self = reader.read("align-self", align_self, None);
    style.align_content = reader.read("align-content", align_content, AlignContent::NORMAL);
    style.justify_content = reader.read("justify-content", align_content, AlignContent::NORMAL);
    style.gap = Size {
        width: reader.read("column-gap", gap, LengthPercentage::length(0.0)),
        height: reader.read("row-gap", gap, LengthPercentage::length(0.0)),
    };
}

fn margins(reader: &mut Reader<'_>) -> Rect<LengthPercentageAuto> {
    let zero = LengthPercentageAuto::length(0.0);
    Rect {
        left: reader.read("margin-left", auto_or_amount, zero),
        right: reader.read("margin-right", auto_or_amount, zero),
        top: reader.read("margin-top", auto_or_amount, zero),
        bottom: reader.read("margin-bottom", auto_or_amount, zero),
    }
}

fn paddings(reader: &mut Reader<'_>) -> Rect<LengthPercentage> {
    let zero = LengthPercentage::length(0.0);
    Rect {
        left: reader.read("padding-left", length_percentage, zero),
        right: reader.read("padding-right", length_percentage, zero),
        top: reader.read("padding-top", length_percentage, zero),
        bottom: reader.read("padding-bottom", length_percentage, zero),
    }
}

fn display(value: &Value) -> Result<Display, String> {
    match keyword(value)? {
        "block" => Ok(Display::Block),
        "flex" => Ok(Display::Flex),
        "none" => Ok(Display::None),
        "grid" => Ok(Display::Grid),
        "inline" | "inline-block" | "inline-flex" | "inline-grid" => Err(later("F2c S6")),
        "contents" => Err("`contents` is not scheduled in docs/F2c.md".to_owned()),
        other => Err(unknown(other)),
    }
}

fn blockified(value: &Value) -> Result<Display, String> {
    match keyword(value)? {
        "inline" | "inline-block" => Ok(Display::Block),
        "inline-flex" => Ok(Display::Flex),
        "inline-grid" => Ok(Display::Grid),
        _ => display(value),
    }
}

fn box_sizing(value: &Value) -> Result<BoxSizing, String> {
    match keyword(value)? {
        "border-box" => Ok(BoxSizing::BorderBox),
        "content-box" => Ok(BoxSizing::ContentBox),
        other => Err(unknown(other)),
    }
}

fn length_percentage(value: &Value) -> Result<LengthPercentage, String> {
    Ok(match amount(value)? {
        Amount::Px(px) => LengthPercentage::length(px),
        Amount::Fraction(f) => LengthPercentage::percent(f),
        Amount::Calc(calc) => LengthPercentage::calc(calc),
    })
}

pub(super) fn auto_or_amount(value: &Value) -> Result<LengthPercentageAuto, String> {
    if value.as_keyword() == Some("auto") {
        return Ok(LengthPercentageAuto::auto());
    }
    Ok(match amount(value)? {
        Amount::Px(px) => LengthPercentageAuto::length(px),
        Amount::Fraction(f) => LengthPercentageAuto::percent(f),
        Amount::Calc(calc) => LengthPercentageAuto::calc(calc),
    })
}

fn gap(value: &Value) -> Result<LengthPercentage, String> {
    match value.as_keyword() {
        Some("normal") => Ok(LengthPercentage::length(0.0)),
        _ => length_percentage(value),
    }
}

#[allow(clippy::cast_possible_truncation)] // ratios and factors are far inside f32
fn aspect_ratio(value: &Value) -> Result<Option<f32>, String> {
    match value {
        Value::Ratio(width, height) => Ok(Some((width / height) as f32)),
        Value::Keyword(k) if k == "auto" => Ok(None),
        _ => Err(shape("a ratio")),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // flex factors are far inside f32
fn number(value: &Value) -> Result<f32, String> {
    match value {
        Value::Number(n) => Ok(*n as f32),
        Value::Integer(n) => Ok(*n as f32),
        _ => Err(shape("a number")),
    }
}

fn flex_direction(value: &Value) -> Result<FlexDirection, String> {
    match keyword(value)? {
        "row-reverse" => Ok(FlexDirection::RowReverse),
        "column" => Ok(FlexDirection::Column),
        "column-reverse" => Ok(FlexDirection::ColumnReverse),
        "row" => Ok(FlexDirection::Row),
        other => Err(unknown(other)),
    }
}

fn flex_wrap(value: &Value) -> Result<FlexWrap, String> {
    match keyword(value)? {
        "wrap" => Ok(FlexWrap::Wrap),
        "wrap-reverse" => Ok(FlexWrap::WrapReverse),
        "nowrap" => Ok(FlexWrap::NoWrap),
        other => Err(unknown(other)),
    }
}
