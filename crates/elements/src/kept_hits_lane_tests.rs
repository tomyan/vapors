use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 200.0,
    scale: 1.0,
};

/// A framed list of `rows` rows 20px tall under body, laid out; the
/// rows.
fn list(rows: usize) -> (Document, Vec<NodeId>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", "body { margin: 0; } .row { height: 20px; }")
        .expect("parses");
    let body = doc.body();
    let list = doc.create_element("div");
    doc.append(body, list);
    let rows = (0..rows)
        .map(|_| {
            let row = doc.create_element("div");
            doc.set_attribute(row, "class", "row");
            doc.append(list, row);
            row
        })
        .collect();
    (doc, rows)
}

/// The element hit at `point` in `rows` rows, and how many hits that
/// tested.
fn hit_in(rows: usize, point: [f32; 2]) -> (Option<NodeId>, usize, Vec<NodeId>) {
    let (mut doc, nodes) = list(rows);
    let painted = doc.update(VIEWPORT).expect("updates");
    let (hit, tested) = painted
        .kept_hits()
        .hit_counted(point, &painted.frames, &|_| None);
    (hit, tested, nodes)
}

#[test]
fn a_hit_in_a_framed_list_finds_its_row() {
    // When
    let (hit, _, rows) = hit_in(100, [10.0, 105.0]);

    // Then: the sixth row, 100 to 120px down
    assert_eq!(hit, Some(rows[5]));
}

#[test]
fn a_hit_in_a_framed_list_tests_the_rows_at_its_point_alone() {
    // When: a hit test in 100 rows and in 400
    let (_, small, _) = hit_in(100, [10.0, 105.0]);
    let (_, large, _) = hit_in(400, [10.0, 105.0]);

    // Then: as many hits tested at either size, and only the page
    // frame's (body, the list) and the rows the point's device pixel
    // meets, widened a device pixel each way (two), not the 10 rows the
    // view holds
    assert_eq!(small, large, "a hit test reads every row's hits");
    assert!(small <= 4, "a hit test read {small} hits");
}

/// A page styled `css` with `rows` rows (class `row`, role-less) in a
/// `div.list` under body, rows of index `special` given the extra class
/// named with them; `wrap` puts the list in a `div.wrap` when true.
fn page(css: &str, rows: usize, special: &[(usize, &str)], wrap: bool) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", css).expect("parses");
    let body = doc.body();
    let parent = if wrap {
        let wrap = doc.create_element("div");
        doc.set_attribute(wrap, "class", "wrap");
        doc.append(body, wrap);
        wrap
    } else {
        body
    };
    let list = doc.create_element("div");
    doc.set_attribute(list, "class", "list");
    doc.append(parent, list);
    for index in 0..rows {
        let row = doc.create_element("div");
        let extra = special.iter().find(|(at, _)| *at == index).map(|(_, c)| *c);
        doc.set_attribute(row, "class", &format!("row {}", extra.unwrap_or("")));
        doc.append(list, row);
        let cell = doc.create_element("div");
        doc.set_attribute(cell, "class", "cell");
        doc.append(row, cell);
    }
    doc
}

/// Hit tests `doc`, updated at `view`, at a grid of points `step` CSS px
/// apart down to `bottom`; the debug build holds each answer to testing
/// every hit (`assert_hit_as_scanned`). How many points hit something.
fn hit_grid(doc: &mut Document, view: Viewport, (step, bottom): (f32, f32)) -> usize {
    let painted = doc.update(view).expect("updates");
    let mut hits = 0;
    let mut y = 0.0;
    while y < bottom {
        for x in [2.0, 30.0, 120.0] {
            hits += usize::from(
                painted
                    .kept_hits()
                    .hit([x, y], &painted.frames, &|_| None)
                    .is_some(),
            );
        }
        y += step;
    }
    hits
}

/// A page of the hit test table: its name, its styles, the rows given an
/// extra class, and whether the list is wrapped.
type Case<'a> = (&'a str, String, Vec<(usize, &'a str)>, bool);

#[test]
fn hit_tests_through_the_frames_at_a_point_answer_as_testing_every_hit() {
    const BASE: &str = "body { margin: 0; } .row { height: 20px; } \
        .cell { width: 40px; height: 20px; background-color: red; }";
    let cases: [Case<'_>; 4] = [
        (
            "overlapping rows",
            format!(
                "{BASE} .tall .cell {{ height: 34px; }} .up {{ margin-top: -9px; }} \
                     .moved {{ position: relative; top: 7px; }}"
            ),
            vec![(5, "tall"), (8, "up"), (12, "moved")],
            false,
        ),
        (
            "a scroll container in a row",
            format!("{BASE} .scrolls .cell {{ overflow: auto; }}"),
            vec![(3, "scrolls")],
            false,
        ),
        (
            "a list in a scrolled container",
            format!("{BASE} .wrap {{ height: 150px; overflow: auto; }}"),
            vec![],
            true,
        ),
        (
            "a fractional scale at a clip edge",
            format!("{BASE} .wrap {{ margin-top: 3.3px; height: 101.3px; overflow: hidden; }}"),
            vec![],
            true,
        ),
    ];
    for (name, css, special, wrap) in cases {
        // Given
        let mut doc = page(&css, 60, &special, wrap);
        let scale = if name.starts_with("a fractional") {
            1.5
        } else {
            1.0
        };
        let view = Viewport { scale, ..VIEWPORT };
        if name == "a list in a scrolled container" {
            doc.update(view).expect("updates");
            let wrap = doc.children(doc.body())[0];
            doc.scroll_to(wrap, [0.0, 37.0]);
        }

        // When: hit tested every 0.75px down the view
        let hits = hit_grid(&mut doc, view, (0.75, VIEWPORT.height));

        // Then: each answer held to the full scan (debug), and the page
        // hit somewhere
        assert!(hits > 0, "{name}: nothing hit");
    }
}
