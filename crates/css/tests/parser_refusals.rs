//! What vapors refuses that sumi accepted: unknown properties, sumi's
//! terminal extras, the `cell` unit and unknown at-rules. vapors-specific
//! tests; sumi's "graceful drop" has no equivalent here.

mod support;

use support::{refusal, sheet};

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
fn cell_unit_is_refused_naming_the_unit() {
    // When
    let err = refusal(".x { width: 20cell; }");

    // Then
    assert_eq!(err.position.column, 13);
    assert!(
        err.message.contains("cell") && err.message.contains("px"),
        "{err}"
    );
}

#[test]
fn vendor_prefixed_properties_on_the_allowlist_are_dropped() {
    // When
    let s = sheet(".x { -webkit-font-smoothing: antialiased; color: red; }");

    // Then
    assert_eq!(s.rules[0].declarations.len(), 1);
    assert_eq!(s.rules[0].declarations[0].name, "color");
}

#[test]
fn vendor_prefixed_properties_off_the_allowlist_are_refused() {
    // When
    let err = refusal(".x { -moz-appearance: none; }");

    // Then
    assert!(err.message.contains("-moz-appearance"), "{err}");
}

#[test]
fn unknown_at_rule_is_refused_naming_it() {
    // Given: sumi dropped @font-face silently.
    let input = "\n@font-face {\n\tfont-family: \"Whatever\";\n\tsrc: url(x.woff2);\n}\n.card { color: red; }\n";

    // When
    let err = refusal(input);

    // Then
    assert_eq!((err.position.line, err.position.column), (2, 1));
    assert!(err.message.contains("@font-face"), "{err}");
}

#[test]
fn layer_at_rule_is_refused_as_unsupported() {
    // Given: sumi has no @layer either; it dropped the block silently.
    let input = "@layer base {\n\t.a { color: blue; }\n}\nspan { color: cyan; }";

    // When
    let err = refusal(input);

    // Then
    assert!(err.message.contains("@layer"), "{err}");
}

#[test]
fn statement_at_rule_is_refused() {
    // When
    let err = refusal("@import \"theme.css\";\n.x { color: yellow; }");

    // Then
    assert!(err.message.contains("@import"), "{err}");
}
