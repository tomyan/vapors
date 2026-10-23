//! Hit testing a wrapped inline element: each of its fragments takes
//! the pointer, and the gap between them, inside the line boxes but
//! outside the content areas, is the block's.

mod support;

use support::{document, element, text, viewport};

#[test]
fn a_wrapped_span_is_hit_on_each_line_and_not_between_them() {
    // Given: 40 px lines, 21 px content areas, a span starting both lines
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "p", "");
    let span = element(&mut doc, p, "span", "");
    text(&mut doc, span, "tap these words here please");
    text(&mut doc, p, " ok");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } p { margin: 0; width: 120px; line-height: 40px; }",
    )
    .expect("parses");
    let painted = doc.update(viewport(400.0, 200.0)).expect("updates");

    // When: on the first line, on the second, and in the gap between
    let first = painted.hit([5.0, 20.0]);
    let second = painted.hit([5.0, 60.0]);
    let between = painted.hit([5.0, 40.0]);

    // Then
    assert_eq!((first, second), (Some(span), Some(span)));
    assert_eq!(between, Some(p));
}
