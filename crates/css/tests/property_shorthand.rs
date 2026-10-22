//! Shorthand expansion into longhands. vapors-specific; sumi stored
//! shorthands as strings.

mod support;

use support::{refusal, sheet};

fn longhands(decl: &str) -> Vec<String> {
    let s = sheet(&format!("a {{ {decl}; }}"));
    s.rules[0].declarations[0]
        .longhands
        .iter()
        .map(|l| format!("{}: {}", l.id.name(), l.value.to_css()))
        .collect()
}

#[test]
fn margin_and_padding_expand_one_to_four_values() {
    assert_eq!(
        longhands("margin: 1px"),
        [
            "margin-top: 1px",
            "margin-right: 1px",
            "margin-bottom: 1px",
            "margin-left: 1px"
        ]
    );
    assert_eq!(
        longhands("padding: 1px 2px"),
        [
            "padding-top: 1px",
            "padding-right: 2px",
            "padding-bottom: 1px",
            "padding-left: 2px"
        ]
    );
    assert_eq!(
        longhands("margin: 1px 2px 3px"),
        [
            "margin-top: 1px",
            "margin-right: 2px",
            "margin-bottom: 3px",
            "margin-left: 2px"
        ]
    );
    assert_eq!(
        longhands("inset: 1px 2px 3px 4px"),
        ["top: 1px", "right: 2px", "bottom: 3px", "left: 4px"]
    );
    assert!(
        refusal("a { margin: 1px 2px 3px 4px 5px; }")
            .message
            .contains("margin")
    );
}

#[test]
fn border_sides_expand() {
    assert_eq!(
        longhands("border-width: 1px 2px"),
        [
            "border-top-width: 1px",
            "border-right-width: 2px",
            "border-bottom-width: 1px",
            "border-left-width: 2px"
        ]
    );
    assert_eq!(
        longhands("border-color: red")[2],
        "border-bottom-color: rgb(255 0 0)"
    );
    assert_eq!(
        longhands("border-style: solid none")[1],
        "border-right-style: none"
    );
}

#[test]
fn border_shorthand_sets_width_style_and_colour_in_any_order() {
    // When
    let got = longhands("border: red 1px solid");

    // Then
    assert_eq!(got.len(), 12);
    assert!(got.contains(&"border-top-width: 1px".to_owned()));
    assert!(got.contains(&"border-left-style: solid".to_owned()));
    assert!(got.contains(&"border-bottom-color: rgb(255 0 0)".to_owned()));
}

#[test]
fn border_shorthand_fills_omitted_parts_with_initial_values() {
    let got = longhands("border: 2px");
    assert!(got.contains(&"border-top-style: none".to_owned()));
    assert!(got.contains(&"border-top-color: currentcolor".to_owned()));
    assert_eq!(longhands("border: none").len(), 12);
}

#[test]
fn border_radius_expands_corners() {
    assert_eq!(
        longhands("border-radius: 4px"),
        [
            "border-top-left-radius: 4px",
            "border-top-right-radius: 4px",
            "border-bottom-right-radius: 4px",
            "border-bottom-left-radius: 4px"
        ]
    );
    assert_eq!(
        longhands("border-radius: 1px 2px")[3],
        "border-bottom-left-radius: 2px"
    );
    assert!(
        refusal("a { border-radius: 1px / 2px; }")
            .message
            .contains('/')
    );
}

#[test]
fn gap_overflow_and_flex_expand() {
    assert_eq!(longhands("gap: 4px"), ["row-gap: 4px", "column-gap: 4px"]);
    assert_eq!(
        longhands("gap: 4px 8px"),
        ["row-gap: 4px", "column-gap: 8px"]
    );
    assert_eq!(
        longhands("overflow: hidden"),
        ["overflow-x: hidden", "overflow-y: hidden"]
    );
    assert_eq!(
        longhands("overflow: hidden auto"),
        ["overflow-x: hidden", "overflow-y: auto"]
    );
    assert_eq!(
        longhands("flex: 1"),
        ["flex-grow: 1", "flex-shrink: 1", "flex-basis: 0px"]
    );
    assert_eq!(
        longhands("flex: none"),
        ["flex-grow: 0", "flex-shrink: 0", "flex-basis: auto"]
    );
    assert_eq!(
        longhands("flex: auto"),
        ["flex-grow: 1", "flex-shrink: 1", "flex-basis: auto"]
    );
    assert_eq!(
        longhands("flex: 2 0 10px"),
        ["flex-grow: 2", "flex-shrink: 0", "flex-basis: 10px"]
    );
    assert_eq!(
        longhands("flex: 0 0"),
        ["flex-grow: 0", "flex-shrink: 0", "flex-basis: 0px"]
    );
}

#[test]
fn grid_row_and_column_expand_start_and_end() {
    assert_eq!(
        longhands("grid-row: 1 / 3"),
        ["grid-row-start: 1", "grid-row-end: 3"]
    );
    assert_eq!(
        longhands("grid-column: span 2"),
        ["grid-column-start: span 2", "grid-column-end: auto"]
    );
    assert_eq!(
        longhands("grid-column: 2"),
        ["grid-column-start: 2", "grid-column-end: auto"]
    );
}

#[test]
fn transition_shorthand_expands_per_item() {
    // When
    let got = longhands("transition: color 200ms ease-out 50ms, background-color 1s");

    // Then
    assert_eq!(
        got,
        [
            "transition-property: color, background-color",
            "transition-duration: 200ms, 1s",
            "transition-timing-function: ease-out, ease",
            "transition-delay: 50ms, 0s",
        ]
    );
}

#[test]
fn transition_shorthand_defaults_and_refusals() {
    assert_eq!(
        longhands("transition: all 300ms linear"),
        [
            "transition-property: all",
            "transition-duration: 300ms",
            "transition-timing-function: linear",
            "transition-delay: 0s",
        ]
    );
    assert_eq!(
        longhands("transition: none")[0],
        "transition-property: none"
    );
    // sumi dropped a transition with no duration; vapors refuses it.
    assert!(
        refusal("a { transition: color; }")
            .message
            .contains("duration")
    );
    assert!(
        refusal("a { transition: color 1s 2s 3s; }")
            .message
            .contains("time")
    );
}

#[test]
fn animation_shorthand_classifies_tokens_by_shape() {
    // When
    let got = longhands("animation: pulse 2s infinite ease-in-out");

    // Then
    assert_eq!(
        got,
        [
            "animation-name: pulse",
            "animation-duration: 2s",
            "animation-timing-function: ease-in-out",
            "animation-delay: 0s",
            "animation-iteration-count: infinite",
            "animation-direction: normal",
            "animation-fill-mode: none",
            "animation-play-state: running",
        ]
    );
    let got = longhands("animation: bounce 1s alternate 3 500ms forwards paused");
    assert!(got.contains(&"animation-iteration-count: 3".to_owned()));
    assert!(got.contains(&"animation-direction: alternate".to_owned()));
    assert!(got.contains(&"animation-delay: 500ms".to_owned()));
    assert!(got.contains(&"animation-fill-mode: forwards".to_owned()));
    assert!(got.contains(&"animation-play-state: paused".to_owned()));
    assert!(
        refusal("a { animation: fade; }")
            .message
            .contains("duration")
    );
}

#[test]
fn important_applies_to_every_longhand() {
    let s = sheet("a { margin: 1px !important; }");
    assert!(s.rules[0].declarations[0].important);
    assert_eq!(s.rules[0].declarations[0].longhands.len(), 4);
}
