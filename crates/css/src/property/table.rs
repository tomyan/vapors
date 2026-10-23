//! The property table. Longhands only; shorthands live in `shorthand`.
//! Sorted by name. Each entry: name, inherited, initial value as the
//! canonical text of the value, and the value grammar (see `syntax`).
//!
//! The flex and grid entries are what Taffy 0.15's `Style` reads.

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

pub(super) const fn def(
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

pub(super) const ALIGN: &str =
    "normal | stretch | center | start | end | flex-start | flex-end | baseline";
pub(super) const ALIGN_CONTENT: &str = "normal | stretch | center | start | end | flex-start | flex-end | space-between | space-around | space-evenly";
pub(super) const JUSTIFY_ITEMS: &str =
    "normal | stretch | center | start | end | flex-start | flex-end | baseline | left | right";
pub(super) const SELF: &str =
    "auto | normal | stretch | center | start | end | flex-start | flex-end | baseline";
pub(super) const BORDER_STYLE: &str = "none | hidden | solid | dashed | dotted | double";
pub(super) const BORDER_WIDTH: &str = "thin | medium | thick | <length [0,inf]>";
pub(super) const RADIUS: &str = "<length-percentage [0,inf]>";
pub(super) const INSET: &str = "auto | <length-percentage>";
pub(super) const MARGIN: &str = "auto | <length-percentage>";
pub(super) const PADDING: &str = "<length-percentage [0,inf]>";
pub(super) const SIZE: &str =
    "auto | min-content | max-content | fit-content | <length-percentage [0,inf]>";
pub(super) const MAX_SIZE: &str =
    "none | min-content | max-content | fit-content | <length-percentage [0,inf]>";
pub(super) const GAP: &str = "normal | <length-percentage [0,inf]>";
pub(super) const OVERFLOW: &str = "visible | hidden | clip | scroll | auto";
pub(super) const GRID_LINE: &str = "auto | <integer> | span <integer [1,inf]>";
pub(super) const CURSOR: &str = "auto | default | none | pointer | text | move | grab | grabbing | not-allowed | crosshair | col-resize | row-resize | ew-resize | ns-resize | nesw-resize | nwse-resize | wait | progress | help | zoom-in | zoom-out";

/// Longhands a to g, sorted by name; `table_rest` has the rest.
pub(super) static FIRST_HALF: &[PropertyDef] = &[
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
    def("container-name", false, "none", "none | <custom-ident>+"),
    def(
        "container-type",
        false,
        "normal",
        "normal | size | inline-size",
    ),
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
];
