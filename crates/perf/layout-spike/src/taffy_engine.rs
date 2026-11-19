//! Option (b): Taffy 0.15.1 on its own, a node per box, the leaves with
//! fixed sizes. The tree build is outside the timed first layout.

use taffy::prelude::*;

use crate::spec::{CHANGED_W, COLS, ROW_H, VIEW_W, middle};
use crate::{Engine, Rect, Unsupported, Workload};

/// A tree, its root and the two probed nodes with their paths from the root.
pub struct Taffy {
    pub(crate) tree: TaffyTree<()>,
    pub(crate) root: NodeId,
    pub(crate) changed: NodeId,
    pub(crate) paths: [Vec<NodeId>; 2],
    pub(crate) flipped: bool,
    pub(crate) sizes: [Size<Dimension>; 2],
}

pub(crate) fn fixed(w: f32, h: f32) -> Style {
    Style {
        size: Size {
            width: length(w),
            height: length(h),
        },
        flex_shrink: 0.0,
        ..Style::default()
    }
}

pub(crate) fn container(direction: FlexDirection, size: Size<Dimension>) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: direction,
        size,
        ..Style::default()
    }
}

pub(crate) fn rest() -> Style {
    Style {
        flex_grow: 1.0,
        flex_basis: length(0.0),
        ..Style::default()
    }
}

fn row(tree: &mut TaffyTree<()>) -> (NodeId, Vec<NodeId>) {
    let widths = COLS.map(|w| fixed(w, ROW_H));
    let mut cells: Vec<NodeId> = widths
        .into_iter()
        .map(|s| tree.new_leaf(s).expect("leaf"))
        .collect();
    let last = Style {
        size: Size {
            width: auto(),
            height: length(ROW_H),
        },
        ..rest()
    };
    cells.push(tree.new_leaf(last).expect("leaf"));
    let size = Size {
        width: auto(),
        height: length(ROW_H),
    };
    let node = tree
        .new_with_children(container(FlexDirection::Row, size), &cells)
        .expect("row");
    (node, cells)
}

fn table(rows: usize) -> Taffy {
    let mut tree = TaffyTree::new();
    let built: Vec<_> = (0..rows).map(|_| row(&mut tree)).collect();
    let kids: Vec<NodeId> = built.iter().map(|b| b.0).collect();
    let size = Size {
        width: length(VIEW_W),
        height: auto(),
    };
    let root = tree
        .new_with_children(container(FlexDirection::Column, size), &kids)
        .expect("root");
    let (mid_row, cells) = &built[middle(rows)];
    let paths = [
        vec![root, *mid_row, cells[1]],
        vec![root, *mid_row, cells[4]],
    ];
    let sizes = [fixed(COLS[1], ROW_H).size, fixed(CHANGED_W, ROW_H).size];
    Taffy {
        tree,
        root,
        changed: cells[1],
        paths,
        flipped: false,
        sizes,
    }
}

impl Taffy {
    fn rect(&self, path: &[NodeId]) -> Rect {
        let at = |n: &NodeId| self.tree.layout(*n).expect("laid out");
        let (x, y) = path.iter().fold((0.0, 0.0), |(x, y), n| {
            (x + at(n).location.x, y + at(n).location.y)
        });
        let last = at(path.last().expect("a path"));
        Rect {
            x,
            y,
            w: last.size.width,
            h: last.size.height,
        }
    }
}

impl Engine for Taffy {
    const NAME: &'static str = "taffy";

    fn build(workload: Workload) -> Result<Self, Unsupported> {
        Ok(match workload {
            Workload::Table(rows) => table(rows),
            Workload::Deep(depth) => crate::taffy_deep::deep(depth),
        })
    }

    fn layout(&mut self) {
        self.tree
            .compute_layout(self.root, Size::max_content())
            .expect("layout");
    }

    fn change_leaf(&mut self) -> Result<(), Unsupported> {
        self.flipped = !self.flipped;
        let mut style = self.tree.style(self.changed).expect("style").clone();
        style.size = self.sizes[usize::from(self.flipped)];
        self.tree.set_style(self.changed, style).expect("set");
        Ok(())
    }

    fn probe(&self) -> [Rect; 2] {
        [self.rect(&self.paths[0]), self.rect(&self.paths[1])]
    }
}
