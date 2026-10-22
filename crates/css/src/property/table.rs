//! The property table. Longhands only; shorthands live in `shorthand`.
//! Sorted by name. Each entry: name, inherited, initial value as the
//! canonical text of the value, and the value grammar (see `syntax`).
//!
//! The flex and grid entries are what Taffy 0.9's `Style` reads.

/// One longhand property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropertyDef {
    /// The property name.
    pub name: &'static str,
    /// Whether the computed value inherits from the parent by default.
    pub inherited: bool,
    /// The initial value, in canonical value text.
    pub initial: &'static str,
    /// The value grammar.
    pub syntax: &'static str,
}

const fn def(
    name: &'static str,
    inherited: bool,
    initial: &'static str,
    syntax: &'static str,
) -> PropertyDef {
    PropertyDef {
        name,
        inherited,
        initial,
        syntax,
    }
}

const ALIGN: &str = "normal | stretch | center | start | end | flex-start | flex-end | baseline";
const ALIGN_CONTENT: &str = "normal | stretch | center | start | end | flex-start | flex-end | space-between | space-around | space-evenly";
const JUSTIFY_ITEMS: &str =
    "normal | stretch | center | start | end | flex-start | flex-end | baseline | left | right";
const SELF: &str =
    "auto | normal | stretch | center | start | end | flex-start | flex-end | baseline";
const BORDER_STYLE: &str = "none | hidden | solid | dashed | dotted | double";
const BORDER_WIDTH: &str = "thin | medium | thick | <length [0,inf]>";
const RADIUS: &str = "<length-percentage [0,inf]>";
const INSET: &str = "auto | <length-percentage>";
const MARGIN: &str = "auto | <length-percentage>";
const PADDING: &str = "<length-percentage [0,inf]>";
const SIZE: &str = "auto | min-content | max-content | fit-content | <length-percentage [0,inf]>";
const MAX_SIZE: &str =
    "none | min-content | max-content | fit-content | <length-percentage [0,inf]>";
const GAP: &str = "normal | <length-percentage [0,inf]>";
const OVERFLOW: &str = "visible | hidden | clip | scroll | auto";
const GRID_LINE: &str = "auto | <integer> | span <integer [1,inf]>";
const CURSOR: &str = "auto | default | none | pointer | text | move | grab | grabbing | not-allowed | crosshair | col-resize | row-resize | ew-resize | ns-resize | nesw-resize | nwse-resize | wait | progress | help | zoom-in | zoom-out";

/// Every longhand, sorted by name.
pub static PROPERTIES: &[PropertyDef] = &[
    def("align-content", false, "normal", ALIGN_CONTENT),
    def("align-items", false, "normal", ALIGN),
    def("align-self", false, "auto", SELF),
    def("animation-delay", false, "0s", "<time>#"),
    def(
        "animation-direction",
        false,
        "normal",
        "[ normal | reverse | alternate | alternate-reverse ]#",
    ),
    def("animation-duration", false, "0s", "<time [0,inf]>#"),
    def(
        "animation-fill-mode",
        false,
        "none",
        "[ none | forwards | backwards | both ]#",
    ),
    def(
        "animation-iteration-count",
        false,
        "1",
        "[ infinite | <number [0,inf]> ]#",
    ),
    def(
        "animation-name",
        false,
        "none",
        "[ none | <custom-ident> ]#",
    ),
    def(
        "animation-play-state",
        false,
        "running",
        "[ running | paused ]#",
    ),
    def(
        "animation-timing-function",
        false,
        "ease",
        "<easing-function>#",
    ),
    def("aspect-ratio", false, "auto", "auto | <ratio>"),
    def("background-color", false, "transparent", "<color>"),
    def("border-bottom-color", false, "currentcolor", "<color>"),
    def("border-bottom-left-radius", false, "0px", RADIUS),
    def("border-bottom-right-radius", false, "0px", RADIUS),
    def("border-bottom-style", false, "none", BORDER_STYLE),
    def("border-bottom-width", false, "medium", BORDER_WIDTH),
    def("border-left-color", false, "currentcolor", "<color>"),
    def("border-left-style", false, "none", BORDER_STYLE),
    def("border-left-width", false, "medium", BORDER_WIDTH),
    def("border-right-color", false, "currentcolor", "<color>"),
    def("border-right-style", false, "none", BORDER_STYLE),
    def("border-right-width", false, "medium", BORDER_WIDTH),
    def("border-top-color", false, "currentcolor", "<color>"),
    def("border-top-left-radius", false, "0px", RADIUS),
    def("border-top-right-radius", false, "0px", RADIUS),
    def("border-top-style", false, "none", BORDER_STYLE),
    def("border-top-width", false, "medium", BORDER_WIDTH),
    def("bottom", false, "auto", INSET),
    def("box-shadow", false, "none", "none | <shadow>#"),
    def(
        "box-sizing",
        false,
        "content-box",
        "content-box | border-box",
    ),
    def("color", true, "rgb(0 0 0)", "<color>"),
    def("column-gap", false, "normal", GAP),
    def("cursor", true, "auto", CURSOR),
    def(
        "display",
        false,
        "inline",
        "block | inline | inline-block | flex | inline-flex | grid | inline-grid | none | contents",
    ),
    def("filter", false, "none", "none | <filter-function>+"),
    def(
        "flex-basis",
        false,
        "auto",
        "auto | content | <length-percentage [0,inf]>",
    ),
    def(
        "flex-direction",
        false,
        "row",
        "row | row-reverse | column | column-reverse",
    ),
    def("flex-grow", false, "0", "<number [0,inf]>"),
    def("flex-shrink", false, "1", "<number [0,inf]>"),
    def("flex-wrap", false, "nowrap", "nowrap | wrap | wrap-reverse"),
    def("font-family", true, "sans-serif", "<font-family-name>#"),
    def(
        "font-feature-settings",
        true,
        "normal",
        "normal | [ <opentype-tag> [ <integer [0,inf]> | on | off ]? ]#",
    ),
    def("font-size", true, "16px", "<length-percentage [0,inf]>"),
    def("font-style", true, "normal", "normal | italic | oblique"),
    def(
        "font-variation-settings",
        true,
        "normal",
        "normal | [ <opentype-tag> <number> ]#",
    ),
    def(
        "font-weight",
        true,
        "normal",
        "normal | bold | bolder | lighter | <number [1,1000]>",
    ),
    def("grid-auto-columns", false, "auto", "<track-size>+"),
    def(
        "grid-auto-flow",
        false,
        "row",
        "row | column | row dense | column dense | dense",
    ),
    def("grid-auto-rows", false, "auto", "<track-size>+"),
    def("grid-column-end", false, "auto", GRID_LINE),
    def("grid-column-start", false, "auto", GRID_LINE),
    def("grid-row-end", false, "auto", GRID_LINE),
    def("grid-row-start", false, "auto", GRID_LINE),
    def(
        "grid-template-columns",
        false,
        "none",
        "none | <track-list>",
    ),
    def("grid-template-rows", false, "none", "none | <track-list>"),
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
