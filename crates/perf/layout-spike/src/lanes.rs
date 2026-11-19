//! Option (a): the shipped layout, vapors-elements' `Document`: CSS,
//! lanes for flex and block containers, Taffy beneath. Its update also
//! styles and paints; `clock::layout_phase` reads the layout phase.

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

use crate::spec::{DEEP_SIDE, VIEW_W, middle};
use crate::{Engine, Rect, Unsupported, Workload};

const TABLE_CSS: &str = "body { margin: 0; display: flex; flex-direction: column; } \
    .row { display: flex; height: 24px; } \
    .c1, .c2, .c3, .c4, .c2w { flex-grow: 0; flex-shrink: 0; height: 24px; } \
    .c1 { width: 80px; } .c2 { width: 120px; } .c2w { width: 150px; } \
    .c3 { width: 100px; } .c4 { width: 140px; } \
    .c5 { flex-grow: 1; flex-shrink: 1; flex-basis: 0px; height: 24px; }";

const DEEP_CSS: &str = "body { margin: 0; display: flex; flex-direction: column; \
    box-sizing: border-box; width: 2000px; height: 2000px; padding: 2px; } \
    .col, .rw { display: flex; flex-grow: 1; flex-shrink: 1; flex-basis: 0px; \
    box-sizing: border-box; padding: 2px; } \
    .col { flex-direction: column; } .rw { flex-direction: row; } \
    .leaf, .big { flex-grow: 0; flex-shrink: 0; } \
    .leaf { width: 10px; height: 10px; } .big { width: 14px; height: 14px; }";

pub struct Lanes {
    doc: Document,
    view: Viewport,
    changed: NodeId,
    probes: [NodeId; 2],
    flipped: bool,
    /// The changed leaf's class before and after the change.
    classes: [&'static str; 2],
}

fn div(doc: &mut Document, parent: NodeId, class: &str) -> NodeId {
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", class);
    doc.append(parent, node);
    node
}

fn new_doc(css: &str) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("spike.css", css)
        .unwrap_or_else(|e| panic!("spike css refused: {e:?}"));
    doc
}

fn table(rows: usize) -> Lanes {
    let mut doc = new_doc(TABLE_CSS);
    let body = doc.body();
    let mut mid = vec![];
    for index in 0..rows {
        let row = div(&mut doc, body, "row");
        let cells: Vec<NodeId> = ["c1", "c2", "c3", "c4", "c5"]
            .iter()
            .map(|c| div(&mut doc, row, c))
            .collect();
        if index == middle(rows) {
            mid = cells;
        }
    }
    let view = Viewport {
        width: VIEW_W,
        height: 600.0,
        scale: 1.0,
    };
    Lanes {
        doc,
        view,
        changed: mid[1],
        probes: [mid[1], mid[4]],
        flipped: false,
        classes: ["c2", "c2w"],
    }
}

fn deep(depth: usize) -> Lanes {
    let mut doc = new_doc(DEEP_CSS);
    let (mut parent, mut leaves) = (doc.body(), vec![]);
    for level in 0..depth {
        if level > 0 {
            parent = div(
                &mut doc,
                parent,
                if level.is_multiple_of(2) { "col" } else { "rw" },
            );
        }
        leaves.push(div(&mut doc, parent, "leaf"));
    }
    let view = Viewport {
        width: DEEP_SIDE,
        height: DEEP_SIDE,
        scale: 1.0,
    };
    Lanes {
        doc,
        view,
        changed: leaves[depth - 2],
        probes: [leaves[depth - 2], leaves[depth - 1]],
        flipped: false,
        classes: ["leaf", "big"],
    }
}

impl Engine for Lanes {
    const NAME: &'static str = "lanes";

    fn build(workload: Workload) -> Result<Self, Unsupported> {
        Ok(match workload {
            Workload::Table(rows) => table(rows),
            Workload::Deep(depth) => deep(depth),
        })
    }

    fn layout(&mut self) {
        self.doc
            .update(self.view)
            .unwrap_or_else(|e| panic!("update refused: {e:#?}"));
    }

    fn change_leaf(&mut self) -> Result<(), Unsupported> {
        self.flipped = !self.flipped;
        self.doc.set_attribute(
            self.changed,
            "class",
            self.classes[usize::from(self.flipped)],
        );
        Ok(())
    }

    fn probe(&self) -> [Rect; 2] {
        let painted = self.doc.painted().expect("laid out");
        self.probes.map(|n| {
            let b = painted.border_box(n).expect("a box");
            Rect {
                x: b.x,
                y: b.y,
                w: b.width,
                h: b.height,
            }
        })
    }
}
