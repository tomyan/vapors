//! Basic value types (`src/syntax/types.rs`) through the property grammars.

mod support;

use support::property_values::{accepts, accepts_as};
use support::refusal;

#[test]
fn keyword_properties() {
    for decl in [
        "display: flex",
        "display: grid",
        "display: none",
        "display: inline-flex",
        "position: absolute",
        "box-sizing: border-box",
        "overflow-x: scroll",
        "visibility: hidden",
        "white-space: pre-wrap",
        "text-align: center",
        "user-select: none",
        "cursor: grab",
        "flex-direction: column-reverse",
        "flex-wrap: wrap",
        "align-items: center",
        "justify-content: space-between",
        "align-self: stretch",
        "grid-auto-flow: row dense",
        "font-style: italic",
        "font-weight: bold",
        "text-decoration: underline line-through",
    ] {
        accepts(decl);
    }
    assert!(
        refusal("a { display: table-cell; }")
            .message
            .contains("display")
    );
}

#[test]
fn length_properties_take_lengths_percentages_auto_and_calc() {
    for decl in [
        "width: 10px",
        "width: 50%",
        "width: auto",
        "width: calc(100% - 10px)",
        "max-height: none",
        "margin-top: auto",
        "margin-left: -4px",
        "padding-left: 1em",
        "top: 2rem",
        "flex-basis: 10vw",
        "letter-spacing: normal",
        "letter-spacing: 0.1em",
        "border-top-width: thin",
        "border-top-left-radius: 50%",
    ] {
        accepts(decl);
    }
    for bad in [
        "width: 10",
        "padding-left: -1px",
        "border-top-width: 50%",
        "width: 1cm",
    ] {
        assert!(
            refusal(&format!("a {{ {bad}; }}"))
                .message
                .contains("length"),
            "{bad}"
        );
    }
}

#[test]
fn number_and_integer_properties() {
    for decl in [
        "opacity: 0.5",
        "opacity: 50%",
        "flex-grow: 2",
        "flex-shrink: 0",
        "z-index: 10",
        "z-index: -1",
        "z-index: auto",
        "line-height: 1.4",
        "line-height: 20px",
        "line-height: normal",
        "font-weight: 650",
        "aspect-ratio: 16 / 9",
        "aspect-ratio: auto",
    ] {
        accepts(decl);
    }
    for bad in [
        "z-index: 1.5",
        "opacity: 2px",
        "font-weight: 1001",
        "flex-grow: -1",
    ] {
        assert!(
            refusal(&format!("a {{ {bad}; }}")).message.contains(':'),
            "{bad}"
        );
    }
}

#[test]
fn colour_properties() {
    for (decl, want) in [
        ("color: red", "color: rgb(255 0 0)"),
        ("color: #0f0", "color: rgb(0 255 0)"),
        (
            "color: rgb(1 2 3 / 50%)",
            "color: rgb(1 2 3 / 0.5019607843137255)",
        ),
        ("color: currentColor", "color: currentcolor"),
        (
            "color: light-dark(white, black)",
            "color: light-dark(rgb(255 255 255), rgb(0 0 0))",
        ),
        (
            "background-color: transparent",
            "background-color: transparent",
        ),
        (
            "border-top-color: oklch(0.7 0.1 200)",
            "border-top-color: rgb(64 177 183)",
        ),
    ] {
        accepts_as(decl, want);
    }
}

#[test]
fn time_and_easing_longhands() {
    for decl in [
        "transition-duration: 200ms",
        "transition-duration: 0.5s, 1s",
        "transition-delay: 0s",
        "transition-timing-function: ease-in-out",
        "transition-timing-function: cubic-bezier(0.1, 0.7, 1, 0.1)",
        "transition-timing-function: steps(4, start)",
        "transition-property: color, background-color",
        "transition-property: all",
        "transition-property: none",
        "animation-name: pulse",
        "animation-name: none",
        "animation-iteration-count: infinite",
        "animation-iteration-count: 3",
        "animation-direction: alternate",
        "animation-fill-mode: both",
        "animation-play-state: paused",
    ] {
        accepts(decl);
    }
    for bad in [
        "transition-duration: 200",
        "transition-duration: -1s",
        "transition-timing-function: bounce",
        "animation-name: 3",
    ] {
        assert!(
            refusal(&format!("a {{ {bad}; }}")).message.contains(':'),
            "{bad}"
        );
    }
}

#[test]
fn gap_longhands() {
    accepts("row-gap: 4px");
    accepts("column-gap: 2%");
    accepts("row-gap: normal");
}

#[test]
fn container_type_and_name_declare_a_container() {
    // Given: review item 15.
    accepts("container-type: inline-size");
    accepts("container-type: size");
    accepts("container-name: sidebar");
    accepts("container-name: none");
}
