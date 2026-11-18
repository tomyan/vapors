//! The template's sections: elements, inputs, events, attributes.

use vapors_compiler::{BOOLEAN_ATTRIBUTES, HTML_TAGS, VOID_TAGS, compile_str};
use vapors_elements::{FIRED_EVENTS, INPUT_TYPES, LATER_TAGS, SUPPORTED_TAGS};

use crate::codes;

/// Templates whose refusals the reference quotes, as the compiler words them.
const REFUSED: &[&str] = &[
    "<title>x</title>",
    "<slot></slot>",
    "<blink></blink>",
    "<!-- note -->",
];

pub fn section() -> String {
    [elements(), refused(), inputs(), events()].join("\n")
}

fn elements() -> String {
    let drawn: Vec<&str> = HTML_TAGS
        .iter()
        .copied()
        .filter(|tag| SUPPORTED_TAGS.contains(tag))
        .collect();
    let later = LATER_TAGS
        .iter()
        .map(|(slice, tags)| format!("- {slice}: {}\n", codes(tags.iter().copied())))
        .collect::<Vec<_>>()
        .concat();
    format!(
        "## Elements\n\nDrawn by the element tree:\n{}.\n\nVoid (they close themselves): {}.\n\n\
         Accepted by the compiler but refused by the element tree when the page runs, by the \
         slice that brings them:\n\n{later}",
        codes(drawn),
        codes(VOID_TAGS.iter().copied()),
    )
}

fn refused() -> String {
    let lines = REFUSED
        .iter()
        .map(|template| {
            let message = compile_str("probe.wisp", template)
                .err()
                .map_or("accepted".to_owned(), |error| error.message);
            format!("- `{template}`: {message}\n")
        })
        .collect::<Vec<_>>()
        .concat();
    format!(
        "## Refused in a template\n\nAny other lowercase tag is refused. The compiler's words:\n\n{lines}"
    )
}

fn inputs() -> String {
    let built = INPUT_TYPES
        .iter()
        .filter(|(_, slice)| slice.is_none())
        .map(|(kind, _)| *kind);
    format!(
        "## Inputs and attributes\n\n`<input type>` values the tree builds: {}; an `input` with no \
         `type` is `text`.\n\nThese HTML boolean attributes are on by being present, so the compiler \
         refuses them from an expression (`disabled={{..}}`); write the literal on an element inside \
         `{{if}}`, or use `bind:checked`: {}.\n",
        codes(built),
        codes(BOOLEAN_ATTRIBUTES.iter().copied()),
    )
}

fn events() -> String {
    format!(
        "## Events\n\nThe element tree fires these; `on<name>={{handler}}` listens for one: {}.\n",
        codes(FIRED_EVENTS.iter().copied())
    )
}
