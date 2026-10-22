//! The property table: every property has a name, an inherited flag, an
//! initial value that parses under its own syntax, and that syntax.
//! vapors-specific: sumi validated no values.

mod support;

use support::{refusal, sheet};
use vapors_css::property::{PROPERTIES, PropertyId, is_shorthand};

#[test]
fn every_initial_value_parses_under_its_own_syntax() {
    for def in PROPERTIES {
        // When
        let s = sheet(&format!("a {{ {}: {}; }}", def.name, def.initial));

        // Then
        let longhands = &s.rules[0].declarations[0].longhands;
        assert_eq!(longhands.len(), 1, "{} is a longhand", def.name);
        assert_eq!(longhands[0].value.to_css(), def.initial, "{}", def.name);
    }
}

#[test]
fn table_is_sorted_and_unique() {
    let names: Vec<&str> = PROPERTIES.iter().map(|d| d.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted);
}

#[test]
fn ids_round_trip_through_names() {
    for (index, def) in PROPERTIES.iter().enumerate() {
        let id = PropertyId::from_name(def.name).unwrap_or_else(|| panic!("{}", def.name));
        assert_eq!(id.name(), def.name);
        assert_eq!(id.index(), index);
    }
    assert_eq!(
        PropertyId::from_name("margin"),
        None,
        "shorthands have no id"
    );
    assert!(is_shorthand("margin"));
}

#[test]
fn inherited_flags_follow_the_specification() {
    for (name, inherited) in [
        ("color", true),
        ("font-family", true),
        ("font-size", true),
        ("line-height", true),
        ("letter-spacing", true),
        ("text-align", true),
        ("white-space", true),
        ("cursor", true),
        ("visibility", true),
        ("user-select", false),
        ("display", false),
        ("width", false),
        ("background-color", false),
        ("opacity", false),
        ("transition-duration", false),
    ] {
        let id = PropertyId::from_name(name).expect(name);
        assert_eq!(id.inherited(), inherited, "{name}");
    }
}

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
