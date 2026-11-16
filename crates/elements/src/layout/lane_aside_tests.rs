use taffy::{AvailableSpace, LayoutInput, NodeId, RunMode, Size};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::layout::lane_page::{VIEWPORT, element};
use crate::layout::tree::BoxTree;
use crate::layout::{LayoutBox, Written};

/// A `list` of `rows` rows under body, styled by `css`, laid out once
/// with its lanes.
fn laid_out(css: &str, rows: usize) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let list = element(&mut doc, body, "list", None);
    for _ in 0..rows {
        element(&mut doc, list, "row", None);
    }
    doc.add_stylesheet("page.css", css).expect("parses");
    doc.update(VIEWPORT).expect("updates");
    doc
}

/// `ask` run on the box tree `doc` laid out, its lanes on, with the
/// slot of the lane container holding `rows` children.
fn asked(doc: &mut Document, rows: usize, ask: impl FnOnce(&mut BoxTree<'_>, usize)) {
    let kept = doc.kept.as_ref().expect("updated");
    let placed = std::mem::take(&mut doc.boxes.placed);
    let mut lanes = placed.lanes.borrow_mut();
    let list = lanes.container_of_children(rows);
    let mut tree = BoxTree {
        boxes: placed.boxes,
        fonts: &mut doc.fonts,
        cache: &mut doc.texts,
        scale: VIEWPORT.scale,
        calcs: &kept.calcs,
        written: Written::default(),
        calls: 0,
        disowned: 0,
        lanes: &mut lanes,
        lanes_enabled: true,
        stale: rustc_hash::FxHashSet::default(),
        lane_counts: crate::layout::lane_layout::LaneCounts::default(),
    };
    ask(&mut tree, list);
}

/// The content-size query the list was asked before its layout.
fn content_query(boxes: &[LayoutBox], slot: usize) -> LayoutInput {
    let asked = boxes[slot].asked.iter().map(|ask| ask.input);
    asked
        .filter(|input| input.run_mode == RunMode::ComputeSize)
        .find(|input| input.known_dimensions.height.is_none())
        .expect("the column asks the list its content size")
}

#[test]
fn a_size_query_with_other_height_bases_leaves_the_written_starts() {
    // Given: a list laid out at 100px whose rows' percentages read it,
    // in a flex column that asks it its content size first
    let css = "body { margin: 0; display: flex; flex-direction: column; height: 200px; } \
               .list { display: flex; flex-direction: column; height: 100px; flex-shrink: 0; } \
               .row { min-height: 10%; flex-shrink: 0; }";
    let mut doc = laid_out(css, 50);

    asked(&mut doc, 50, |tree, list| {
        // When: the content-size query asked again after the layout, as
        // when Taffy then serves the layout from its cache
        let query = content_query(&tree.boxes, list);
        tree.lane_or_flex(NodeId::from(list), query);

        // Then: the lane's starts are the ones the rows were laid out at
        tree.lanes.assert_starts_written();
    });
}

#[test]
fn a_size_query_the_lane_declines_keeps_the_lane() {
    // Given: a list laid out by its lane
    let css = "body { margin: 0; } .row { height: 20px; }";
    let mut doc = laid_out(css, 40);

    asked(&mut doc, 40, |tree, list| {
        // When: a max-content query, which a lane declines (no width)
        let query = LayoutInput {
            run_mode: RunMode::ComputeSize,
            known_dimensions: Size::NONE,
            available_space: Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
            ..LayoutInput::HIDDEN
        };
        tree.lane_or_block(NodeId::from(list), query, None);

        // Then: the lane is kept, as written; Taffy wrote no row
        assert!(
            tree.lanes.block[&list].written.is_some(),
            "the lane was dropped"
        );
        tree.lanes.assert_starts_written();
    });
}

#[test]
fn a_lane_a_size_query_made_is_not_framed_until_it_writes() {
    // Given: the list's lane gone, as after a declined layout
    let css = "body { margin: 0; display: flex; flex-direction: column; height: 200px; } \
               .list { display: flex; flex-direction: column; height: 100px; flex-shrink: 0; } \
               .row { height: 20px; flex-shrink: 0; }";
    let mut doc = laid_out(css, 50);

    asked(&mut doc, 50, |tree, list| {
        tree.lanes.remove(list, &mut tree.boxes);

        // When: a size query makes it again, writing no row
        let query = content_query(&tree.boxes, list);
        tree.lane_or_flex(NodeId::from(list), query);

        // Then: its rows hold Taffy's layouts, not its starts: neither
        // framed nor read with a start added
        assert_eq!(tree.lanes.framing(list, 0.0, 1.0), None);
        let row = tree.boxes[list].children[10];
        let layout = tree.boxes[row].layout;
        assert_eq!(tree.lanes.taffy_location(row, &layout), layout.location);
    });
}

#[test]
fn a_size_query_a_child_declines_puts_the_starts_back() {
    // Given: a list its lane wrote, in a flex column that asks it its
    // content size, and each row where Taffy puts it
    let css = "body { margin: 0; display: flex; flex-direction: column; height: 200px; } \
               .list { display: flex; flex-direction: column; height: 100px; flex-shrink: 0; } \
               .row { height: 20px; flex-shrink: 0; }";
    let mut doc = laid_out(css, 50);

    asked(&mut doc, 50, |tree, list| {
        let rows = tree.boxes[list].children.clone();
        let taffy = |tree: &BoxTree<'_>, row: usize| {
            tree.lanes.taffy_location(row, &tree.boxes[row].layout)
        };
        let before: Vec<_> = rows.iter().map(|&row| taffy(tree, row)).collect();

        // When: a row taken out of flow, so the content-size query, the
        // lane laying it out again, declines it (`OutOfFlowChild`)
        let out = rows[5];
        tree.boxes[out].style.position = taffy::Position::Absolute;
        tree.stale.insert(out);
        let query = content_query(&tree.boxes, list);
        tree.lane_or_flex(NodeId::from(list), query);

        // Then: the lane is gone and every other row holds where Taffy
        // put it, as a layout served from Taffy's cache leaves it
        assert!(!tree.lanes.flex.contains_key(&list), "the lane was kept");
        for (&row, location) in rows.iter().zip(&before) {
            if row != out {
                assert_eq!(tree.boxes[row].layout.location, *location, "row {row}");
            }
        }
    });
}

#[test]
#[should_panic(expected = "past the")]
fn restoring_starts_into_too_few_boxes_is_refused() {
    // Given: a list its lane wrote
    let css = "body { margin: 0; } .row { height: 20px; }";
    let mut doc = laid_out(css, 40);

    asked(&mut doc, 40, |tree, list| {
        // When: its starts put back into boxes that end before its rows
        let lane = tree.lanes.lane(list).expect("a lane");
        lane.restore_starts(list, &mut tree.boxes[..2]);
    });
}
