//! Option (d), a hand-written prototype of per-component generated
//! code: the structure of each tree is in the code (no style or node
//! lookups, no generic flex algorithm), the sizes are runtime values
//! the component's state can change, and each change marks the part of
//! the tree it can reach: a table's row, or a deep tree's levels from
//! the changed one down (the levels above lie outside what a leaf
//! changes, since the sizes come down from the root).

use crate::spec::{
    CHANGED_LEAF, CHANGED_W, COLS, DEEP_SIDE, LEAF, PAD, ROW_H, VIEW_W, middle, row_y,
};
use crate::{Engine, Rect, Unsupported, Workload};

enum Shape {
    /// Runtime widths of the first four cells per row, and dirty rows.
    Table {
        widths: Vec<[f32; 4]>,
        dirty: Vec<bool>,
        boxes: Vec<[Rect; 5]>,
    },
    /// The leaf side per level, the container boxes and leaf boxes, and
    /// the first level to lay out again.
    Deep {
        sides: Vec<f32>,
        from: usize,
        containers: Vec<Rect>,
        leaves: Vec<Rect>,
    },
}

pub struct Generated {
    shape: Shape,
    flipped: bool,
}

fn row_boxes(index: usize, widths: [f32; 4]) -> [Rect; 5] {
    let y = row_y(index);
    let mut x = 0.0;
    let mut out = [Rect::default(); 5];
    for (cell, w) in widths.into_iter().enumerate() {
        out[cell] = Rect { x, y, w, h: ROW_H };
        x += w;
    }
    out[4] = Rect {
        x,
        y,
        w: VIEW_W - x,
        h: ROW_H,
    };
    out
}

/// Level `level`'s leaf and the container of the level below, from its own box.
fn level_boxes(level: usize, own: Rect, side: f32) -> (Rect, Rect) {
    let (cx, cy, cw, ch) = (
        own.x + PAD,
        own.y + PAD,
        own.w - 2.0 * PAD,
        own.h - 2.0 * PAD,
    );
    let leaf = Rect {
        x: cx,
        y: cy,
        w: side,
        h: side,
    };
    let nested = if level.is_multiple_of(2) {
        Rect {
            x: cx,
            y: cy + side,
            w: cw,
            h: ch - side,
        }
    } else {
        Rect {
            x: cx + side,
            y: cy,
            w: cw - side,
            h: ch,
        }
    };
    (leaf, nested)
}

impl Generated {
    fn lay_out_levels(sides: &[f32], from: usize, containers: &mut [Rect], leaves: &mut [Rect]) {
        for level in from..sides.len() {
            let (leaf, nested) = level_boxes(level, containers[level], sides[level]);
            leaves[level] = leaf;
            if let Some(next) = containers.get_mut(level + 1) {
                *next = nested;
            }
        }
    }
}

impl Engine for Generated {
    const NAME: &'static str = "generated per component";

    fn build(workload: Workload) -> Result<Self, Unsupported> {
        let shape = match workload {
            Workload::Table(rows) => Shape::Table {
                widths: vec![COLS; rows],
                dirty: vec![true; rows],
                boxes: vec![[Rect::default(); 5]; rows],
            },
            Workload::Deep(depth) => {
                let mut containers = vec![Rect::default(); depth];
                containers[0] = Rect {
                    x: 0.0,
                    y: 0.0,
                    w: DEEP_SIDE,
                    h: DEEP_SIDE,
                };
                Shape::Deep {
                    sides: vec![LEAF; depth],
                    from: 0,
                    containers,
                    leaves: vec![Rect::default(); depth],
                }
            }
        };
        Ok(Self {
            shape,
            flipped: false,
        })
    }

    fn layout(&mut self) {
        match &mut self.shape {
            Shape::Table {
                widths,
                dirty,
                boxes,
            } => {
                for (index, row) in dirty.iter_mut().enumerate().filter(|(_, d)| **d) {
                    boxes[index] = row_boxes(index, widths[index]);
                    *row = false;
                }
            }
            Shape::Deep {
                sides,
                from,
                containers,
                leaves,
            } => {
                Self::lay_out_levels(sides, *from, containers, leaves);
                *from = sides.len();
            }
        }
    }

    fn change_leaf(&mut self) -> Result<(), Unsupported> {
        self.flipped = !self.flipped;
        match &mut self.shape {
            Shape::Table { widths, dirty, .. } => {
                let mid = middle(widths.len());
                widths[mid][1] = if self.flipped { CHANGED_W } else { COLS[1] };
                dirty[mid] = true;
            }
            Shape::Deep { sides, from, .. } => {
                let level = sides.len() - 2;
                sides[level] = if self.flipped { CHANGED_LEAF } else { LEAF };
                *from = level;
            }
        }
        Ok(())
    }

    fn probe(&self) -> [Rect; 2] {
        match &self.shape {
            Shape::Table { boxes, .. } => {
                let row = &boxes[middle(boxes.len())];
                [row[1], row[4]]
            }
            Shape::Deep { leaves, .. } => [leaves[leaves.len() - 2], leaves[leaves.len() - 1]],
        }
    }
}
