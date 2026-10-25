//! The fixtures' `data-scroll`, applied as the oracle page applies it.

use vapors_elements::Document;

/// Scrolls each element with `data-scroll="x y"` there, as the oracle
/// page does; whether any has the attribute.
pub fn as_the_fixture_says(doc: &mut Document, name: &str) -> bool {
    let mut any = false;
    for node in doc.elements() {
        let Some(value) = doc.attribute(node, "data-scroll").map(str::to_owned) else {
            continue;
        };
        let offset: Vec<f32> = value
            .split(' ')
            .map(|v| v.parse().expect("a number"))
            .collect();
        let [x, y] = offset[..] else {
            panic!("{name}: data-scroll=\"{value}\" is not \"x y\"");
        };
        assert!(
            doc.scroll_to(node, [x, y]) || doc.scroll_offset(node).is_some(),
            "{name}: data-scroll on an element that does not scroll"
        );
        any = true;
    }
    any
}
