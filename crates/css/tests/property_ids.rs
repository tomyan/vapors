//! Longhand ids worked out at compile time (`src/property/longhands.rs`):
//! `PropertyId::named` agrees with the table, and a name the table does
//! not list stops the build (a constant) or panics (a run-time call).

use vapors_css::property::{PROPERTIES, PropertyId, longhands};

#[test]
fn named_gives_every_longhand_its_table_index() {
    for (index, def) in PROPERTIES.iter().enumerate() {
        // When
        let id = PropertyId::named(def.name);

        // Then
        assert_eq!(id.index(), index, "{}", def.name);
    }
}

#[test]
#[should_panic(expected = "not a longhand in vapors-css's table")]
fn named_refuses_a_misspelt_name() {
    // When / Then
    let _ = PropertyId::named("colour");
}

#[test]
#[should_panic(expected = "not a longhand in vapors-css's table")]
fn named_refuses_a_shorthand() {
    // When / Then
    let _ = PropertyId::named("margin");
}

#[test]
fn constants_name_their_longhands() {
    // Given: the first, last and a border side of the table
    let cases = [
        (longhands::ALIGN_CONTENT, "align-content"),
        (longhands::BORDER_TOP_COLOR, "border-top-color"),
        (longhands::Z_INDEX, "z-index"),
    ];

    // Then
    for (id, name) in cases {
        assert_eq!(id.name(), name);
    }
}
