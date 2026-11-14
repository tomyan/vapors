//! Links (`<a href>`): a click that no listener prevented, on a link or
//! on what it holds, asks to go to the link's `href`, where its
//! modifiers say, as in Chromium: here, or with Control or Meta in a new
//! tab, with Shift in a new window, and with Alt as a download. The
//! link is found on the click's path before its listeners run, so a
//! listener that removes the link or what was clicked does not stop it
//! (Chromium follows both). A checkbox on the path takes the click
//! instead: its toggle is the activation, and the link is not followed
//! (Chromium; a button inside a link still follows it). The app takes
//! the request (`take_navigation`) and hands it to the platform. Enter
//! on a focused link clicks it, with its modifiers (`keys.rs`); Space
//! does not.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Modifiers, NodeId};

use crate::document::Document;

/// Where a followed link opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    /// In place of the page: an unmodified activation.
    Here,
    /// In a new tab: Control or Meta held.
    NewTab,
    /// In a new window: Shift held.
    NewWindow,
    /// As a download: Alt held.
    Download,
}

impl Opened {
    /// Where `modifiers` open a link, as Chromium has it: Control or Meta
    /// first, then Shift, then Alt.
    #[must_use]
    pub fn by(modifiers: Modifiers) -> Self {
        if modifiers.control || modifiers.meta {
            Self::NewTab
        } else if modifiers.shift {
            Self::NewWindow
        } else if modifiers.alt {
            Self::Download
        } else {
            Self::Here
        }
    }
}

/// A followed link: its address and where it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Navigation {
    /// The link's `href`, as written.
    pub href: String,
    /// Where it opens.
    pub opened: Opened,
}

/// The link a click at `target` would follow, found before its
/// listeners run: the node and its `href` then.
pub(crate) fn on_path(
    document: &Rc<RefCell<Document>>,
    target: NodeId,
) -> Option<(NodeId, String)> {
    let doc = document.borrow();
    let link = doc.link_on_path(target)?;
    Some((link, doc.attribute(link, "href")?.to_owned()))
}

/// After a click that no listener prevented: `link` is followed, its
/// `href` read again if it is still in the document (a listener may
/// have changed it), else as it was before the click.
pub(crate) fn follow(
    document: &Rc<RefCell<Document>>,
    (link, before): (NodeId, String),
    modifiers: Modifiers,
) {
    let now = document.borrow().still_in(link).and_then(|link| {
        let doc = document.borrow();
        doc.attribute(link, "href").map(str::to_owned)
    });
    document.borrow_mut().navigation = Some(Navigation {
        href: now.unwrap_or(before),
        opened: Opened::by(modifiers),
    });
}

impl Document {
    /// The link the last followed click asks to go to, once: the app
    /// takes it after each event and asks the platform to open it.
    pub fn take_navigation(&mut self) -> Option<Navigation> {
        self.navigation.take()
    }

    /// Whether `node` is a link: an `a` with an `href`. The one answer
    /// for focus, key activation, the a11y role and following; the UA
    /// sheet's `a[href]` is held to it by `tests/a_href_link_rule.rs`.
    pub(crate) fn is_link(&self, node: NodeId) -> bool {
        self.tag(node) == Some("a") && self.attribute(node, "href").is_some()
    }

    /// The nearest link at or above `node`, unless a checkbox comes
    /// first: the checkbox's toggle is the click's activation then.
    fn link_on_path(&self, node: NodeId) -> Option<NodeId> {
        let mut at = Some(node);
        while let Some(node) = at {
            if self.checked(node).is_some() {
                return None;
            }
            if self.is_link(node) {
                return Some(node);
            }
            at = self.parent(node);
        }
        None
    }
}
