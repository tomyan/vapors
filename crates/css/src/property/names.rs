//! The vendor-prefixed names dropped on purpose, and the sumi terminal
//! extras refused with a hint.

/// Vendor-prefixed properties the crate drops without refusing, each with
/// the reason. Anything prefixed and not listed here is refused.
pub static IGNORED_VENDOR_PROPERTIES: &[(&str, &str)] = &[
    (
        "-webkit-font-smoothing",
        "macOS browser hint; vapors sets its own text contrast (see the F1 text slice)",
    ),
    (
        "-moz-osx-font-smoothing",
        "Firefox twin of -webkit-font-smoothing",
    ),
    (
        "-webkit-tap-highlight-color",
        "mobile browser tap flash; vapors has no such flash",
    ),
    (
        "-webkit-user-select",
        "prefixed twin of user-select, which is supported unprefixed",
    ),
];

/// A CSS shorthand the crate knows of but does not expand, and the
/// longhands to write instead.
pub(super) fn unsupported_shorthand_hint(name: &str) -> Option<&'static str> {
    Some(match name {
        "font" => "write font-size, font-family, font-weight, font-style and line-height",
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            "write border-top-width, border-top-style and border-top-color (and so on per side)"
        }
        "background" => "write background-color",
        "outline" | "place-items" | "place-content" | "grid-area" | "grid-template" | "grid" => {
            "write the longhands the table lists"
        }
        _ => return None,
    })
}

/// A sumi terminal extra and the vapors property that replaces it.
pub(super) fn sumi_extra_hint(name: &str) -> Option<&'static str> {
    Some(match name {
        "bold" => "use font-weight: bold",
        "dim" => "use opacity: 0.5 or a lighter color",
        "italic" => "use font-style: italic",
        "underline" => "use text-decoration: underline",
        "strikethrough" => "use text-decoration: line-through",
        "inverse" => "swap color and background-color",
        "border-title" => "draw the title as an element",
        _ => return None,
    })
}

/// A sumi terminal value and the hint for it: `opacity: dim`, the box
/// drawing border styles.
pub(super) fn sumi_value_hint(property: &str, keyword: &str) -> Option<String> {
    let border = property == "border" || property.ends_with("-style");
    match keyword {
        "dim" if property == "opacity" => Some(
            "opacity: dim is sumi's terminal dim attribute; use a number between 0 and 1"
                .to_owned(),
        ),
        "rounded" if border => Some(
            "border: rounded is sumi's box drawing; use border-style: solid with border-radius"
                .to_owned(),
        ),
        "single" | "heavy" | "ascii" if border => {
            Some(format!("{keyword} is a sumi box-drawing border; use solid"))
        }
        _ => None,
    }
}
