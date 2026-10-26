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
use crate::events_script::{Action, Script};

/// What vapors did at each step.
pub type Log = Vec<Logged>;

/// One step's events as (type, target), and the element focused after
/// it, named as the Chromium player names it (`body` for none).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Logged {
    /// The events, in order.
    pub events: Vec<(String, String)>,
    /// The focused element.
    pub focus: String,
}

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
    let mut log = Log::new();
    let mut press = Press::default();
    for step in &script.steps {
        act(&doc, &step.action, &mut press)
            .map_err(|why| format!("{}.events:{}: {why}", fixture.name, step.line))?;
        let focus = {
            let doc = doc.borrow();
            doc.active_element()
                .map_or_else(|| "body".to_owned(), |node| name(&doc, node))
        };
        log.push(Logged {
            events: heard.borrow_mut().drain(..).collect(),
            focus,
        });
    }
    Ok(log)
}

fn mounted(fixture: &Fixture, viewport: Viewport) -> Result<Rc<RefCell<Document>>, String> {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
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
                heard.borrow_mut().push((e.name().to_owned(), target));
            }),
        );
    }
}

/// The platform's tap slop (`vapors_platform::TAP_SLOP`, which this
/// crate does not depend on; `tests/events_vapors.rs` holds the two
/// equal): a finger pans only once it has moved further than this from
/// where it went down, as the platform's `Gestures` sends pans.
pub const TAP_SLOP: f32 = 20.0;

/// Where the finger went down, while it is still inside the tap slop.
#[derive(Default)]
struct Press(Option<[f32; 2]>);

fn act(doc: &Rc<RefCell<Document>>, action: &Action, press: &mut Press) -> Result<(), String> {
    #[allow(clippy::cast_possible_truncation)] // CSS px within a viewport
    let at = |x: f64, y: f64| [x as f32, y as f32];
    match *action {
        Action::Click(x, y) => {
            Document::pointer_pressed(doc, at(x, y));
            Document::pointer_released(doc, at(x, y));
        }
        Action::Key(ref combination) => {
            let (key, modifiers) = key_of(combination);
            Document::key_pressed(doc, &key, modifiers);
            Document::key_released(doc, &key, modifiers);
        }
        Action::Move(x, y) => Document::pointer_moved(doc, at(x, y)),
        Action::Press(x, y) => Document::pointer_pressed(doc, at(x, y)),
        Action::Release(x, y) => Document::pointer_released(doc, at(x, y)),
        Action::TouchDown(x, y) => {
            press.0 = Some(at(x, y));
            Document::touch_pressed(doc, 0, at(x, y));
        }
        Action::TouchMove(x, y) => {
            Document::touch_moved(doc, 0, at(x, y));
            if press.left(at(x, y)) {
                // past the slop the platform pans, which the app hands to
                // `touch_panned`
                Document::touch_panned(doc);
            }
        }
        // inside the slop the platform would send a tap, which this
        // player does not model: refused, not approximated
        Action::TouchUp(..) if press.0.is_some() => {
            return Err(
                "a touch that never left the tap slop is a tap, which is not played on vapors yet"
                    .into(),
            );
        }
        Action::TouchUp(x, y) => Document::touch_released(doc, 0, at(x, y)),
        ref other => return Err(format!("{other:?} is not played on vapors yet")),
    }
    Ok(())
}

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

impl Press {
    /// Whether the finger, now at `at`, has left the tap slop (it stays
    /// out once it has).
    fn left(&mut self, at: [f32; 2]) -> bool {
        let Some(down) = self.0 else { return true };
        if (at[0] - down[0]).hypot(at[1] - down[1]) <= TAP_SLOP {
            return false;
        }
        self.0 = None;
        true
    }
}

/// A script's `key` (`Shift+Tab`, `Space`, `x`) as the DOM's key and the
/// modifiers held, as the Chromium player sends it.
fn key_of(combination: &str) -> (String, vapors_elements::Modifiers) {
    let parts: Vec<&str> = combination.split('+').collect();
    let (held, key) = parts.split_at(parts.len() - 1);
    let modifiers = vapors_elements::Modifiers {
        shift: held.contains(&"Shift"),
        control: held.contains(&"Control"),
        alt: held.contains(&"Alt"),
        meta: held.contains(&"Meta"),
    };
    let key = match key[0] {
        "Space" => " ".to_owned(),
        other => other.to_owned(),
    };
    (key, modifiers)
}

/// The element `target` names: `tag#id` only.
fn find(doc: &Document, target: &str) -> Option<NodeId> {
    let (tag, id) = target.split_once('#')?;
    doc.elements()
        .into_iter()
        .find(|node| doc.tag(*node) == Some(tag) && doc.attribute(*node, "id") == Some(id))
}
