use super::*;

#[test]
fn every_css_longhand_has_exactly_one_treatment() {
    // Given
    let css: Vec<&str> = PropertyId::all().map(PropertyId::name).collect();

    // When
    let ours: Vec<&str> = TABLE.iter().map(|(name, _)| *name).collect();

    // Then: the same names, in the same (sorted) order, none twice
    assert_eq!(
        ours, css,
        "vapors-elements' honoured table and vapors-css's property table disagree; add or remove entries in crates/elements/src/style/honoured.rs"
    );
}
