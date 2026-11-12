//! A rule under `@container` keeps the elements it can match from
//! sharing a computed style (analysis PR 2): its condition reads the
//! nearest container's laid-out size, which no share-key input
//! carries. Containers have no size yet (`ElementRef::container_size`
//! is `None`), so two rows in containers of different sizes cannot be
//! built; the count of unshareable rows pins the guard until they can.

mod support;

use support::{document, viewport};
use vapors_dom::{Dom, StyleKey};

const SCOPE: &str = "tests/style_sharing_container.wisp";

static ROW: StyleKey = StyleKey {
    site: "tests/style_sharing_container.wisp:1",
    scope: Some(SCOPE),
    tag: "div",
    classes: &["row"],
    toggles: &[],
    style: None,
};

#[test]
fn rows_a_container_rule_can_match_style_each_for_itself() {
    // Given
    let mut doc = document();
    doc.add_style(
        SCOPE,
        ".row { color: rgb(1 2 3); } @container (min-width: 100px) { .row { color: rgb(4 5 6); } }",
    );
    let body = doc.body();
    for _ in 0..3 {
        let row = doc.create_keyed_element(&ROW);
        doc.append(body, row);
    }

    // When
    doc.update(viewport(200.0, 100.0)).expect("updates");

    // Then
    let counts = doc.update_counts();
    assert_eq!(
        (
            counts.styles_unshareable,
            counts.styles_shared,
            counts.styles_computed
        ),
        (3, 0, 0)
    );
}
