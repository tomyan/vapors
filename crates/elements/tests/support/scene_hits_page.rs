//! A page with a 3D scene over a `gpu-surface`, hit through a fake
//! source (`Dom::watch_hits`, docs/plans/css-for-3d.md C4): the surface
//! is 200 x 100 at (20, 10) with a 2 px border, so its content box
//! starts at (22, 12); `scene` > `group.craft` > `mesh.a`, `mesh.b`.
//! The source answers by halves: `a` left of x 100, `b` right of it,
//! passing over an anchor that does not take the pointer.

#![allow(dead_code)] // each test file uses a subset

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use vapors_dom::{Appearance, Dom, HitAsk, NodeId};
use vapors_elements::Document;

use super::{document, element, viewport};

/// Over `a`'s half, page pixels.
pub const OVER_A: [f32; 2] = [72.0, 52.0];
/// Over `b`'s half, page pixels.
pub const OVER_B: [f32; 2] = [172.0, 52.0];
/// Off the surface, over nothing.
pub const OFF: [f32; 2] = [300.0, 200.0];
/// Over `div.side`, below the surface (y 114 to 134).
pub const OVER_SIDE: [f32; 2] = [50.0, 120.0];

pub const SHEET: &str = "body { margin: 0; } \
    gpu-surface { display: block; margin: 10px 0 0 20px; width: 200px; height: 100px; border: 2px solid #000000; } \
    .side { height: 20px; }";

/// Each ask's point and size.
pub type Asks = Rc<RefCell<Vec<([f32; 2], [f32; 2])>>>;

/// What the fake source answers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Answer {
    /// `a` left of x 100, `b` right, when it takes the pointer.
    Halves,
    /// `b` everywhere: the scene moved under a still pointer.
    AllB,
    /// No node.
    Nothing,
    /// This node, whatever it is.
    Node(NodeId),
    /// An error.
    Fails,
}

pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    pub surface: NodeId,
    pub scene: NodeId,
    pub craft: NodeId,
    pub a: NodeId,
    pub b: NodeId,
    pub side: NodeId,
    pub asks: Asks,
    pub answer: Rc<Cell<Answer>>,
    /// Each appearance `a` reported.
    pub seen: Rc<RefCell<Vec<Appearance>>>,
}

/// The page with `css` after [`SHEET`], `a` watched, the scene's hits
/// watched on "view", updated once.
pub fn page(css: &str) -> Page {
    let mut doc = document();
    let body = doc.body();
    let surface = element(&mut doc, body, "gpu-surface", "");
    doc.set_attribute(surface, "name", "view");
    let scene = element(&mut doc, body, "scene", "");
    let craft = element(&mut doc, scene, "group", "craft");
    let a = element(&mut doc, craft, "mesh", "a");
    let b = element(&mut doc, craft, "mesh", "b");
    let side = element(&mut doc, body, "div", "side");
    doc.add_stylesheet("app.css", &format!("{SHEET}\n{css}"))
        .expect("parses");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let into = Rc::clone(&seen);
    doc.watch_appearance(
        a,
        Box::new(move |appearance| {
            into.borrow_mut().push(appearance);
            Ok(())
        }),
    );
    let page = Page {
        doc: Rc::new(RefCell::new(doc)),
        surface,
        scene,
        craft,
        a,
        b,
        side,
        asks: Rc::default(),
        answer: Rc::new(Cell::new(Answer::Halves)),
        seen,
    };
    page.watch();
    page.update();
    page
}

impl Page {
    fn watch(&self) {
        let (asks, answer, a, b) = (
            Rc::clone(&self.asks),
            Rc::clone(&self.answer),
            self.a,
            self.b,
        );
        let source = move |ask: HitAsk<'_>| {
            asks.borrow_mut().push((ask.point, ask.size));
            match answer.get() {
                Answer::Halves => {
                    let half = if ask.point[0] < 100.0 { a } else { b };
                    Ok((ask.takes)(half).then_some(half))
                }
                Answer::AllB => Ok(Some(b)),
                Answer::Nothing => Ok(None),
                Answer::Node(node) => Ok(Some(node)),
                Answer::Fails => Err("the camera is gone".to_owned()),
            }
        };
        self.doc
            .borrow_mut()
            .watch_hits(self.scene, "view", Box::new(source));
    }

    pub fn update(&self) {
        self.doc
            .borrow_mut()
            .update(viewport(400.0, 300.0))
            .expect("no refusal");
    }

    /// The update's refusals, each as its message; panics when it succeeds.
    pub fn refusals(&self) -> Vec<String> {
        super::refusals(&mut self.doc.borrow_mut())
    }

    pub fn hit(&self, point: [f32; 2]) -> Option<NodeId> {
        self.doc.borrow().hit(point)
    }

    /// The red channel of `a`'s last reported colour.
    pub fn a_red(&self) -> f32 {
        self.seen.borrow().last().expect("a reported").rgb[0]
    }
}
