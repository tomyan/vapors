//! The random changes of `incremental_random.rs`: a page, a CSS that
//! exercises every widening of a restyle, and one change at a time
//! from a seeded generator.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

use super::document;
use super::tree::{is_inside, text_nodes};

/// Inherited properties, `var()`, `+` and `~`, `:nth-child`,
/// `:last-child`, `:empty`, an ancestor attribute, `:hover` on a
/// non-subject compound, `:active`, `@media`, calc with % and em, and
/// vw.
pub const CSS: &str = "
:root { --pad: 3px; }
body { margin: 0; font-size: 14px; color: #222222; }
.row { display: flex; gap: 4px; }
.box { padding: var(--pad); background-color: #eeeeff; }
.big { font-size: 1.2em; }
.box + .box { margin-top: 2px; }
.box ~ .alt { color: #aa0000; }
.box:nth-child(odd) { background-color: #ffffee; }
.box:last-child { border-bottom: 1px solid #333333; }
.box:empty { height: 6px; }
[data-on] .box { color: #006600; }
.row:hover .box { background-color: #ccffcc; }
.box:active { background-color: #ffcccc; }
.wide { width: calc(50% - 1em); }
.vw { width: 30vw; }
@media (min-width: 500px) { .box { padding: 6px; } }
";

const CLASSES: &[&str] = &["box", "box big", "box alt", "box wide", "box vw", "row", ""];
const VIEWPORTS: &[(f32, f32)] = &[(640.0, 400.0), (400.0, 300.0), (700.0, 500.0)];

/// xorshift64: a seed gives the same changes on every machine.
pub struct Random(pub u64);

impl Random {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).expect("small")
    }

    pub fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.below(from.len())]
    }
}

/// body > 3 rows of up to 3 boxes with text.
pub fn page(random: &mut Random) -> Rc<RefCell<Document>> {
    let mut doc = document();
    let body = doc.body();
    for _ in 0..3 {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.append(body, row);
        for index in 0..random.below(4) {
            let cell = doc.create_element("div");
            doc.set_attribute(cell, "class", "box");
            let text = doc.create_text(&format!("cell {index}"));
            doc.append(cell, text);
            doc.append(row, cell);
        }
    }
    doc.add_stylesheet("random.css", CSS).expect("parses");
    Rc::new(RefCell::new(doc))
}

/// One random change, described for a failure message; the viewport
/// may change with it.
pub fn change(
    doc: &Rc<RefCell<Document>>,
    random: &mut Random,
    viewport: &mut Viewport,
    step: usize,
) -> String {
    let (body, nodes) = {
        let d = doc.borrow();
        (
            d.body(),
            d.elements().into_iter().skip(2).collect::<Vec<NodeId>>(),
        )
    };
    let any = |random: &mut Random| {
        if nodes.is_empty() {
            body
        } else {
            random.pick(&nodes)
        }
    };
    let coordinate = |random: &mut Random, below: usize| {
        f32::from(u16::try_from(random.below(below)).expect("a few hundred"))
    };
    let point = [coordinate(random, 640), coordinate(random, 400)];
    match random.below(11) {
        0 => {
            let (node, class) = (any(random), random.pick(CLASSES));
            doc.borrow_mut().set_attribute(node, "class", class);
            format!("class of {node:?} to {class:?}")
        }
        1 => {
            let node = any(random);
            doc.borrow_mut()
                .set_attribute(node, "data-on", &step.to_string());
            format!("data-on on {node:?}")
        }
        2 => {
            let (node, style) = (any(random), random.pick(&["display: none", ""]));
            doc.borrow_mut().set_attribute(node, "style", style);
            format!("style of {node:?} to {style:?}")
        }
        3 => set_some_text(doc, random, step),
        4 => {
            let parent = any(random);
            let mut d = doc.borrow_mut();
            let cell = d.create_element("div");
            d.set_attribute(cell, "class", "box");
            let text = d.create_text(&format!("new {step}"));
            d.append(cell, text);
            let before = d.children(parent).first().copied();
            d.insert_before(parent, cell, before);
            format!("insert a box into {parent:?}")
        }
        5 => move_one(doc, random, &nodes, body),
        6 if !nodes.is_empty() => {
            let node = random.pick(&nodes);
            doc.borrow_mut().remove(node);
            format!("remove {node:?}")
        }
        7 => {
            Document::pointer_moved(doc, point);
            format!("hover at {point:?}")
        }
        8 => {
            Document::pointer_pressed(doc, point);
            Document::pointer_released(doc, point);
            format!("press and release at {point:?}")
        }
        9 => {
            let (width, height) = random.pick(VIEWPORTS);
            *viewport = super::viewport(width, height);
            format!("resize to {width}x{height}")
        }
        _ => {
            let mut d = doc.borrow_mut();
            d.add_stylesheet(&format!("more{step}.css"), ".row { padding-left: 1px; }")
                .expect("parses");
            d.add_style(
                &format!("scope{step}"),
                ":global(.box) { margin-left: 1px; }",
            );
            "add a sheet and a scoped style".to_owned()
        }
    }
}

fn set_some_text(doc: &Rc<RefCell<Document>>, random: &mut Random, step: usize) -> String {
    let texts = text_nodes(&doc.borrow());
    if texts.is_empty() {
        return "no text to change".to_owned();
    }
    let node = random.pick(&texts);
    // Empty as often as not, so :empty flips both ways.
    let text = match random.below(4) {
        0 | 1 => String::new(),
        2 => format!("x{step}"),
        _ => format!("a longer text that wraps in a narrow box {step}"),
    };
    doc.borrow_mut().set_text(node, &text);
    format!("text of {node:?} to {text:?}")
}

/// Moves an element under another that is not inside it.
fn move_one(
    doc: &Rc<RefCell<Document>>,
    random: &mut Random,
    nodes: &[NodeId],
    body: NodeId,
) -> String {
    if nodes.is_empty() {
        return "nothing to move".to_owned();
    }
    let node = random.pick(nodes);
    let mut d = doc.borrow_mut();
    let targets: Vec<NodeId> = std::iter::once(body)
        .chain(nodes.iter().copied())
        .filter(|t| !is_inside(&d, *t, node))
        .collect();
    let target = random.pick(&targets);
    d.append(target, node);
    format!("move {node:?} into {target:?}")
}
