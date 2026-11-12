//! An `.events` script played on vapors (3-E1's capture oracle): the
//! fixture mounted and laid out, the script's `capture` targets given a
//! `pointerdown` listener that calls `set_pointer_capture`, each pointer
//! action sent through the document's pointer calls, and every event of
//! a listened type logged with its target, named as the Chromium player
//! names it. Its log is compared, step by step, with what Chromium
//! logged (`<name>.events.json`). An action vapors cannot play here yet
//! is refused, naming its line.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

use crate::Fixture;
use crate::events_script::Script;

/// What vapors did at each step.
pub type Log = Vec<Logged>;

/// One step's events as (type, target), the element focused after it,
/// named as the Chromium player names it (`body` for none), and the
/// value of each control the script expects a value of.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Logged {
    /// The events, in order.
    pub events: Vec<(String, String)>,
    /// The focused element.
    pub focus: String,
    /// Whether it matches `:focus-visible`.
    pub focus_visible: bool,
    /// Each value target's value, in the script's sorted order.
    pub values: Vec<(String, String)>,
    /// Each checked target's state, in the script's sorted order.
    pub checked: Vec<(String, bool)>,
    /// Each selection target's selection, in the script's sorted order.
    pub selections: Vec<(String, Selection)>,
    /// The a11y projection to `project`'s roles, in the oracle's shape
    /// (`ax_projection`); `None` without `project`.
    pub ax: Option<serde_json::Value>,
}

/// A text field's selection, start then end, in UTF-16 units.
pub type Selection = (usize, usize);

type Shared = Rc<RefCell<Vec<(String, String)>>>;

/// The script played on fixture `fixture`, laid out at `viewport`.
///
/// # Errors
/// A refused update, a `capture` target that names no element, or an
/// action not played here, each naming the script's line.
pub fn play(fixture: &Fixture, script: &Script, viewport: Viewport) -> Result<Log, String> {
    let doc = mounted(fixture, viewport)?;
    let heard = Shared::default();
    listen(&doc, &script.listen, &heard);
    for target in &script.capture {
        let node = find(&doc.borrow(), target)
            .ok_or_else(|| format!("{}.events: capture {target} names no element", fixture.name))?;
        doc.borrow_mut().listen(
            node,
            "pointerdown",
            Box::new(|e: &mut Event| e.set_pointer_capture()),
        );
    }
    for target in &script.cancel {
        let node = find(&doc.borrow(), target)
            .ok_or_else(|| format!("{}.events: cancel {target} names no element", fixture.name))?;
        doc.borrow_mut().listen(
            node,
            "pointerdown",
            Box::new(|e: &mut Event| e.prevent_default()),
        );
    }
    let mut log = Log::new();
    let mut held = Held::default();
    for step in &script.steps {
        act(&doc, &step.action, &mut held)
            .map_err(|why| format!("{}.events:{}: {why}", fixture.name, step.line))?;
        // a frame after each step, as Chromium draws one before it is read
        doc.borrow_mut().update(viewport).map_err(|errors| {
            format!(
                "{}.events:{}: the update was refused: {errors:?}",
                fixture.name, step.line
            )
        })?;
        // and the focus fixup the app runs after it
        Document::settle_focus(&doc);
        let (focus, focus_visible) = {
            let doc = doc.borrow();
            let focus = doc
                .active_element()
                .map_or_else(|| "body".to_owned(), |node| name(&doc, node));
            (focus, doc.focus_visible())
        };
        let values = values(&doc.borrow(), &script.value_targets())
            .map_err(|why| format!("{}.events:{}: {why}", fixture.name, step.line))?;
        let checked = checked(&doc.borrow(), &script.checked_targets())
            .map_err(|why| format!("{}.events:{}: {why}", fixture.name, step.line))?;
        let selections = selections(&doc.borrow());
        let ax = projection(&doc.borrow(), &script.project)
            .map_err(|why| format!("{}.events:{}: {why}", fixture.name, step.line))?;
        log.push(Logged {
            events: heard.borrow_mut().drain(..).collect(),
            focus,
            focus_visible,
            values,
            checked,
            selections,
            ax,
        });
    }
    Ok(log)
}

fn mounted(fixture: &Fixture, viewport: Viewport) -> Result<Rc<RefCell<Document>>, String> {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    // the answers are Linux Chromium's: Control+A is select-all
    doc.borrow_mut()
        .set_shortcut_modifier(vapors_elements::ShortcutModifier::Control);
    doc.borrow_mut()
        .add_stylesheet(fixture.name, fixture.style)
        .map_err(|e| format!("{}: {e:?}", fixture.name))?;
    let body = doc.borrow().body();
    (fixture.mount)(&doc, body);
    doc.borrow_mut()
        .update(viewport)
        .map_err(|errors| format!("{}: the update was refused: {errors:?}", fixture.name))?;
    Ok(doc)
}

/// A body listener per type logs every event of it, with its target:
/// the listened types all bubble, so each reaches the body once.
fn listen(doc: &Rc<RefCell<Document>>, types: &[String], heard: &Shared) {
    let body = doc.borrow().body();
    for kind in types {
        let (heard, names) = (Rc::clone(heard), Rc::downgrade(doc));
        doc.borrow_mut().listen(
            body,
            kind,
            Box::new(move |e: &mut Event| {
                let target = match (names.upgrade(), e.target()) {
                    (Some(doc), Some(node)) => name(&doc.borrow(), node),
                    _ => "(none)".to_owned(),
                };
                heard.borrow_mut().push((logged_type(e), target));
            }),
        );
    }
}

pub use act::TAP_SLOP;
use act::{Held, act};

/// `tag#id`, else the child-index path from body, as the Chromium player
/// names a target.
fn name(doc: &Document, node: NodeId) -> String {
    let tag = doc.tag(node).unwrap_or("?").to_owned();
    if node == doc.body() || doc.parent(node).is_none() {
        return tag;
    }
    if let Some(id) = doc.attribute(node, "id") {
        return format!("{tag}#{id}");
    }
    let parent = doc.parent(node).expect("checked above");
    let siblings: Vec<NodeId> = doc
        .children(parent)
        .into_iter()
        .filter(|n| doc.tag(*n).is_some())
        .collect();
    let index = siblings
        .iter()
        .position(|n| *n == node)
        .map_or(0, |i| i + 1);
    format!("{}>{tag}:nth-child({index})", name(doc, parent))
}

/// The element `target` names: `tag#id` only.
fn find(doc: &Document, target: &str) -> Option<NodeId> {
    let (tag, id) = target.split_once('#')?;
    doc.elements()
        .into_iter()
        .find(|node| doc.tag(*node) == Some(tag) && doc.attribute(*node, "id") == Some(id))
}

#[path = "events_vapors_act.rs"]
mod act;

#[path = "events_vapors_read.rs"]
mod read;
use read::{checked, projection, selections, values};

/// The event's type as the log names it, each modifier held added as the
/// recording adds it (`events_recording::MODIFIER_KEYS`): `click+alt`.
/// Pointer events only, as the recording script reads the modifiers of
/// a `MouseEvent` alone (`events_play.js`); a key event's are not logged.
fn logged_type(event: &vapors_dom::Event) -> String {
    if event.pointer().is_none() {
        return event.name().to_owned();
    }
    let m = event.modifiers();
    let held = [m.alt, m.control, m.meta, m.shift];
    crate::events_recording::MODIFIER_KEYS
        .iter()
        .zip(held)
        .filter(|(_, held)| *held)
        .fold(event.name().to_owned(), |logged, ((_, name), _)| {
            format!("{logged}+{name}")
        })
}
