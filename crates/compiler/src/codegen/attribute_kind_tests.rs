//! Each attribute shape has one kind, tested in the emitter's order.

use super::{AttributeKind, fixed_attributes, kind};
use crate::ast::{AttrValue, Attribute, Attributes, Expr};

fn literal(name: &str, value: &str) -> Attribute {
    Attribute {
        name: name.to_owned(),
        value: AttrValue::Literal(value.to_owned()),
        offset: 0,
    }
}

fn expression(name: &str) -> Attribute {
    Attribute {
        name: name.to_owned(),
        value: AttrValue::Expr(Expr {
            code: "x".to_owned(),
            offset: 0,
        }),
        offset: 0,
    }
}

#[test]
fn each_shape_has_one_kind() {
    // Given / Then
    assert_eq!(kind(&expression("bind:this")), AttributeKind::BindThis);
    assert_eq!(
        kind(&expression("class:open")),
        AttributeKind::ClassDirective("open")
    );
    assert_eq!(kind(&literal("Class", "a b")), AttributeKind::Class);
    assert_eq!(kind(&literal("STYLE", "x")), AttributeKind::Style);
    assert_eq!(
        kind(&literal(crate::SCOPE_ATTRIBUTE, "s")),
        AttributeKind::Scope
    );
    assert_eq!(
        kind(&expression("onclick")),
        AttributeKind::Listener("click")
    );
    assert_eq!(
        kind(&expression("bind:value")),
        AttributeKind::Binding("value")
    );
    assert_eq!(kind(&literal("title", "t")), AttributeKind::Literal("t"));
    assert_eq!(kind(&literal("one", "1")), AttributeKind::Listener("e"));
    assert!(matches!(
        kind(&expression("id")),
        AttributeKind::Expression(_)
    ));
}

#[test]
fn the_key_fixes_the_literals_and_nothing_else() {
    // Given
    let attributes = Attributes(vec![
        literal("type", "text"),
        literal("class", "a"),
        expression("id"),
        expression("onclick"),
        literal("aria-label", "Go"),
        expression("bind:value"),
    ]);

    // Then
    assert_eq!(
        fixed_attributes(&attributes),
        [("type", "text"), ("aria-label", "Go")]
    );
}
