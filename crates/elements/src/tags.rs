//! Which tags and events the element tree implements in this slice, and
//! where the rest arrive. A tag the compiler accepts but the tree does
//! not implement is an error, never drawn as a `div`.

/// Tags laid out and painted now: block-level containers, the inline
/// elements of F2c S6, `img` (S10) and `input` (3-E3; `check` refuses
/// the types not built yet).
const SUPPORTED: &[&str] = &[
    "html",
    "body",
    "button",
    "input",
    "div",
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "section",
    "header",
    "footer",
    "main",
    "nav",
    "aside",
    "article",
    "blockquote",
    "figure",
    "figcaption",
    "hr",
    "img",
    "span",
    "b",
    "strong",
    "code",
    "kbd",
    "samp",
    "mark",
    "abbr",
    "em",
    "i",
    "var",
];

/// Where each unimplemented tag arrives.
const LATER: &[(&str, &[&str])] = &[
    ("F2d, the first item after F2c", &["br", "a"]),
    ("F3", &["canvas"]),
    (
        "F4",
        &[
            "textarea", "select", "option", "optgroup", "label", "fieldset", "legend", "ul", "ol",
            "li", "pre", "table", "caption", "col", "colgroup", "tbody", "thead", "tfoot", "tr",
            "td", "th", "meter", "progress",
        ],
    ),
    ("F5", &["dialog", "details", "summary"]),
];

/// `None` when the tree implements `tag`; otherwise why not, with the
/// slice that brings it.
pub(crate) fn unsupported(tag: &str) -> Option<String> {
    if SUPPORTED.contains(&tag) {
        return None;
    }
    let reason = match LATER.iter().find(|(_, tags)| tags.contains(&tag)) {
        Some((slice, _)) => format!("arrives in {slice} (docs/F2c.md)"),
        None => "is not scheduled in docs/F2c.md".to_owned(),
    };
    Some(format!(
        "<{tag}> is not implemented by vapors-elements yet: it {reason}; use one of {}",
        SUPPORTED[2..].join(", ")
    ))
}

/// The events the tree fires (F2c S2; capture, 3-E1a; touches, 3-E1b;
/// keys and focus, 3-E2a; a range's value, 3-E3a).
pub(crate) const FIRED: &[&str] = &[
    "click",
    "pointerdown",
    "pointermove",
    "pointerup",
    "gotpointercapture",
    "lostpointercapture",
    "pointercancel",
    "keydown",
    "keyup",
    "focus",
    "blur",
    "focusin",
    "focusout",
    "input",
    "change",
];

/// `None` when the tree fires `event`; otherwise why not.
pub(crate) fn unfired(event: &str) -> Option<String> {
    if FIRED.contains(&event) {
        return None;
    }
    Some(format!(
        "on{event}: vapors-elements never fires `{event}`; it fires {} (docs/F2c.md), so this listener would never run",
        FIRED.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_container_is_supported() {
        // Given
        let tag = "section";

        // When
        let refusal = unsupported(tag);

        // Then
        assert_eq!(refusal, None);
    }

    #[test]
    fn a_scheduled_tag_is_refused_naming_its_slice() {
        // Given
        let tag = "canvas";

        // When
        let refusal = unsupported(tag);

        // Then
        let refusal = refusal.expect("canvas is refused until F3");
        assert!(
            refusal.contains("<canvas>") && refusal.contains("F3"),
            "{refusal}"
        );
    }

    #[test]
    fn an_unscheduled_tag_is_refused_saying_so() {
        // Given
        let tag = "marquee";

        // When
        let refusal = unsupported(tag).expect("refused");

        // Then
        assert!(refusal.contains("not scheduled"), "{refusal}");
    }

    #[test]
    fn line_breaks_and_links_are_refused_naming_f2d() {
        for tag in ["br", "a"] {
            // When
            let refusal = unsupported(tag).expect("refused until F2d");

            // Then
            assert!(refusal.contains("F2d"), "{refusal}");
        }
    }
}
