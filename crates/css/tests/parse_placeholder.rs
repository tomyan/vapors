//! `::placeholder` (`src/parse/placeholder.rs`): only at the end of a
//! selector and only with `color`, which sets the originating element's
//! `-vapors-placeholder-color` (`docs/placeholder.md`).

use crate::support;

use support::{prop, refusal, sheet};
use vapors_css::property::longhands::PLACEHOLDER_COLOR;

const LOWERED: &str = "-vapors-placeholder-color";

#[test]
fn a_placeholder_colour_is_the_originating_elements_placeholder_colour() {
    // When
    let s = sheet(".search::placeholder { color: #6b7280 !important; }");

    // Then: one rule on `.search`, its one declaration the lowered longhand
    let [rule] = &s.rules[..] else {
        panic!("one rule: {:?}", s.rules);
    };
    assert_eq!(rule.selector_text(), ".search");
    assert_eq!(rule.declarations.len(), 1);
    let declaration = &rule.declarations[0];
    assert_eq!(declaration.name, LOWERED);
    assert!(declaration.important);
    assert_eq!(declaration.longhands[0].id, PLACEHOLDER_COLOR);
    assert_eq!(prop(rule, LOWERED), "#6b7280");
}

#[test]
fn a_bare_or_descendant_placeholder_originates_in_any_element() {
    for (text, originating) in [
        ("::placeholder", "*"),
        ("form ::placeholder", "form *"),
        ("form > ::placeholder", "form > *"),
        ("INPUT::PLACEHOLDER", "INPUT"),
    ] {
        // When
        let s = sheet(&format!("{text} {{ color: red; }}"));

        // Then
        assert_eq!(s.rules[0].selector_text(), originating, "{text}");
        assert_eq!(prop(&s.rules[0], LOWERED), "red", "{text}");
    }
}

#[test]
fn a_list_lowers_only_its_placeholder_selectors() {
    // When
    let s = sheet("input::placeholder, .hint { color: red; }");

    // Then
    assert_eq!(prop(&s.rules[0], LOWERED), "red");
    assert_eq!(prop(&s.rules[0], "color"), "");
    assert_eq!(s.rules[1].selector_text(), ".hint");
    assert_eq!(prop(&s.rules[1], "color"), "red");
}

#[test]
fn a_placeholder_declaration_other_than_colour_is_refused_naming_it() {
    for declaration in ["opacity: 0.5", "font-size: 12px", "--tone: red"] {
        // When
        let err = refusal(&format!("input::placeholder {{ {declaration}; }}"));

        // Then
        let name = declaration.split(':').next().expect("a name");
        assert!(err.message.contains(&format!("`{name}`")), "{err}");
        assert!(err.message.contains("only `color`"), "{err}");
    }
}

#[test]
fn a_css_wide_keyword_as_the_placeholder_colour_is_refused() {
    for keyword in ["inherit", "initial", "unset"] {
        // When
        let err = refusal(&format!("input::placeholder {{ color: {keyword}; }}"));

        // Then
        assert!(err.message.contains(keyword), "{err}");
        assert!(err.message.contains("write a colour"), "{err}");
    }
}

#[test]
fn a_placeholder_not_at_the_end_of_its_selector_is_refused() {
    for text in ["input::placeholder:hover", "::placeholder .x", "a::before"] {
        // When
        let err = refusal(&format!("{text} {{ color: red; }}"));

        // Then
        assert!(err.message.contains("pseudo-element"), "{text}: {err}");
        assert!(err.message.contains("::placeholder"), "{text}: {err}");
    }
}

#[test]
fn the_lowered_longhand_written_by_an_author_is_refused() {
    // When
    let err = refusal(&format!("input {{ {LOWERED}: red; }}"));

    // Then
    assert!(err.message.contains("::placeholder { color"), "{err}");
}
