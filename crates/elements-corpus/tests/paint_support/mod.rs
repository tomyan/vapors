//! Shared by the paint tests: fixtures laid out by vapors, rendered by
//! Chromium, and the regions the rules hold to their budgets.

#[path = "../corpus_support/images.rs"]
mod images;

use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;

use vapors_elements::{Document, Viewport};
use vapors_elements_corpus::oracle::{fixtures_dir, write_fonts};
use vapors_elements_corpus::page::{html, split};
use vapors_elements_corpus::paint_check::{Image, Region};
use vapors_elements_corpus::{Fixture, all};
use vapors_text::FontSystem;
use vapors_text::comparison::{Chromium, Rgba};

pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 400;

pub fn paint_fixtures() -> Vec<Fixture> {
    let fixtures: Vec<Fixture> = all()
        .into_iter()
        .filter(|f| f.style.contains("/* paint"))
        .collect();
    assert!(
        !fixtures.is_empty(),
        "no paint fixtures: nothing is compared"
    );
    fixtures
}

/// The fixture laid out at `scale`.
pub fn laid_out(fixture: &Fixture, scale: u32) -> Rc<RefCell<Document>> {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    doc.borrow_mut()
        .add_stylesheet(fixture.name, fixture.style)
        .expect("parses");
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

/// Chromium's rendering of fixture `name` at `scale`, with `changed`
/// CSS appended to its style (empty for the fixture as it is). Its
/// files go in a directory of the test's own, named by `test`: tests
/// run in parallel, and one rewriting a page or screenshot another is
/// reading fails that one.
pub fn chromiums(test: &str, name: &str, scale: u32, changed: &str) -> Rgba {
    let label = if changed.is_empty() {
        "as-is".to_owned()
    } else {
        let mut hasher = DefaultHasher::new();
        changed.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("paint-{test}-{scale}x"));
    std::fs::create_dir_all(&work).expect("work dir");
    write_fonts(&work).expect("fonts");
    let path = fixtures_dir().join(format!("{name}.wisp"));
    let mut source =
        split(name, &std::fs::read_to_string(&path).expect("fixture")).expect("splits");
    source.style.push_str(changed);
    let page = work.join(format!("{name}_{label}.html"));
    std::fs::write(&page, html(&source)).expect("page");
    let chrome = Chromium::find().unwrap_or_else(|e| panic!("{e}"));
    let png = work.join(format!("{name}_{label}.png"));
    chrome
        .screenshot(&page, &png, WIDTH, HEIGHT, scale)
        .unwrap_or_else(|e| panic!("{e}"))
}

pub fn image(rgba: &Rgba) -> Image<'_> {
    Image {
        width: rgba.width,
        height: rgba.height,
        rgba: &rgba.data,
    }
}
