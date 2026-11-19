//! Option (c), a hand-written prototype of what the compiler would emit
//! for a table whose shape is static: four fixed column widths, a fixed
//! row height and a last column that takes the rest. The compiler
//! resolves every column offset and width, and the row's y as `i * ROW_H`;
//! only the viewport width is left to run time, as one subtraction.
//! No tree, no style, no cache: a pass writing the boxes.
//!
//! Such a subtree is a boundary: nothing inside it changes a box, so a
//! leaf's declared size changing is a compile error in the component,
//! not a relayout.

use crate::spec::{COLS, ROW_H, VIEW_W, middle, row_y};
use crate::{Engine, Rect, Unsupported, Workload};

/// Column left edges and widths, resolved at compile time.
const X: [f32; 5] = [
    0.0,
    COLS[0],
    COLS[0] + COLS[1],
    COLS[0] + COLS[1] + COLS[2],
    COLS[0] + COLS[1] + COLS[2] + COLS[3],
];
const W: [f32; 4] = COLS;

pub struct Fixed {
    rows: usize,
    boxes: Vec<[Rect; 5]>,
}

/// What the compiler emits for one row: constants and the one runtime width.
fn resolve_row(index: usize, table_w: f32) -> [Rect; 5] {
    let y = row_y(index);
    let at = |i: usize, w: f32| Rect {
        x: X[i],
        y,
        w,
        h: ROW_H,
    };
    [
        at(0, W[0]),
        at(1, W[1]),
        at(2, W[2]),
        at(3, W[3]),
        at(4, table_w - X[4]),
    ]
}

impl Engine for Fixed {
    const NAME: &'static str = "compile-time boundary";

    fn build(workload: Workload) -> Result<Self, Unsupported> {
        match workload {
            Workload::Table(rows) => Ok(Self {
                rows,
                boxes: vec![[Rect::default(); 5]; rows],
            }),
            Workload::Deep(_) => Err(Unsupported(format!(
                "{workload:?} is not static: its boxes follow flex-grow down {} levels, so the \
                 compiler cannot make it a boundary; measure Table instead: \
                 cargo run --profile timing --features all --bin bench",
                crate::spec::DEEP_DEPTH
            ))),
        }
    }

    fn layout(&mut self) {
        for (index, row) in self.boxes.iter_mut().enumerate() {
            *row = resolve_row(index, VIEW_W);
        }
    }

    fn change_leaf(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported(
            "a boundary's leaf sizes are declared constants; a size change there is refused when \
             the component compiles, so there is no relayout to time"
                .into(),
        ))
    }

    fn probe(&self) -> [Rect; 2] {
        let row = &self.boxes[middle(self.rows)];
        [row[1], row[4]]
    }
}
