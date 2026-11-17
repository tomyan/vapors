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
fn the_demos_longest_clip_chain_is_the_measured_one_and_none_is_past_the_limit() {
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
    assert_eq!(
        longest.1,
        vapors_elements::MEASURED_DEMOS_CLIP_CHAIN,
        "the longest clip chain is {} links ({}), not the measurement MAX_CLIP_CHAIN \
         cites: set the constant in crates/elements/src/clip_chain.rs and say so at \
         MAX_CLIP_CHAIN",
        longest.1,
        longest.0
    );
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
