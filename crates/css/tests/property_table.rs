//! The property table (`src/property/table.rs`): every entry parses and is sorted.

mod support;

use support::sheet;
use vapors_css::property::{PROPERTIES, PropertyId, is_shorthand};

#[test]
fn every_initial_value_parses_under_its_own_syntax() {
    for def in PROPERTIES.iter() {
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
