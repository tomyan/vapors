//! Which sheet each winning declaration came from (`ComputedStyle::
//! declared_in`), so a caller can name the sheet, line and column.

mod support;

#[test]
fn the_winning_declaration_names_its_sheet_by_the_order_sheets_were_added() {
    // Given: a user-agent sheet (0) and two author sheets (1, 2)
    let (tree, leaf) = support::chain(&["div", "span.x"]);
    let sheet = |text: &str| vapors_css::parse(text).expect("parses");
    let cascade = vapors_css::cascade::Cascade::new()
        .user_agent(sheet("span { display: block; }"))
        .author(sheet(".x { width: 10px; }"))
        .author(sheet("div { color: red; }\n.x { height: 5px; }"));

    // When
    let styles = cascade
        .compute_tree(&tree.node(tree.root()), &support::env())
        .expect("computes");
    let style = &styles.last().expect("the leaf").1;
    let sheet_of = |name: &str| {
        style.declared_in(vapors_css::property::PropertyId::from_name(name).expect("a longhand"))
    };
    let line_of = |name: &str| {
        style
            .declared_at(vapors_css::property::PropertyId::from_name(name).expect("a longhand"))
            .map(|p| p.line)
    };

    // Then: color is inherited from div, so not declared on the span
    assert_eq!(
        [
            sheet_of("display"),
            sheet_of("width"),
            sheet_of("height"),
            sheet_of("color")
        ],
        [Some(0), Some(1), Some(2), None]
    );
    assert_eq!(line_of("height"), Some(2));
    let _ = leaf;
}
