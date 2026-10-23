//! A component with a `<style>` adds it under its scope when mounted and
//! tags every element it creates with that scope; a child component
//! tags its own elements with its own scope. A component without a
//! style does neither.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::styled_card::{STYLE, StyledCard, StyledCardProps};
use vapors_compiler_examples::todo_list::{TodoList, TodoListProps};
use vapors_dom::recording::{Call, RecordingDom};

const CARD: &str = "vapors-compiler-examples/src/styled_card.wisp";
const TITLE: &str = "vapors-compiler-examples/src/styled_title.wisp";

fn dom() -> Rc<RefCell<RecordingDom>> {
    Rc::new(RefCell::new(RecordingDom::new()))
}

fn styles_added(dom: &Rc<RefCell<RecordingDom>>) -> Vec<(String, String)> {
    dom.borrow()
        .calls()
        .iter()
        .filter_map(|call| match call {
            Call::AddStyle { scope, css } => Some((scope.clone(), css.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn mounting_adds_the_style_under_the_components_scope_before_any_element() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();

    // When
    let _card = StyledCard::mount(&dom, root, StyledCardProps::default());

    // Then: the card's style first, then its child's
    let first = dom.borrow().calls().first().cloned();
    assert_eq!(
        first,
        Some(Call::AddStyle {
            scope: CARD.to_owned(),
            css: STYLE.to_owned()
        })
    );
    let scopes: Vec<String> = styles_added(&dom).into_iter().map(|(s, _)| s).collect();
    assert_eq!(scopes, [CARD, TITLE]);
}

#[test]
fn each_element_carries_the_scope_of_the_component_that_created_it() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();

    // When
    let _card = StyledCard::mount(&dom, root, StyledCardProps::default());

    // Then
    assert_eq!(
        dom.borrow().to_html(),
        format!(
            "<section data-wisp=\"{CARD}\"><h2 class=\"title\" data-wisp=\"{CARD}\">Card</h2><p class=\"title\" data-wisp=\"{TITLE}\">Inner</p></section>"
        )
    );
}

#[test]
fn a_component_without_a_style_adds_none_and_tags_nothing() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();

    // When
    let _list = TodoList::mount(&dom, root, TodoListProps::default());

    // Then
    assert_eq!(styles_added(&dom), Vec::new());
    assert!(
        !dom.borrow().to_html().contains("data-wisp"),
        "{}",
        dom.borrow().to_html()
    );
}

#[test]
fn the_compiler_reserves_the_attribute_the_dom_scopes_by() {
    // Then: one name, held in two crates that do not depend on each other
    assert_eq!(
        vapors_compiler::SCOPE_ATTRIBUTE,
        vapors_dom::SCOPE_ATTRIBUTE
    );
}
