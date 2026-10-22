//! At-rules (`src/parse/at_rules.rs`). Translated from sumi `style_atrule_test.go` and `media_test.go`; unknown at-rules are refused rather than dropped.

mod support;

use support::{prop, refusal, sheet};

#[test]
fn media_block_rules_carry_query() {
    // Given
    let input = "\n.card { color: red; }\n@media (min-width: 600px) {\n\t.card { border-style: solid; }\n}\n.other { color: blue; }\n";

    // When
    let s = sheet(input);

    // Then
    assert_eq!(s.rules.len(), 3, "flat, source order");
    assert_eq!(s.rules[0].media_text(), None);
    assert_eq!(s.rules[2].media_text(), None);
    assert_eq!(
        s.rules[1].media_text().as_deref(),
        Some("(min-width: 600px)")
    );
    assert_eq!(prop(&s.rules[1], "border-style"), "solid");
}

#[test]
fn media_block_with_multiple_rules() {
    // When
    let s = sheet("@media (max-width: 400px) { .a { color: red; } .b { color: blue; } }");

    // Then
    assert_eq!(s.rules.len(), 2);
    for r in &s.rules {
        assert_eq!(r.media_text().as_deref(), Some("(max-width: 400px)"));
    }
}

#[test]
fn media_block_with_keyframes_untouched() {
    // When
    let s = sheet(
        "@media (min-width: 1px) { .a { color: red; } } @keyframes pulse { from { color: red; } to { color: blue; } }",
    );

    // Then
    assert_eq!((s.rules.len(), s.keyframes.len()), (1, 1));
}

#[test]
fn unterminated_media_block_is_refused() {
    // When
    let err = refusal("@media (min-width: 1px) { .a { color: red; }");

    // Then: the refusal points at the brace that was never closed.
    assert_eq!(err.position.column, 25);
    assert!(err.message.contains("@media"), "{err}");
}

#[test]
fn container_and_supports_blocks_tag_their_rules() {
    // When
    let s = sheet(
        "@container (min-width: 40px) { .label { color: red; } } @supports (display: grid) { .g { display: grid; } }",
    );

    // Then
    assert_eq!(
        s.rules[0].container_text().as_deref(),
        Some("(min-width: 40px)")
    );
    assert_eq!(
        s.rules[1].supports_text().as_deref(),
        Some("(display: grid)")
    );
}

#[test]
fn nested_conditional_blocks_stack_their_conditions() {
    // When
    let s = sheet("@media (min-width: 1px) { @supports (display: grid) { .a { color: red; } } }");

    // Then
    assert_eq!(s.rules[0].media_text().as_deref(), Some("(min-width: 1px)"));
    assert_eq!(
        s.rules[0].supports_text().as_deref(),
        Some("(display: grid)")
    );
}

#[test]
fn unknown_at_rule_is_refused_naming_it() {
    // Given: sumi dropped @font-face silently.
    let input = "\n@font-face {\n\tfont-family: \"Whatever\";\n\tsrc: url(x.woff2);\n}\n.card { color: red; }\n";

    // When
    let err = refusal(input);

    // Then
    assert_eq!((err.position.line, err.position.column), (2, 1));
    assert!(err.message.contains("@font-face"), "{err}");
}

#[test]
fn layer_at_rule_is_refused_as_unsupported() {
    // Given: sumi has no @layer either; it dropped the block silently.
    let input = "@layer base {\n\t.a { color: blue; }\n}\nspan { color: cyan; }";

    // When
    let err = refusal(input);

    // Then
    assert!(err.message.contains("@layer"), "{err}");
}

#[test]
fn statement_at_rule_is_refused() {
    // When
    let err = refusal("@import \"theme.css\";\n.x { color: yellow; }");

    // Then
    assert!(err.message.contains("@import"), "{err}");
}
