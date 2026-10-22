//! Longhands h to z, sorted by name; `table` has the first half and the
//! grammar constants.

use super::table::{
    GAP, INSET, JUSTIFY_ITEMS, MARGIN, MAX_SIZE, OVERFLOW, PADDING, PropertyDef, SIZE, def,
};

/// Longhands h to z.
pub(super) static SECOND_HALF: &[PropertyDef] = &[
    def("height", false, "auto", SIZE),
    def(
        "justify-content",
        false,
        "normal",
        "normal | stretch | center | start | end | flex-start | flex-end | space-between | space-around | space-evenly | left | right",
    ),
    def("justify-items", false, "normal", JUSTIFY_ITEMS),
    def(
        "justify-self",
        false,
        "auto",
        "auto | normal | stretch | center | start | end | flex-start | flex-end | baseline | left | right",
    ),
    def("left", false, "auto", INSET),
    def("letter-spacing", true, "normal", "normal | <length>"),
    def(
        "line-height",
        true,
        "normal",
        "normal | <number [0,inf]> | <length-percentage [0,inf]>",
    ),
    def("margin-bottom", false, "0px", MARGIN),
    def("margin-left", false, "0px", MARGIN),
    def("margin-right", false, "0px", MARGIN),
    def("margin-top", false, "0px", MARGIN),
    def("max-height", false, "none", MAX_SIZE),
    def("max-width", false, "none", MAX_SIZE),
    def("min-height", false, "auto", SIZE),
    def("min-width", false, "auto", SIZE),
    def(
        "opacity",
        false,
        "1",
        "<number [0,1]> | <percentage [0,100]>",
    ),
    def("overflow-x", false, "visible", OVERFLOW),
    def("overflow-y", false, "visible", OVERFLOW),
    def("padding-bottom", false, "0px", PADDING),
    def("padding-left", false, "0px", PADDING),
    def("padding-right", false, "0px", PADDING),
    def("padding-top", false, "0px", PADDING),
    def(
        "position",
        false,
        "static",
        "static | relative | absolute | fixed | sticky",
    ),
    def("right", false, "auto", INSET),
    def("row-gap", false, "normal", GAP),
    def(
        "text-align",
        true,
        "start",
        "start | end | left | right | center | justify",
    ),
    def(
        "text-decoration",
        false,
        "none",
        "none | underline line-through | line-through underline | underline | line-through",
    ),
    def("top", false, "auto", INSET),
    def("transform", false, "none", "none | <transform-function>+"),
    def("transition-delay", false, "0s", "<time>#"),
    def("transition-duration", false, "0s", "<time [0,inf]>#"),
    def(
        "transition-property",
        false,
        "all",
        "none | [ all | <custom-ident> ]#",
    ),
    def(
        "transition-timing-function",
        false,
        "ease",
        "<easing-function>#",
    ),
    def(
        "user-select",
        false,
        "auto",
        "auto | text | none | contain | all",
    ),
    def("visibility", true, "visible", "visible | hidden | collapse"),
    def(
        "white-space",
        true,
        "normal",
        "normal | nowrap | pre | pre-wrap | pre-line",
    ),
    def("width", false, "auto", SIZE),
    def("z-index", false, "auto", "auto | <integer>"),
];
