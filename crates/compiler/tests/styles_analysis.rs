//! `vapors_compiler::styles::analyse` and its report: what each
//! template element fixes, its rules and winners over the user-agent
//! sheet and the component's own scoped sheet, and values taken from
//! a parent resolved through the template.

use vapors_compiler::styles::{Analysis, Resolved, analyse, report};
use vapors_css::analysis::{Constancy, Match, Winner};
use vapors_css::property::PropertyId;

fn analysed(text: &str) -> Analysis {
    let component = vapors_compiler::parse_str("card.wisp", text).unwrap_or_else(|e| panic!("{e}"));
    analyse(&component, &[]).unwrap_or_else(|e| panic!("{e}"))
}

fn id(name: &str) -> usize {
    PropertyId::from_name(name).expect("longhand").index()
}

#[test]
fn an_elements_facts_come_from_its_attributes_as_its_key_does() {
    // Given
    let analysis = analysed(
        "<script>\nlet o: Signal<bool> = Signal::new(false);\n</script>\n<style>\n.card { color: red }\n</style>\n<div class=\"card wide\" class:open={o} id={x} title=\"t\" onclick={f} bind:value={v} style=\"width: 2px\"></div>",
    );

    // Then
    let facts = &analysis.elements[0].facts;
    assert_eq!(
        (facts.tag.as_str(), facts.scope.as_deref()),
        ("div", Some("card.wisp"))
    );
    assert_eq!(facts.classes, ["card", "wide"]);
    assert_eq!(facts.toggles, ["open"]);
    assert_eq!(facts.expression_attributes, ["id"]);
    assert_eq!(
        facts.literal_attributes,
        [("title".to_owned(), "t".to_owned())]
    );
    assert_eq!(facts.style.as_deref(), Some("width: 2px"));
    assert_eq!(facts.inline.len(), 1);
    assert_eq!(
        (analysis.elements[0].line, analysis.elements[0].col),
        (7, 1)
    );
}

#[test]
fn the_own_sheet_is_scoped_and_judged_with_the_user_agent_sheet() {
    // Given: `.card` scoped is always for the keyed <div>; `.open` is no
    // candidate: the key has no such class or directive
    let analysis = analysed(
        "<style>\n.card { color: red }\n.open { color: blue }\n</style>\n<div class=\"card\"></div>",
    );

    // Then
    let element = &analysis.elements[0];
    let own: Vec<&Match> = (element.winners.rules.iter())
        .filter(|rule| rule.sheet == 1)
        .map(|rule| &rule.verdict)
        .collect();
    assert_eq!(own, [&Match::Always]);
    assert!(matches!(
        element.winners.longhands[id("color")],
        Winner::Constant {
            value: Constancy::Constant,
            ..
        }
    ));
    assert_eq!(element.resolved[id("color")], Resolved::Constant);
}

#[test]
fn a_value_from_the_parent_is_constant_through_a_template_parent_and_variable_at_a_root() {
    // Given: color inherits; the span's parent declares it, the root's parent is the mount site's
    let analysis = analysed(
        "<style>\n.card { color: red }\n</style>\n<div class=\"card\"><span></span></div>\n<p></p>",
    );

    // Then
    let [div, span, p] = analysis.elements.as_slice() else {
        panic!("three elements, got {}", analysis.elements.len());
    };
    assert_eq!(div.resolved[id("color")], Resolved::Constant);
    assert_eq!(span.resolved[id("color")], Resolved::Constant);
    assert_eq!(
        p.resolved[id("color")],
        Resolved::Variable("the mount site's parent")
    );
    assert_eq!(p.resolved[id("width")], Resolved::Constant);
}

#[test]
fn rows_of_a_for_keep_the_enclosing_parent_and_snippets_do_not() {
    // Given
    let analysis = analysed(
        "<script>\nlet items: Signal<Vec<i32>> = Signal::new(vec![]);\n</script>\n<style>\n.card { color: red }\n</style>\n<div class=\"card\">{for item in items}<span></span>{/for}</div>\n{snippet row()}<em></em>{/snippet}\n<p>{render row()}</p>",
    );

    // Then
    let by_tag = |tag: &str| {
        (analysis.elements.iter())
            .find(|element| element.facts.tag == tag)
            .unwrap_or_else(|| panic!("<{tag}>"))
    };
    assert_eq!(by_tag("span").resolved[id("color")], Resolved::Constant);
    assert_eq!(
        by_tag("em").resolved[id("color")],
        Resolved::Variable("placed by a snippet or a component")
    );
}

