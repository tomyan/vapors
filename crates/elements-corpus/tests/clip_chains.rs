//! The clip chains of the corpus's fixtures (flat-scene F3,
//! `docs/plans/flat-scene-f3.md` item 6): the longest is the measurement
//! `MAX_CLIP_CHAIN` cites, and none is past it.

#[path = "corpus_support/images.rs"]
mod images;

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::{Document, MAX_CLIP_CHAIN, Viewport};
use vapors_elements_corpus::all;
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 800.0,
    height: 600.0,
    scale: 1.0,
};

/// Each fixture's longest clip chain, by name.
fn longest_chains() -> Vec<(&'static str, usize)> {
    all()
        .into_iter()
        .map(|fixture| {
            let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
            let body = doc.borrow().body();
            (fixture.mount)(&doc, body);
            images::register_fixture_images(&mut doc.borrow_mut());
            let mut doc = doc.borrow_mut();
            let painted = doc
                .update(VIEWPORT)
                .unwrap_or_else(|errors| panic!("{}: refused: {errors:#?}", fixture.name));
            (fixture.name, painted.longest_clip_chain())
        })
        .collect()
}

#[test]
fn the_corpus_s_longest_clip_chain_is_the_measured_one_and_none_is_past_the_limit() {
    // When
    let chains = longest_chains();

    // Then
    let longest = chains
        .iter()
        .max_by_key(|(_, links)| *links)
        .expect("fixtures");
    assert_eq!(
        longest.1,
        vapors_elements::MEASURED_CORPUS_CLIP_CHAIN,
        "the longest clip chain is {} links ({}), not the measurement MAX_CLIP_CHAIN \
         cites: set the constant in crates/elements/src/clip_chain.rs and say so at \
         MAX_CLIP_CHAIN",
        longest.1,
        longest.0
    );
    eprintln!(
        "longest clip chain in the corpus: {} links ({})",
        longest.1, longest.0
    );
    let past: Vec<_> = chains
        .iter()
        .filter(|(_, links)| *links > MAX_CLIP_CHAIN)
        .collect();
    assert!(
        past.is_empty(),
        "past MAX_CLIP_CHAIN {MAX_CLIP_CHAIN}: {past:?}"
    );
}
