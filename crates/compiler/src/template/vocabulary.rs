//! The HTML elements the template recognises, the void ones among
//! them, and the messages for tags that were removed.

/// The recognised element vocabulary (sumi's, plus `canvas`, `br`,
/// `fieldset`, `legend`, `small`, `sub`, `sup`, `figure`, `figcaption`).
const HTML_TAGS: &[&str] = &[
    "a",
    "abbr",
    "article",
    "aside",
    "b",
    "blockquote",
    "br",
    "button",
    "canvas",
    "caption",
    "code",
    "col",
    "colgroup",
    "del",
    "details",
    "dialog",
    "div",
    "em",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "i",
    "img",
    "input",
    "kbd",
    "label",
    "legend",
    "li",
    "main",
    "mark",
    "meter",
    "nav",
    "ol",
    "optgroup",
    "option",
    "p",
    "pre",
    "progress",
    "s",
    "samp",
    "section",
    "select",
    "small",
    "span",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "tr",
    "u",
    "ul",
    "var",
];

/// Elements with no body, which close themselves.
const VOID_TAGS: &[&str] = &["br", "col", "hr", "img", "input"];

pub(crate) fn is_html_tag(name: &str) -> bool {
    HTML_TAGS.binary_search(&name).is_ok()
}

pub(crate) fn is_void(name: &str) -> bool {
    VOID_TAGS.contains(&name)
}

pub(crate) const SLOT_REMOVED: &str = "slots were removed; declare a {snippet name()} inside the component tag and {render name()} in the component template";

#[cfg(test)]
mod tests {
    use super::HTML_TAGS;

    #[test]
    fn vocabulary_is_sorted_for_binary_search() {
        // Then
        assert!(HTML_TAGS.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
