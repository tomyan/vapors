//! Shared by the paint tests: fixtures laid out by vapors, Chromium's
//! saved rendering of them, and the regions the rules hold to their
//! budgets.

#[path = "../corpus_support/images.rs"]
mod images;

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::{Document, Viewport};
use vapors_elements_corpus::paint_check::{Image, Region};
use vapors_elements_corpus::paint_reference::{is_paint, saved};
use vapors_elements_corpus::{Fixture, all};
use vapors_text::FontSystem;
use vapors_text::comparison::Rgba;

pub use vapors_elements_corpus::paint_reference::{HEIGHT, WIDTH};

pub fn paint_fixtures() -> Vec<Fixture> {
    let fixtures: Vec<Fixture> = all().into_iter().filter(is_paint).collect();
    assert!(
        !fixtures.is_empty(),
        "no paint fixtures: nothing is compared"
    );
    fixtures
}

/// The fixture laid out at `scale`.
pub fn laid_out(fixture: &Fixture, scale: u32) -> Rc<RefCell<Document>> {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    // the mount registers the fixture's sheet itself
    let body = doc.borrow().body();
    (fixture.mount)(&doc, body);
    images::register_fixture_images(&mut doc.borrow_mut());
    #[allow(clippy::cast_precision_loss)]
    let viewport = Viewport {
        width: WIDTH as f32,
        height: HEIGHT as f32,
        scale: scale as f32,
    };
    if let Err(errors) = doc.borrow_mut().update(viewport) {
        panic!("{}: refused: {errors:#?}", fixture.name);
    }
    doc
}

/// Each element under `body`: its border box in device pixels, one
/// pixel larger all round.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn regions(doc: &Document, scale: u32) -> Vec<Region> {
    let painted = doc.painted().expect("painted");
    let s = scale as f32;
    doc.elements()
        .into_iter()
        .skip(2)
        .filter_map(|node| {
            let r = painted.border_box(node)?;
            let class = doc.attribute(node, "class").unwrap_or_default();
            let device = |v: f32| (v * s).round().max(0.0) as u32;
            let rect = [
                device(r.x).saturating_sub(1),
                device(r.y).saturating_sub(1),
                device(r.x + r.width) + 1,
                device(r.y + r.height) + 1,
            ];
            Some(Region {
                label: format!(".{class}"),
                rect,
            })
        })
        .collect()
}

/// Chromium's rendering of paint fixture `name` in `state` (`None` as
/// it is) at `scale`: its committed saved reference, read without
/// starting a browser (`vapors_elements_corpus::paint_reference`).
pub fn theirs(name: &str, state: Option<&str>, scale: u32) -> Rgba {
    saved(name, state, scale).unwrap_or_else(|why| panic!("{why}"))
}

pub fn image(rgba: &Rgba) -> Image<'_> {
    Image {
        width: rgba.width,
        height: rgba.height,
        rgba: &rgba.data,
    }
}
