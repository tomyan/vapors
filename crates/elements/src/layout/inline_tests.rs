use super::*;
use crate::style::Face;
use vapors_text::LineHeight;

fn style(weight: f32) -> TextStyle {
    TextStyle {
        face: Face::Geist,
        size: 16.0,
        weight,
        line_height: LineHeight::Normal,
        color: vapors_scene::Color::default(),
    }
}

/// Collects `pieces`: `Text` pushes text, `Open`/`Close` bracket an
/// element (closes match the latest open).
enum Piece<'a> {
    Text(&'a str),
    Open,
    Close(u64),
}

fn collect(pieces: &[Piece<'_>]) -> Option<InlineRun> {
    let mut collector = Collector::new(style(400.0), true);
    let mut open = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Text(text) => collector.push_text(text),
            Piece::Open => open.push((collector.open(), collector.position())),
            Piece::Close(id) => {
                let (slot, start) = open.pop().expect("an open element");
                let span = InlineSpan {
                    node: NodeId(*id),
                    range: start..collector.position(),
                    style: style(700.0),
                    visible: true,
                };
                collector.close(slot, span);
            }
        }
    }
    collector.finish()
}

fn ranges(run: &InlineRun) -> Vec<(u64, Range<usize>)> {
    run.spans
        .iter()
        .map(|s| (s.node.0, s.range.clone()))
        .collect()
}

#[test]
fn whitespace_collapses_across_an_element_boundary() {
    // When: `a <b> b</b> c`
    let run = collect(&[
        Piece::Text("a "),
        Piece::Open,
        Piece::Text(" b"),
        Piece::Close(1),
        Piece::Text("  c"),
    ])
    .expect("a run");

    // Then
    assert_eq!(run.text, "a b c");
    assert_eq!(ranges(&run), [(1, 2..3)]);
}

#[test]
fn leading_and_trailing_whitespace_is_dropped() {
    // When
    let run = collect(&[Piece::Text("\n   Hello,\t vapors \n")]).expect("a run");

    // Then
    assert_eq!(run.text, "Hello, vapors");
}

#[test]
fn nested_elements_are_listed_outermost_first() {
    // When: `<b>x <i>y</i></b>`
    let run = collect(&[
        Piece::Open,
        Piece::Text("x "),
        Piece::Open,
        Piece::Text("y"),
        Piece::Close(2),
        Piece::Close(1),
    ])
    .expect("a run");

    // Then
    assert_eq!(ranges(&run), [(1, 0..3), (2, 2..3)]);
}

#[test]
fn a_span_holding_the_trailing_space_ends_with_the_text() {
    // When: `a<b> </b>`
    let run = collect(&[
        Piece::Text("a"),
        Piece::Open,
        Piece::Text(" "),
        Piece::Close(1),
    ])
    .expect("a run");

    // Then
    assert_eq!(run.text, "a");
    assert_eq!(ranges(&run), [(1, 1..1)]);
}

#[test]
fn a_run_of_whitespace_only_is_no_run() {
    // Then
    assert_eq!(
        collect(&[
            Piece::Text(" \n "),
            Piece::Open,
            Piece::Text(" "),
            Piece::Close(1)
        ]),
        None
    );
}

#[test]
fn a_no_break_space_is_kept() {
    // When
    let run = collect(&[Piece::Text("a\u{a0}\u{a0}b")]).expect("a run");

    // Then
    assert_eq!(run.text, "a\u{a0}\u{a0}b");
}
