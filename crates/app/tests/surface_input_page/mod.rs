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

const LOGGED: [&str; 5] = [
    "pointerdown",
    "pointermove",
    "pointerup",
    "pointercancel",
    "click",
];

/// The page in its app, at a device scale.
pub struct Page {
    pub app: ElementsApp,
    pub doc: Rc<RefCell<Document>>,
    pub left_panel: NodeId,
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
        let left_panel = panel(&mut doc, "left", (&log, captures));
        panel(&mut doc, "right", (&log, captures));
        let body = doc.body();
        let hud = child(&mut doc, body, "button", "hud");
        logged(&mut doc, hud, "hud", (&log, captures));
        let doc = Rc::new(RefCell::new(doc));
        let page = Self {
            app: ElementsApp::new(Rc::clone(&doc)),
            doc,
            left_panel,
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

    /// The events logged since the last take: `"<who> <event> <x>,<y>"`.
    pub fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

/// A panel titled and holding the surface `name`; the panel.
fn panel(doc: &mut Document, name: &str, logging: (&Log, bool)) -> NodeId {
    let body = doc.body();
    let panel = child(doc, body, "div", &format!("panel {name}"));
    child(doc, panel, "div", "title");
    let surface = child(doc, panel, "gpu-surface", "view");
    doc.set_attribute(surface, "name", name);
    logged(doc, surface, name, logging);
    panel
}

fn child(doc: &mut Document, parent: NodeId, tag: &str, class: &str) -> NodeId {
    let made = doc.create_element(tag);
    doc.set_attribute(made, "class", class);
    doc.append(parent, made);
    made
}

/// Logs `LOGGED` on `node` as `who`, capturing on `pointerdown` when asked.
fn logged(doc: &mut Document, node: NodeId, who: &str, (log, captures): (&Log, bool)) {
    for name in LOGGED {
        let (log, who) = (Rc::clone(log), who.to_owned());
        let capture = captures && name == "pointerdown";
        let listener = move |e: &mut DomEvent| {
            let [x, y] = e.offset();
            log.borrow_mut().push(format!("{who} {} {x},{y}", e.name()));
            if capture {
                e.set_pointer_capture();
            }
        };
        doc.listen(node, name, Box::new(listener));
    }
}
