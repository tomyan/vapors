use super::*;

#[test]
fn a_fixture_splits_into_style_template_and_viewport() {
    // Given
    let text = "<style>\n/* viewport 320x200 */\ndiv { width: 1px; }\n</style>\n<div></div>\n";

    // When
    let source = split("x", text).expect("splits");

    // Then
    assert_eq!(source.viewport, [320, 200]);
    assert!((source.scale - 1.0).abs() < f64::EPSILON);
    assert!(source.style.contains("div { width: 1px; }"));
    assert_eq!(source.template, "\n<div></div>\n");
}

#[test]
fn a_fixture_without_a_viewport_comment_is_refused() {
    // Given
    let text = "<style>div {}</style><div></div>";

    // When
    let error = split("x", text).expect_err("refused");

    // Then
    assert!(error.contains("/* viewport WxH */"), "{error}");
}

#[test]
fn a_viewport_may_name_a_device_scale() {
    // Given
    let text = "<style>/* viewport 640x400@2.625 */</style><div></div>";

    // When
    let source = split("x", text).expect("splits");

    // Then
    assert_eq!(source.viewport, [640, 400]);
    assert!((source.scale - 2.625).abs() < f64::EPSILON);
}

#[test]
fn a_viewport_with_a_scale_that_is_not_a_positive_number_is_refused() {
    for scale in ["two", "0", "-1", ""] {
        // Given
        let text = format!("<style>/* viewport 640x400@{scale} */</style><div></div>");

        // When
        let error = split("x", &text).expect_err("refused");

        // Then
        assert!(error.contains("/* viewport WxH@S */"), "{scale}: {error}");
    }
}

#[test]
fn a_fixture_with_a_script_is_refused() {
    // Given
    let text = "<script>let a = 1;</script><style>/* viewport 1x1 */</style><div></div>";

    // When
    let error = split("x", text).expect_err("refused");

    // Then
    assert!(error.contains("no <script>"), "{error}");
}

#[test]
fn boxes_are_read_from_the_dumped_dom() {
    // Given
    let dom = "<html><body></body><pre id=\"vapors-boxes\">[{\"tag\":\"a&amp;b\"}]</pre></html>";

    // When
    let boxes = boxes_from_dom(dom).expect("found");

    // Then
    assert_eq!(boxes, "[{\"tag\":\"a&b\"}]");
}

#[test]
fn a_dump_without_boxes_is_refused() {
    // Given
    let dom = "<html><body></body></html>";

    // When
    let error = boxes_from_dom(dom).expect_err("refused");

    // Then
    assert!(error.contains("document.fonts.ready"), "{error}");
}

#[test]
fn a_page_that_reports_unloaded_fonts_is_refused() {
    // Given
    let dom = "<pre id=\"vapors-boxes\">{\"error\":\"fonts did not load: Geist (error)\"}</pre>";

    // When
    let error = boxes_from_dom(dom).expect_err("refused");

    // Then
    assert!(error.contains("Geist (error)"), "{error}");
}
