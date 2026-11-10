//! Two panels side by side, each a title over a `gpu-surface` (as in
//! `examples/surface_panels`), and a HUD button over the left surface,
//! driven through `ElementsApp::give` with no GPU: the document is
//! updated by hand. The surfaces and the HUD log each pointer event they
//! get with its offset, for the `elements_app_surface_*` tests.

// each test crate including this uses a part of it
#![allow(dead_code)]

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use vapors_app::{ElementsApp, Route};
use vapors_dom::{Dom, Event as DomEvent, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Modifiers, Platform, TouchPhase};
use vapors_text::FontSystem;

/// Panels at x 10..190 and 210..390, y 10..170; under a 20 px title,
/// the surfaces' content boxes start at (10, 30) and (210, 30), 180 x
/// 140. The HUD button is at x 40..100, y 50..80, over the left surface.
pub const PANELS: &str = "html, body { margin: 0; } body { display: flex; } \
    .panel { width: 180px; height: 160px; margin: 10px; overflow: hidden; \
    display: flex; flex-direction: column; } \
    .title { height: 20px; flex-shrink: 0; } .view { flex-grow: 1; } \
    .hud { position: absolute; left: 40px; top: 50px; width: 60px; height: 30px; }";

/// `PANELS`, with the left panel scrolling a 300 px surface.
pub fn scrolling() -> String {
    format!(
        "{PANELS} .panel.left {{ overflow: auto; }} .left .view {{ flex: none; height: 300px; }}"
    )
}

type Log = Rc<RefCell<Vec<String>>>;

/// Every pointer event vapors fires: a listener for pointerover,
/// pointerenter, pointerout or pointerleave is refused by the document
/// (`elements_app_surface_input.rs` pins that).
const LOGGED: [&str; 7] = [
    "pointerdown",
    "pointermove",
    "pointerup",
    "pointercancel",
    "click",
    "gotpointercapture",
    "lostpointercapture",
];

/// The page in its app, at a device scale.
pub struct Page {
    pub app: ElementsApp,
    pub doc: Rc<RefCell<Document>>,
    pub left_panel: NodeId,
    pub right_panel: NodeId,
    pub left_surface: NodeId,
    pub right_surface: NodeId,
    pub hud: NodeId,
    log: Log,
    scale: f32,
}

impl Page {
    /// The page styled by `style`, laid out at 400 x 200 CSS pixels and
    /// `scale`. With `captures`, the surfaces and the HUD each take the
    /// mouse's capture on their `pointerdown`.
    pub fn new(style: &str, captures: bool, scale: f32) -> Self {
        let mut doc = Document::new(FontSystem::bundle_only());
        doc.add_stylesheet("surface_input.css", style)
            .expect("parses");
        let log = Log::default();
        let (left_panel, left_surface) = panel(&mut doc, "left", (&log, captures));
        let (right_panel, right_surface) = panel(&mut doc, "right", (&log, captures));
        let body = doc.body();
        let hud = child(&mut doc, body, "button", "hud");
        logged(&mut doc, hud, "hud", (&log, captures), false);
        logged(&mut doc, body, "body", (&log, false), true);
        let doc = Rc::new(RefCell::new(doc));
        let page = Self {
            app: ElementsApp::new(Rc::clone(&doc)),
            doc,
            left_panel,
            right_panel,
            left_surface,
            right_surface,
            hud,
            log,
            scale,
        };
        page.update();
        page
    }

    /// Lays the document out again, as the app's next frame would.
    pub fn update(&self) {
        let viewport = Viewport {
            width: 400.0,
            height: 200.0,
            scale: self.scale,
        };
        self.doc.borrow_mut().update(viewport).expect("updates");
    }

    /// Gives `event`, then updates: who it was for.
    pub fn give(&mut self, event: &Event) -> Route {
        let route = self.app.give(event, &Platform::detached()).expect("placed");
        self.update();
        route
    }

    /// The mouse moves to (`x`, `y`) in CSS pixels.
    pub fn move_to(&mut self, x: f32, y: f32) -> Route {
        let [x, y] = [x * self.scale, y * self.scale];
        self.give(&Event::PointerMoved { x, y })
    }

    /// A key goes down (`true`) or up.
    pub fn key(&mut self, code: &str, pressed: bool) -> Route {
        self.give(&Event::Key {
            key: code.to_owned(),
            code: code.to_owned(),
            pressed,
            repeat: false,
            modifiers: Modifiers::default(),
        })
    }

    /// The primary button goes down (`true`) or up where the mouse is.
    pub fn button(&mut self, pressed: bool) -> Route {
        let (clicks, modifiers) = (NonZeroU32::MIN, Modifiers::default());
        self.give(&Event::PrimaryButton {
            pressed,
            clicks,
            modifiers,
        })
    }

    /// Finger 1 at (`x`, `y`) in CSS pixels.
    pub fn touch(&mut self, phase: TouchPhase, x: f32, y: f32) -> Route {
        let [x, y] = [x * self.scale, y * self.scale];
        self.give(&Event::Touch { id: 1, phase, x, y })
    }

    /// The platform's tap at (`x`, `y`) in CSS pixels, which the
    /// document turns into a `click`; sent after the finger's lift.
    pub fn tap(&mut self, x: f32, y: f32) -> Route {
        let [x, y] = [x * self.scale, y * self.scale];
        let clicks = NonZeroU32::MIN;
        self.give(&Event::Tap { x, y, clicks })
    }

    /// The events logged since the last take: `"<who> <event> <x>,<y>"`.
    pub fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

/// A panel titled and holding the surface `name`; the panel and the
/// surface. The panel logs only what is aimed at it, as `"<name>-panel"`.
fn panel(doc: &mut Document, name: &str, logging: (&Log, bool)) -> (NodeId, NodeId) {
    let body = doc.body();
    let panel = child(doc, body, "div", &format!("panel {name}"));
    logged(
        doc,
        panel,
        &format!("{name}-panel"),
        (logging.0, false),
        true,
    );
    child(doc, panel, "div", "title");
    let surface = child(doc, panel, "gpu-surface", "view");
    doc.set_attribute(surface, "name", name);
    logged(doc, surface, name, logging, false);
    (panel, surface)
}

fn child(doc: &mut Document, parent: NodeId, tag: &str, class: &str) -> NodeId {
    let made = doc.create_element(tag);
    doc.set_attribute(made, "class", class);
    doc.append(parent, made);
    made
}

/// Logs `LOGGED` on `node` as `who`, capturing on `pointerdown` when
/// asked; with `own`, only events aimed at `node` itself, not bubbled
/// up from a descendant.
fn logged(doc: &mut Document, node: NodeId, who: &str, (log, captures): (&Log, bool), own: bool) {
    for name in LOGGED {
        let (log, who) = (Rc::clone(log), who.to_owned());
        let capture = captures && name == "pointerdown";
        let listener = move |e: &mut DomEvent| {
            if own && e.target() != Some(node) {
                return;
            }
            let [x, y] = e.offset();
            log.borrow_mut().push(format!("{who} {} {x},{y}", e.name()));
            if capture {
                e.set_pointer_capture();
            }
        };
        doc.listen(node, name, Box::new(listener));
    }
}
