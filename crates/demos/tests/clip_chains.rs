//! The clip chains of the demos (flat-scene F3,
//! `docs/plans/flat-scene-f3.md` item 6): the longest is the measurement
//! `MAX_CLIP_CHAIN` cites, with the corpus's, and none is past it.

use vapors_elements::{MAX_CLIP_CHAIN, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

#[test]
fn no_demo_s_clip_chain_is_past_the_limit() {
    // When
    let chains: Vec<(&str, usize)> = vapors_demos::all()
        .into_iter()
        .map(|demo| {
            let document = vapors_demos::document(&demo).document;
            let mut document = document.borrow_mut();
            let painted = document
                .update(VIEWPORT)
                .unwrap_or_else(|errors| panic!("{}: refused: {errors:#?}", demo.name));
            (demo.name, painted.longest_clip_chain())
        })
        .collect();

    // Then
    let longest = chains
        .iter()
        .max_by_key(|(_, links)| *links)
        .expect("demos");
    eprintln!(
        "longest clip chain in the demos: {} links ({})",
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
