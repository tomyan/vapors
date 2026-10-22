//! Value syntax per property family. vapors-specific.

mod support;

use support::{refusal, sheet};

/// The longhand values of one declaration, as `name: value` text.
fn longhands(decl: &str) -> Vec<String> {
    let s = sheet(&format!("a {{ {decl}; }}"));
    s.rules[0].declarations[0]
        .longhands
        .iter()
        .map(|l| format!("{}: {}", l.id.name(), l.value.to_css()))
        .collect()
}

fn accepts(decl: &str) {
    accepts_as(decl, decl);
}

/// `decl` parses and serializes canonically as `want`.
fn accepts_as(decl: &str, want: &str) {
    let got = longhands(decl);
    assert_eq!(got.len(), 1, "{decl}: {got:?}");
    assert_eq!(got[0], want, "{decl}");
}

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
fn font_properties() {
    for decl in [
        "font-family: \"Inter Var\", system-ui, sans-serif",
        "font-family: Inter",
        "font-size: 14px",
        "font-size: 120%",
        "font-size: 1.2em",
        "font-feature-settings: \"tnum\"",
        "font-feature-settings: \"tnum\" 1, \"liga\" off",
        "font-feature-settings: normal",
        "font-variation-settings: \"wght\" 500, \"wdth\" 80",
        "font-variation-settings: normal",
    ] {
        accepts(decl);
    }
    assert!(
        refusal("a { font-feature-settings: tnum; }")
            .message
            .contains("string")
    );
}

#[test]
fn shadow_transform_and_filter_lists() {
    accepts_as(
        "box-shadow: 0 2px 8px rgb(0 0 0 / 40%)",
        "box-shadow: 0px 2px 8px rgb(0 0 0 / 0.4)",
    );
    accepts_as(
        "box-shadow: inset 0 0 0 1px red, 0 1px 2px black",
        "box-shadow: inset 0px 0px 0px 1px rgb(255 0 0), 0px 1px 2px rgb(0 0 0)",
    );
    for decl in [
        "box-shadow: none",
        "transform: none",
        "transform: translate(10px, 20%)",
        "transform: translate(10px)",
        "transform: translateX(1px) translateY(2px)",
        "transform: scale(1.1)",
        "transform: scale(1, 2)",
        "transform: translate(1px, 2px) scale(2)",
        "filter: none",
        "filter: blur(4px)",
    ] {
        accepts(decl);
    }
    for bad in [
        "transform: rotate(45deg)",
        "filter: grayscale(1)",
        "box-shadow: 1px",
    ] {
        let err = refusal(&format!("a {{ {bad}; }}"));
        assert!(
            err.message.contains("translate")
                || err.message.contains("blur")
                || err.message.contains("shadow"),
            "{bad}: {err}"
        );
    }
}

#[test]
fn grid_properties_for_taffy() {
    for decl in [
        "grid-template-columns: 1fr 2fr",
        "grid-template-columns: repeat(3, 1fr)",
        "grid-template-columns: minmax(100px, 1fr) auto",
        "grid-template-rows: none",
        "grid-auto-rows: min-content",
        "grid-auto-columns: 40px",
        "grid-row-start: 2",
        "grid-column-end: span 2",
        "grid-row-start: auto",
    ] {
        accepts(decl);
    }
    assert!(
        refusal("a { grid-template-columns: [a] 1fr; }")
            .message
            .contains("line names")
    );
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
fn steps_one_jump_none_is_refused_naming_it() {
    // Given: review item 9; jump-none needs at least two steps.
    let err = refusal("a { transition-timing-function: steps(1, jump-none); }");
    assert!(err.message.contains("jump-none"), "{err}");
    accepts("transition-timing-function: steps(2, jump-none)");
}
