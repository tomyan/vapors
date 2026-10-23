//! `bolder` and `lighter` compute to a number from the inherited weight
//! (CSS Fonts 4 section 2.2.1's table).

mod support;

use support::css_at;

#[test]
fn bolder_steps_up_from_the_inherited_weight() {
    for (inherited, bolder) in [
        ("100", "400"),
        ("400", "700"),
        ("normal", "700"),
        ("700", "900"),
        ("bold", "900"),
        ("900", "900"),
    ] {
        // When
        let css = format!(".p {{ font-weight: {inherited}; }} .c {{ font-weight: bolder; }}");

        // Then
        assert_eq!(
            css_at(&css, &["div.p", "span.c"], "font-weight"),
            bolder,
            "from {inherited}"
        );
    }
}

#[test]
fn lighter_steps_down_from_the_inherited_weight() {
    for (inherited, lighter) in [
        ("50", "50"),
        ("400", "100"),
        ("600", "400"),
        ("700", "400"),
        ("800", "700"),
    ] {
        // When
        let css = format!(".p {{ font-weight: {inherited}; }} .c {{ font-weight: lighter; }}");

        // Then
        assert_eq!(
            css_at(&css, &["div.p", "span.c"], "font-weight"),
            lighter,
            "from {inherited}"
        );
    }
}

#[test]
fn bolder_at_the_root_steps_up_from_the_initial_weight() {
    // Then
    assert_eq!(
        css_at("div { font-weight: bolder; }", &["div"], "font-weight"),
        "700"
    );
}
