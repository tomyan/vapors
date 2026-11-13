//! Every longhand's [`PropertyId`] as a constant, worked out at compile
//! time by [`PropertyId::named`]: reading a computed value by one of
//! these is an index, not a search by name, and a misspelt name stops
//! the build at the constant that holds it.

use super::PropertyId;

macro_rules! longhands {
    ($($id:ident = $name:literal,)*) => {
        $(
            #[doc = concat!("`", $name, "`")]
            pub const $id: PropertyId = PropertyId::named($name);
        )*

        /// Every constant above, in the order written.
        #[cfg(test)]
        const WRITTEN: &[PropertyId] = &[$($id),*];
    };
}

longhands! {
    ALIGN_CONTENT = "align-content",
    ALIGN_ITEMS = "align-items",
    ALIGN_SELF = "align-self",
    ANIMATION_DELAY = "animation-delay",
    ANIMATION_DIRECTION = "animation-direction",
    ANIMATION_DURATION = "animation-duration",
    ANIMATION_FILL_MODE = "animation-fill-mode",
    ANIMATION_ITERATION_COUNT = "animation-iteration-count",
    ANIMATION_NAME = "animation-name",
    ANIMATION_PLAY_STATE = "animation-play-state",
    ANIMATION_TIMING_FUNCTION = "animation-timing-function",
    ASPECT_RATIO = "aspect-ratio",
    BACKGROUND_COLOR = "background-color",
    BORDER_BOTTOM_COLOR = "border-bottom-color",
    BORDER_BOTTOM_LEFT_RADIUS = "border-bottom-left-radius",
    BORDER_BOTTOM_RIGHT_RADIUS = "border-bottom-right-radius",
    BORDER_BOTTOM_STYLE = "border-bottom-style",
    BORDER_BOTTOM_WIDTH = "border-bottom-width",
    BORDER_LEFT_COLOR = "border-left-color",
    BORDER_LEFT_STYLE = "border-left-style",
    BORDER_LEFT_WIDTH = "border-left-width",
    BORDER_RIGHT_COLOR = "border-right-color",
    BORDER_RIGHT_STYLE = "border-right-style",
    BORDER_RIGHT_WIDTH = "border-right-width",
    BORDER_TOP_COLOR = "border-top-color",
    BORDER_TOP_LEFT_RADIUS = "border-top-left-radius",
    BORDER_TOP_RIGHT_RADIUS = "border-top-right-radius",
    BORDER_TOP_STYLE = "border-top-style",
    BORDER_TOP_WIDTH = "border-top-width",
    BOTTOM = "bottom",
    BOX_SHADOW = "box-shadow",
    BOX_SIZING = "box-sizing",
    COLOR = "color",
    COLUMN_GAP = "column-gap",
    CONTAINER_NAME = "container-name",
    CONTAINER_TYPE = "container-type",
    CURSOR = "cursor",
    DISPLAY = "display",
    FILTER = "filter",
    FLEX_BASIS = "flex-basis",
    FLEX_DIRECTION = "flex-direction",
    FLEX_GROW = "flex-grow",
    FLEX_SHRINK = "flex-shrink",
    FLEX_WRAP = "flex-wrap",
    FONT_FAMILY = "font-family",
    FONT_FEATURE_SETTINGS = "font-feature-settings",
    FONT_SIZE = "font-size",
    FONT_STYLE = "font-style",
    FONT_VARIATION_SETTINGS = "font-variation-settings",
    FONT_WEIGHT = "font-weight",
    GRID_AUTO_COLUMNS = "grid-auto-columns",
    GRID_AUTO_FLOW = "grid-auto-flow",
    GRID_AUTO_ROWS = "grid-auto-rows",
    GRID_COLUMN_END = "grid-column-end",
    GRID_COLUMN_START = "grid-column-start",
    GRID_ROW_END = "grid-row-end",
    GRID_ROW_START = "grid-row-start",
    GRID_TEMPLATE_COLUMNS = "grid-template-columns",
    GRID_TEMPLATE_ROWS = "grid-template-rows",
    HEIGHT = "height",
    JUSTIFY_CONTENT = "justify-content",
    JUSTIFY_ITEMS = "justify-items",
    JUSTIFY_SELF = "justify-self",
    LEFT = "left",
    LETTER_SPACING = "letter-spacing",
    LINE_HEIGHT = "line-height",
    MARGIN_BOTTOM = "margin-bottom",
    MARGIN_LEFT = "margin-left",
    MARGIN_RIGHT = "margin-right",
    MARGIN_TOP = "margin-top",
    MAX_HEIGHT = "max-height",
    MAX_WIDTH = "max-width",
    MIN_HEIGHT = "min-height",
    MIN_WIDTH = "min-width",
    OPACITY = "opacity",
    OUTLINE_COLOR = "outline-color",
    OUTLINE_OFFSET = "outline-offset",
    OUTLINE_STYLE = "outline-style",
    OUTLINE_WIDTH = "outline-width",
    OVERFLOW_X = "overflow-x",
    OVERFLOW_Y = "overflow-y",
    PADDING_BOTTOM = "padding-bottom",
    PADDING_LEFT = "padding-left",
    PADDING_RIGHT = "padding-right",
    PADDING_TOP = "padding-top",
    POINTER_EVENTS = "pointer-events",
    POSITION = "position",
    RIGHT = "right",
    ROW_GAP = "row-gap",
    SCROLLBAR_COLOR = "scrollbar-color",
    SCROLLBAR_WIDTH = "scrollbar-width",
    TEXT_ALIGN = "text-align",
    TEXT_DECORATION = "text-decoration",
    TOP = "top",
    TRANSFORM = "transform",
    TRANSITION_DELAY = "transition-delay",
    TRANSITION_DURATION = "transition-duration",
    TRANSITION_PROPERTY = "transition-property",
    TRANSITION_TIMING_FUNCTION = "transition-timing-function",
    USER_SELECT = "user-select",
    VISIBILITY = "visibility",
    WHITE_SPACE = "white-space",
    WIDTH = "width",
    Z_INDEX = "z-index",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_longhand_has_one_constant_in_table_order() {
        // When
        let written: Vec<&str> = WRITTEN.iter().map(|id| id.name()).collect();
        let table: Vec<&str> = PropertyId::all().map(PropertyId::name).collect();

        // Then: compared by name, so a failure names the property
        assert_eq!(
            written, table,
            "the constants in crates/css/src/property/longhands.rs and the table in table.rs and table_rest.rs disagree; add, remove or reorder constants to match the table"
        );
    }
}
