//! Links (`<a href>`): a click on a link, or on what it holds, that no
//! listener prevented asks to go to its `href`; the app takes the
//! address (`take_navigation`) and hands it to the platform, which on
//! the web sets the page's address. Enter on a focused link clicks it
//! (`keys.rs`), Space does not, as in Chromium.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;

/// After a click at `target` that no listener prevented: the `href` of
/// the nearest link at or above it, if any, is where to go; nothing
/// when a listener removed the target.
pub(crate) fn follow(document: &Rc<RefCell<Document>>, target: NodeId) {
    // a listener may have removed the target
    let Some(target) = document.borrow().still_in(target) else {
        return;
    };
    let href = document.borrow().link_href(target);
    if let Some(href) = href {
        document.borrow_mut().navigation = Some(href);
    }
}

impl Document {
    /// The address the last followed link asks to go to, once: the app
    /// takes it after each event and asks the platform to go there.
    pub fn take_navigation(&mut self) -> Option<String> {
        self.navigation.take()
    }

    /// The `href` of the nearest `a` with one at or above `node`.
    fn link_href(&self, node: NodeId) -> Option<String> {
        let mut at = Some(node);
        while let Some(node) = at {
            if self.tag(node) == Some("a")
                && let Some(href) = self.attribute(node, "href")
            {
                return Some(href.to_owned());
            }
            at = self.parent(node);
        }
        None
    }
}
