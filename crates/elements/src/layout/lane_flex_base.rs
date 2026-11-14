//! A lane column item's flex base size and hypothetical height,
//! mirrored from taffy 0.15.1 `compute/flexbox.rs`
//! (`determine_flex_base_size` 947-1231) for one item of a column of
//! known width; `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::{MaybeMath, MaybeResolve};
use taffy::{
    AbsoluteAxis, AlignItems, AvailableSpace, LayoutInput, Line, NodeId, RequestedAxis, RunMode,
    Size, SizingMode,
};

use super::lane_flex::Column;
use super::lane_flex_item::FlexChild;
use super::tree::BoxTree;

/// The item's sizes along the column before the flexible lengths.
#[derive(Clone, Copy, Debug)]
pub(super) struct Base {
    pub(super) flex_basis: f32,
    pub(super) inner_flex_basis: f32,
    pub(super) flex_basis_is_definite: bool,
    pub(super) resolved_minimum_main_size: f32,
    pub(super) hypothetical_inner: f32,
    pub(super) hypothetical_outer: f32,
}

/// What the item is measured with along the column: its width when
/// known or stretched, the space across, and its parent size.
struct Measuring {
    known: Size<Option<f32>>,
    cross_size_is_definite: bool,
    cross_space: AvailableSpace,
    parent_size: Size<Option<f32>>,
}

impl BoxTree<'_> {
    /// The item at `id`'s base size, minimum and hypothetical height
    /// (955-1210).
    pub(super) fn flex_base(&mut self, id: NodeId, column: &Column, child: &FlexChild) -> Base {
        let measuring = measuring(column, child);
        let (flex_basis, definite) = self.flex_basis(id, column, child, &measuring);
        let padding_border = child.padding.vertical_axis_sum() + child.border.vertical_axis_sum();
        let flex_basis = flex_basis.max(padding_border);
        let resolved_min = self.resolved_minimum(id, child, &measuring);
        let transferred = transferred(child);
        let floor = resolved_min
            .maybe_max(transferred.0.height)
            .maybe_max(Some(padding_border));
        let hypothetical_inner = flex_basis.maybe_clamp(Some(floor), transferred.1.height);
        Base {
            flex_basis,
            inner_flex_basis: flex_basis - padding_border,
            flex_basis_is_definite: definite,
            resolved_minimum_main_size: resolved_min,
            hypothetical_inner,
            hypothetical_outer: hypothetical_inner + child.margin.vertical_axis_sum(),
        }
    }

    /// The flex base size and whether it is definite (1026-1146): the
    /// `flex-basis`, else a definite height under `flex-basis: auto`,
    /// else the height from the width by the aspect ratio, else the
    /// item measured.
    fn flex_basis(
        &mut self,
        id: NodeId,
        column: &Column,
        child: &FlexChild,
        measuring: &Measuring,
    ) -> (f32, bool) {
        if let Some(basis) = styled_basis(self, column, child) {
            return (basis, true);
        }
        if measuring.cross_size_is_definite
            && let (Some(ratio), Some(width)) = (child.aspect_ratio, measuring.known.width)
        {
            return (width / ratio, true);
        }
        let along = match column.available_space.height {
            AvailableSpace::MinContent => AvailableSpace::MinContent,
            _ => AvailableSpace::MaxContent,
        };
        (self.measure_height(id, measuring, along), false)
    }

    /// The minimum height (1172-1197): the styled one, else 0 for an
    /// item that scrolls along the column, else the min-content height,
    /// at most the item's height and transferred maximum, at least its
    /// padding and border.
    fn resolved_minimum(&mut self, id: NodeId, child: &FlexChild, measuring: &Measuring) -> f32 {
        let scrolls_along = child.overflow.y.is_scroll_container();
        let styled = child.min_size.height.or(scrolls_along.then_some(0.0));
        if let Some(min) = styled {
            return min;
        }
        let padding_border = (child.padding + child.border).sum_axes().height;
        self.measure_height(id, measuring, AvailableSpace::MinContent)
            .maybe_min(child.size.height)
            .maybe_min(transferred(child).1.height)
            .maybe_max(Some(padding_border))
    }

    /// The item's height as `measure_child_size` asks it (394-422).
    fn measure_height(&mut self, id: NodeId, measuring: &Measuring, along: AvailableSpace) -> f32 {
        let inputs = LayoutInput {
            known_dimensions: measuring.known,
            known_dimensions_are_definite: Size {
                width: true,
                height: true,
            },
            parent_size: measuring.parent_size,
            available_space: Size {
                width: measuring.cross_space,
                height: along,
            },
            sizing_mode: SizingMode::ContentSize,
            axis: RequestedAxis::from(AbsoluteAxis::Vertical),
            run_mode: RunMode::ComputeSize,
            vertical_margins_are_collapsible: Line::FALSE,
        };
        self.lay_out(id, inputs, None).size.height
    }
}

/// The `flex-basis` resolved, or under `flex-basis: auto` a definite
/// height (1019-1024, 1063-1066).
fn styled_basis(tree: &BoxTree<'_>, column: &Column, child: &FlexChild) -> Option<f32> {
    let percent_main = column
        .node_inner_size
        .height
        .filter(|_| column.known_main_size_is_definite());
    let calc = |val, basis| tree.calcs.resolve(val, basis);
    let basis_style = child.flex_basis_style;
    let flex_basis = basis_style
        .maybe_resolve(percent_main, calc)
        .maybe_add(child.box_sizing_adjustment.height);
    flex_basis.or(child.size.height.filter(|_| basis_style.is_auto()))
}

/// What the item is measured with (959-1004): the width across the
/// column, its own clamped, or stretched to the space across.
fn measuring(column: &Column, child: &FlexChild) -> Measuring {
    let (min, max) = transferred(child);
    let cross_parent = column.node_inner_size.width;
    let cross_space = cross_space(column, child, (min.width, max.width));
    let mut known = Size {
        height: None,
        ..child.size
    };
    known.width = known.width.maybe_clamp(min.width, max.width);
    let mut cross_size_is_definite = child.size.width.is_some();
    if child.align_self == AlignItems::STRETCH
        && !child.margin_is_auto.left
        && !child.margin_is_auto.right
        && known.width.is_none()
    {
        known.width = cross_space.into_option();
        cross_size_is_definite = column.has_definite_cross_size() && cross_parent.is_some();
    }
    Measuring {
        known,
        cross_size_is_definite,
        cross_space,
        parent_size: Size {
            width: cross_parent,
            height: None,
        },
    }
}

/// The space across for the item (971-987): the column's inner width
/// less the item's margins, clamped; the column's width is known, so
/// the space across is definite.
fn cross_space(
    column: &Column,
    child: &FlexChild,
    (min, max): (Option<f32>, Option<f32>),
) -> AvailableSpace {
    match column.available_space.width {
        AvailableSpace::Definite(space) => AvailableSpace::Definite(
            (column.node_inner_size.width.unwrap_or(space) - child.margin.horizontal_axis_sum())
                .max(0.0)
                .maybe_clamp(min, max),
        ),
        AvailableSpace::MinContent => {
            min.map_or(AvailableSpace::MinContent, AvailableSpace::Definite)
        }
        AvailableSpace::MaxContent => {
            max.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite)
        }
    }
}

/// The item's minimum and maximum with its aspect ratio applied
/// (965-966).
pub(super) fn transferred(child: &FlexChild) -> (Size<Option<f32>>, Size<Option<f32>>) {
    (
        child.min_size.maybe_apply_aspect_ratio(child.aspect_ratio),
        child.max_size.maybe_apply_aspect_ratio(child.aspect_ratio),
    )
}
