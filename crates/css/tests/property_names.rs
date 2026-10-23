//! Property name checks (`src/property/names.rs`): unknown names, sumi extras, unexpanded shorthands.

mod support;

use support::refusal;

#[test]
fn unknown_property_is_refused_naming_it_and_the_line() {
    // When
    let err = refusal(".x {\n  colour: red;\n}");

    // Then
    assert_eq!((err.position.line, err.position.column), (2, 3));
    assert!(err.message.contains("colour"), "{err}");
    assert!(err.message.contains("unknown property"), "{err}");
}

#[test]
fn sumi_terminal_boolean_properties_are_refused_by_name() {
    for (name, hint) in [
        ("bold", "font-weight"),
        ("dim", "opacity"),
        ("italic", "font-style"),
        ("underline", "text-decoration"),
        ("strikethrough", "text-decoration"),
        ("inverse", "color"),
        ("border-title", "element"),
    ] {
        // When
        let err = refusal(&format!(".x {{ {name}: true; }}"));

        // Then
        assert!(err.message.contains(name), "{name}: {err}");
        assert!(err.message.contains("sumi"), "{name}: {err}");
        assert!(err.message.contains(hint), "{name}: {err}");
    }
}

#[test]
fn known_but_unsupported_shorthands_are_refused_by_name() {
    // Given: review item 18.
    for (decl, hint) in [
        ("font: 12px serif", "font-size"),
        ("background: red", "background-color"),
    ] {
        let err = refusal(&format!("a {{ {decl}; }}"));
        assert!(
            err.message.contains("not supported") && err.message.contains(hint),
            "{decl}: {err}"
        );
    }
}
