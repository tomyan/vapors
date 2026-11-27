//! `appearance` refused, and its computed value: `none` on a range, the
//! compat keywords, and `auto` for a `type` written in capitals
//! (`docs/appearance.md`).

use crate::support::{document, element, refusals, viewport};
use vapors_dom::Dom;

#[test]
fn appearance_none_on_a_range_is_refused_naming_what_chromium_keeps() {
    // Given: Chromium takes a range's track away and keeps its thumb, which
    // vapors draws as one part
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "range");
    doc.add_stylesheet("app.css", "input { display: block; appearance: none }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    let named = ["app.css:1:", "appearance", "range", "thumb"];
    assert!(named.iter().all(|n| errors[0].contains(n)), "{errors:?}");
}

#[test]
fn a_compatibility_keyword_is_refused_naming_its_sheet_and_line() {
    // Given: CSS UI 4's compat keywords, which vapors does not take
    let mut doc = document();

    // When
    let parsed = doc.add_stylesheet("app.css", "input {\n  appearance: checkbox }");

    // Then
    let error = parsed.expect_err("refused").to_string();
    assert!(
        error.contains("appearance") && error.contains("app.css:2:"),
        "{error}"
    );
}

#[test]
fn a_range_type_written_in_capitals_computes_appearance_auto() {
    // Given: `type=Range` with no `appearance` of the author's (`display:
    // block` only, as an inline-block input is laid out only as a flex
    // item); Chromium 157 computes `auto` (docs/appearance.md)
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "Range");
    doc.add_stylesheet("app.css", "input { display: block }")
        .expect("parses");

    // When
    doc.update(viewport(400.0, 200.0)).expect("updates");

    // Then
    let style = doc.computed_style(input).expect("styled");
    let appearance = style.get_by_name("appearance").expect("a property");
    assert_eq!(appearance.to_css(), "auto");
}
