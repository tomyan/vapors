//! A node component's children: built in its anchor, and refused unless
//! each is a scene node. Markup there would be built in a `display:
//! none` anchor and never seen.

use crate::placement::KEY_ATTRIBUTE;
use std::cell::RefCell;
use std::rc::Rc;
use vapors_dom::{Dom, NodeId as ElementId, Snippet, current_nodes};

/// Builds `children` in `anchor`.
///
/// # Panics
///
/// When a child is an element that is not a scene node's anchor, or
/// text other than a block's empty marker, naming `component` and the
/// element's template site. Only what the first build makes is checked:
/// an `{if}` or `{for}` inside that is rebuilt later is not, until the
/// compiler can refuse it at build time.
pub(crate) fn render_children(
    dom: &Rc<RefCell<dyn Dom>>,
    anchor: ElementId,
    children: &Snippet,
    component: &str,
) {
    let roots = children.render(dom, anchor, None, || ());
    let dom = dom.borrow();
    for child in current_nodes(&roots) {
        if let Some(refusal) = refusal(&*dom, child) {
            panic!(
                "<{component}> holds {refusal}, which a scene does not draw: inside <Scene> and <Group> put only <Group>, <Mesh> and <Camera>, and put markup outside the <Scene>"
            );
        }
    }
}

/// What `child` is when it is not a scene node or a block's marker.
fn refusal(dom: &dyn Dom, child: ElementId) -> Option<String> {
    if let Some(text) = dom.current_text(child) {
        return (!text.is_empty()).then(|| format!("the text {text:?}"));
    }
    if dom.current_attribute(child, KEY_ATTRIBUTE).is_some() {
        return None;
    }
    Some(match dom.style_key(child) {
        Some(key) => format!("<{}> at {}", key.tag, key.site),
        None => "an element".to_owned(),
    })
}
