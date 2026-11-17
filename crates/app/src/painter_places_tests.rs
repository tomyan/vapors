use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_scene::Rect;
use vapors_text::FontSystem;

use super::*;

const VIEWPORT: Viewport = Viewport {
    width: 10.0,
    height: 10.0,
    scale: 1.0,
};

const VIEW: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 10.0,
    bottom: 10.0,
};

/// A page of 40 coloured rows, the second with a 2 px margin above it,
/// framed (a lane is framed from its first written update) and updated.
fn rows_page() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } div { height: 2px; background-color: #f00; } \
        .above { margin-top: 2px; } .below { margin-bottom: 2px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    for at in 0..40 {
        let div = doc.create_element("div");
        doc.set_attribute(div, "class", if at == 1 { "above" } else { "" });
        doc.append(doc.body(), div);
    }
    doc.update(VIEWPORT).expect("updates");
    let last = doc.create_element("div");
    doc.append(doc.body(), last);
    doc.update(VIEWPORT).expect("updates");
    doc
}

/// `places` worked out for each of `doc`'s batches at the view.
fn made(doc: &Document, places: &mut BatchPlaces) {
    let painted = doc.painted().expect("painted");
    let kept = painted.kept_batches(VIEW);
    places.reset((painted, VIEW), &kept);
    for at in 0..kept.frames.len() {
        places.frame(at, painted, &kept);
        places.rows(at, (painted, &kept));
    }
}

/// Whether `places` are current for `doc`'s batches at `view`.
fn current(doc: &Document, places: &BatchPlaces, view: Rect) -> bool {
    let painted = doc.painted().expect("painted");
    places.current(painted, view, &painted.kept_batches(view))
}

#[test]
fn places_stay_current_while_nothing_is_painted() {
    // Given
    let mut places = BatchPlaces::default();
    let doc = rows_page();
    made(&doc, &mut places);

    // When / Then: the next frame reads them
    assert!(current(&doc, &places, VIEW));
}

/// The keys of `doc`'s batches at `view`.
fn keys(doc: &Document, view: Rect) -> Vec<vapors_elements::BatchKey> {
    doc.painted()
        .expect("painted")
        .kept_batches(view)
        .keys
        .clone()
}

#[test]
fn places_are_made_again_for_another_view_with_the_same_batches() {
    // Given
    let mut places = BatchPlaces::default();
    let doc = rows_page();
    made(&doc, &mut places);
    let before = keys(&doc, VIEW);

    // When: a view a pixel narrower, over the same rows
    let narrower = Rect::new(0.0, 0.0, 9.0, 10.0);

    // Then
    assert_eq!(
        keys(&doc, narrower),
        before,
        "other batches: the case is not reached"
    );
    assert!(!current(&doc, &places, narrower));
}

#[test]
fn places_are_made_again_after_a_paint_that_keeps_the_batches() {
    // Given
    let mut places = BatchPlaces::default();
    let mut doc = rows_page();
    made(&doc, &mut places);
    let before = keys(&doc, VIEW);

    // When: the second row's margin moves below it, the batches' runs
    // and rows the same, the rows' offsets not
    let second = doc.children(doc.body())[1];
    doc.set_attribute(second, "class", "below");
    doc.update(VIEWPORT).expect("updates");

    // Then
    assert_eq!(
        keys(&doc, VIEW),
        before,
        "other batches: the case is not reached"
    );
    assert!(!current(&doc, &places, VIEW));
}

#[test]
fn places_made_for_more_batches_than_are_drawn_are_not_current() {
    // Given
    let mut places = BatchPlaces::default();
    let doc = rows_page();
    made(&doc, &mut places);

    // When: the batches drawn under the same paint and view are fewer
    let painted = doc.painted().expect("painted");
    let mut fewer = painted.kept_batches(VIEW).clone();
    fewer.frames.pop();

    // Then
    assert!(!places.current(painted, VIEW, &fewer));
}

#[test]
#[should_panic(expected = "the batches were made again with no new paint")]
fn places_kept_for_other_batches_are_refused() {
    // Given
    let mut places = BatchPlaces::default();
    let doc = rows_page();
    made(&doc, &mut places);

    // When: as many batches drawn under the same paint and view, keyed
    // otherwise
    let painted = doc.painted().expect("painted");
    let mut other = painted.kept_batches(VIEW).clone();
    let elsewhere = rows_page();
    other.keys[0] = elsewhere
        .painted()
        .expect("painted")
        .kept_batches(VIEW)
        .keys[0];

    // Then
    places.assert_current(painted, &other);
}
