//! A build without vapors-elements' `grid` feature (touché's page, for
//! its size) refuses `display: grid`, `inline-grid` and the grid
//! properties, naming the feature, rather than lay grid out as anything
//! else. Runs under `cargo test -p vapors-elements --no-default-features`
//! (see `style/grid_gate.rs` for why ci does not run it).
#![cfg(not(feature = "grid"))]

mod support;

use support::{document, element, refusals};

/// The refusals for an element of class `box` styled with `css`.
fn refused(css: &str) -> Vec<String> {
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "box");
    doc.add_stylesheet("app.css", css).expect("parses");
    refusals(&mut doc)
}

#[test]
fn display_grid_is_refused_naming_the_feature() {
    // inline-grid as a flex item, which blockifies it to grid
    let blockified = "body { display: flex; } .box { display: inline-grid; }";
    for (css, written) in [
        (".box { display: grid; }", "`display: grid`"),
        (blockified, "`display: inline-grid`"),
    ] {
        let errors = refused(css);
        assert!(
            errors
                .iter()
                .any(|e| e.contains(written) && e.contains("`grid` feature")),
            "{css}: {errors:?}"
        );
    }
}

#[test]
fn a_grid_property_is_refused_naming_the_feature() {
    let errors = refused(".box { grid-template-columns: 1fr 1fr; }");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("grid-template-columns") && e.contains("`grid` feature")),
        "{errors:?}"
    );
}

#[test]
fn justify_self_is_refused_as_grid_only() {
    let errors = refused(".box { justify-self: center; }");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("justify-self") && e.contains("grid-only")),
        "{errors:?}"
    );
}
