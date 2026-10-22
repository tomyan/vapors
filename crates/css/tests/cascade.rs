//! The cascade entry points (`src/cascade/mod.rs`).

mod support;

use support::sheet;
use vapors_css::cascade::Cascade;

#[test]
fn keyframes_are_reachable_by_name() {
    let cascade = Cascade::new().author(sheet(
        "@keyframes pulse { from { opacity: 0; } to { opacity: 1; } }",
    ));
    assert_eq!(cascade.keyframes("pulse").map(|k| k.stops.len()), Some(2));
    assert!(cascade.keyframes("nope").is_none());
}
