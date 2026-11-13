//! `class`, `class:` and `style` on a component tag pass the tag's
//! `StyleKey` as the component's `anchor` prop (css-for-3d.md, C1); a
//! component in the same directory that declares no `anchor` prop has
//! no element to put them on, and the tag is refused at the attribute.

#[path = "common/build.rs"]
mod build;
#[path = "common/props.rs"]
mod props;

use props::refusals;

#[test]
fn a_class_on_a_component_without_an_anchor_is_refused_naming_the_tag() {
    // When
    let errors = refusals("class_no_anchor", "<Plain title=\"a\" class=\"x\" />");

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert!(error.contains("page.wisp:5:18"), "{error}");
    assert!(
        error.contains("`<Plain>` takes no `class`: it has no anchor element"),
        "{error}"
    );
    assert!(error.contains("plain.wisp"), "{error}");
}

#[test]
fn a_class_directive_and_a_style_are_refused_alike() {
    // When
    let errors = refusals(
        "directive_no_anchor",
        "<Plain title=\"a\" class:on={true} style=\"color: red\" />",
    );

    // Then: one refusal for each, at its attribute
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors[0].contains("page.wisp:5:18") && errors[0].contains("takes no `class:on`"),
        "{}",
        errors[0]
    );
    assert!(
        errors[1].contains("page.wisp:5:34") && errors[1].contains("takes no `style`"),
        "{}",
        errors[1]
    );
}

#[test]
fn a_wisp_component_declaring_an_anchor_prop_is_still_refused() {
    // Given: a generated component has no anchor element to make from
    // the key, whatever props it declares
    let (src, out) = build::dirs("declared_anchor");
    build::write(&src, "card.wisp", CARD_WITH_ANCHOR);
    let page = "<script>\nuse crate::card::{Card, CardProps};\n</script>\n\n<Card class=\"x\" />";
    build::write(&src, "page.wisp", page);

    // When
    let errors = props::shown(vapors_compiler::build::compile_dir_into(&src, &out));

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("`<Card>` takes no `class`"),
        "{}",
        errors[0]
    );
}

const CARD_WITH_ANCHOR: &str = "<script>\n#[prop]\nlet anchor: Option<&'static vapors_dom::StyleKey> = None;\n</script>\n\n<p>card</p>";
