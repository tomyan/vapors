//! Declaration parsing (`src/property/declaration.rs`): grammar refusals, CSS-wide keywords, pending values.

mod support;

use support::{refusal, sheet};

#[test]
fn value_that_fails_its_syntax_is_refused_naming_property_and_position() {
    // When
    let err = refusal(".x {\n  width: red;\n}");

    // Then
    assert_eq!((err.position.line, err.position.column), (2, 10));
    assert!(err.message.contains("width"), "{err}");
    assert!(
        err.message.contains("auto") && err.message.contains("length"),
        "{err}"
    );
}

#[test]
fn sumi_terminal_values_are_refused_with_a_hint() {
    for (decl, hint) in [
        ("opacity: dim", "number"),
        ("border: rounded", "border-radius"),
        ("border-style: single", "solid"),
    ] {
        let err = refusal(&format!(".x {{ {decl}; }}"));
        assert!(err.message.contains("sumi"), "{decl}: {err}");
        assert!(err.message.contains(hint), "{decl}: {err}");
    }
}

#[test]
fn css_wide_keywords_are_accepted_for_every_property() {
    for kw in ["inherit", "initial", "unset"] {
        let s = sheet(&format!("a {{ width: {kw}; color: {kw}; margin: {kw}; }}"));
        for d in &s.rules[0].declarations {
            for l in &d.longhands {
                assert_eq!(l.value.to_css(), kw, "{}", d.name);
            }
        }
    }
}

#[test]
fn declarations_with_var_stay_pending_until_computed() {
    // When
    let s = sheet("a { width: var(--w); }");

    // Then
    let l = &s.rules[0].declarations[0].longhands[0];
    assert!(l.value.is_pending(), "{:?}", l.value);
}