#[test]
fn both_branches_of_an_if_keep_the_enclosing_parent_and_a_component_tags_children_do_not() {
    // Given
    let analysis = analysed(
        "<script>\nlet c: Signal<bool> = Signal::new(false);\n</script>\n<style>\n.card { color: red }\n</style>\n<div class=\"card\">{if c}<span></span>{else}<em></em>{/if}<Child><b></b></Child></div>",
    );

    // Then
    let by_tag = |tag: &str| {
        (analysis.elements.iter())
            .find(|element| element.facts.tag == tag)
            .unwrap_or_else(|| panic!("<{tag}>"))
    };
    assert_eq!(by_tag("span").resolved[id("color")], Resolved::Constant);
    assert_eq!(by_tag("em").resolved[id("color")], Resolved::Constant);
    assert_eq!(
        by_tag("b").resolved[id("color")],
        Resolved::Variable("placed by a snippet or a component")
    );
}

#[test]
fn a_child_of_a_variable_parent_names_the_parent_and_custom_properties_resolve_through_parents() {
    // Given: `.card`'s colour is variable; `--x` is constant on it
    let analysis = analysed(
        "<script>\nlet o: Signal<bool> = Signal::new(false);\n</script>\n<style>\n.card { --x: red; color: blue }\n.open { color: red }\n</style>\n<div class=\"card\" class:open={o}><span></span></div>",
    );

    // Then
    let [div, span] = analysis.elements.as_slice() else {
        panic!("two elements");
    };
    assert_eq!(
        div.resolved[id("color")],
        Resolved::Variable("conditional rules")
    );
    assert_eq!(
        span.resolved[id("color")],
        Resolved::Variable("the parent's value is variable")
    );
    assert_eq!(div.resolved_custom["--x"], Resolved::Constant);
    assert!(span.resolved_custom.is_empty());
}

#[test]
fn a_key_refusal_is_the_analysis_refusal_too() {
    // Given: a directive that is also a literal class
    let component = vapors_compiler::parse_str(
        "card.wisp",
        "<script>\nlet o: Signal<bool> = Signal::new(false);\n</script>\n<div class=\"open\" class:open={o}></div>",
    )
    .expect("parses");

    // When
    let error = analyse(&component, &[]).expect_err("refused");

    // Then
    assert!(
        error.message.contains("`class:open` toggles a class"),
        "{error}"
    );
}

#[test]
fn a_literal_style_that_does_not_parse_is_refused_at_its_attribute() {
    // Given
    let component =
        vapors_compiler::parse_str("card.wisp", "<div style=\"colr: red\"></div>").expect("parses");

    // When
    let error = analyse(&component, &[]).expect_err("refused");

    // Then
    assert_eq!((error.line, error.col), (1, 6), "{error}");
    assert!(error.message.contains("style attribute"), "{error}");
}

#[test]
fn the_report_names_each_element_its_rules_and_its_declared_properties() {
    // Given
    let analysis = analysed(
        "<script>\nlet o: Signal<bool> = Signal::new(false);\n</script>\n<style>\n.card { color: red; width: 2em }\n.open { color: blue }\n</style>\n<div class=\"card\" class:open={o} id={o}></div>",
    );

    // When
    let text = report(&analysis, "card.wisp");

    // Then
    for line in [
        "sheets: ua, card.wisp; other sheets of the crate: not read",
        "card.wisp:8:1 <div>",
        "  inputs: id",
        "    always      card.wisp:0 .card",
        "    conditional card.wisp:1 .open (class:open)",
        "  color: variable winner: card.wisp:0:0 unless card.wisp:1 matches; computed variable (conditional rules)",
        "  width: constant winner card.wisp:0:1 with a variable value; computed variable (the value reads its context)",
        "  shareable: yes",
    ] {
        assert!(text.contains(line), "missing {line:?} in:\n{text}");
    }
}
