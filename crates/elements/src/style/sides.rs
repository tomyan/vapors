//! The longhands of each box side and corner, as ids in the order taffy
//! and `Decoration` keep them: sides top, right, bottom, left; corners
//! top-left, top-right, bottom-right, bottom-left.

use vapors_css::property::{PropertyId, longhands};

pub(crate) const MARGINS: [PropertyId; 4] = [
    longhands::MARGIN_TOP,
    longhands::MARGIN_RIGHT,
    longhands::MARGIN_BOTTOM,
    longhands::MARGIN_LEFT,
];

pub(crate) const PADDINGS: [PropertyId; 4] = [
    longhands::PADDING_TOP,
    longhands::PADDING_RIGHT,
    longhands::PADDING_BOTTOM,
    longhands::PADDING_LEFT,
];

pub(crate) const BORDER_STYLES: [PropertyId; 4] = [
    longhands::BORDER_TOP_STYLE,
    longhands::BORDER_RIGHT_STYLE,
    longhands::BORDER_BOTTOM_STYLE,
    longhands::BORDER_LEFT_STYLE,
];

pub(crate) const BORDER_WIDTHS: [PropertyId; 4] = [
    longhands::BORDER_TOP_WIDTH,
    longhands::BORDER_RIGHT_WIDTH,
    longhands::BORDER_BOTTOM_WIDTH,
    longhands::BORDER_LEFT_WIDTH,
];

pub(crate) const BORDER_COLORS: [PropertyId; 4] = [
    longhands::BORDER_TOP_COLOR,
    longhands::BORDER_RIGHT_COLOR,
    longhands::BORDER_BOTTOM_COLOR,
    longhands::BORDER_LEFT_COLOR,
];

pub(crate) const BORDER_RADII: [PropertyId; 4] = [
    longhands::BORDER_TOP_LEFT_RADIUS,
    longhands::BORDER_TOP_RIGHT_RADIUS,
    longhands::BORDER_BOTTOM_RIGHT_RADIUS,
    longhands::BORDER_BOTTOM_LEFT_RADIUS,
];
