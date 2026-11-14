//! One answer to "is this a link" (`Document::is_link`: an `a` with an
//! `href`) for focus, Enter, the a11y role and following, held here
//! together with the UA sheet's `a[href]` rule, which CSS cannot share:
//! a link is focusable, followed on Enter, link-coloured and given the
//! link role; an `a` without `href` gets none of those.

mod support;

use support::link_page::{Page, key, page, taken};
use support::viewport;
use vapors_dom::NodeId;
use vapors_elements::Document;

/// Whether `node` is focusable, followed on Enter, coloured as a link,
/// and has the link role.
fn as_link(page: &Page, node: NodeId) -> [bool; 4] {
    let focused = {
        Document::focus(&page.doc, None);
        Document::focus(&page.doc, Some(node));
        page.doc.borrow().active_element() == Some(node)
    };
    key(page, "Enter");
    let followed = taken(page).is_some();
    page.doc
        .borrow_mut()
        .update(viewport(200.0, 100.0))
        .expect("updates");
    let doc = page.doc.borrow();
    let style = doc.computed_style(node).expect("styled");
    let coloured = style.get_by_name("color").expect("a longhand").to_css() == "rgb(0 0 238)";
    let tree = doc.ax_tree().expect("an updated document");
    let role = tree.iter().any(|ax| ax.role == "link" && ax.node == node);
    [focused, followed, coloured, role]
}

#[test]
fn a_link_is_all_of_a_link_and_an_a_without_href_none_of_it() {
    // Given
    let page = page();

    // When
    let (link, plain) = (as_link(&page, page.link), as_link(&page, page.plain));

    // Then
    assert_eq!(link, [true; 4]);
    assert_eq!(plain, [false; 4]);
}
