use std::rc::Rc;

use vapors_text::{Align, FontSystem, Layout};

use super::super::inline::InlineRun;
use super::TextCache;
use super::tests::run;

/// A run no other call names: update `update`'s `n`th.
fn other(update: usize, n: usize) -> InlineRun {
    run(&format!("{update} {n}"), Align::Start)
}

/// Updates after the first, each asking for one run more than the map
/// holds, so each sweeps at least once; after each call, `check`.
fn later_updates(cache: &mut TextCache, fonts: &mut FontSystem, check: &dyn Fn(&TextCache)) {
    for update in 1..4 {
        cache.begin(0);
        for n in 0..=cache.layouts.entries.len() {
            cache.layout(fonts, 1.0, &other(update, n), None);
            check(cache);
        }
    }
}

/// `text` laid out in a first update, then the layout `held` keeps
/// after `later_updates`, and the layout asked for again and how many
/// layouts that built.
fn asked_again(text: &str, held: bool) -> (Option<Rc<Layout>>, Rc<Layout>, usize) {
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    cache.begin(0);
    let first = cache.layout(&mut fonts, 1.0, &run(text, Align::Start), None);
    let first = held.then_some(first);
    later_updates(&mut cache, &mut fonts, &|_| {});
    cache.begin(0);
    let again = cache.layout(&mut fonts, 1.0, &run(text, Align::Start), None);
    (first, again, cache.built())
}

#[test]
fn a_layout_a_kept_part_holds_survives_updates_that_do_not_ask_for_it() {
    // Given, When
    let (first, again, built) = asked_again("held", true);

    // Then
    assert_eq!(built, 0);
    assert!(Rc::ptr_eq(&first.expect("held"), &again));
}

#[test]
fn a_layout_nothing_holds_is_gone_after_the_next_sweep() {
    // Given, When
    let (_, _, built) = asked_again("dropped", false);

    // Then
    assert_eq!(built, 1);
}

#[test]
fn the_map_never_holds_more_than_twice_what_the_last_sweep_left() {
    // Given
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    cache.begin(0);
    let _held: Vec<_> = (0..5)
        .map(|n| cache.layout(&mut fonts, 1.0, &other(0, n), None))
        .collect();

    // When, Then
    later_updates(&mut cache, &mut fonts, &|cache| {
        let bound = 2 * cache.layouts.swept_to.max(1);
        assert!(
            cache.layouts.entries.len() <= bound,
            "{} > {bound}",
            cache.layouts.entries.len()
        );
    });
}

#[test]
fn a_layout_reused_at_a_width_is_dropped_once_nothing_holds_it() {
    // Given
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    let label = run("label", Align::Start);
    cache.begin(0);
    cache.layout(&mut fonts, 1.0, &label, None);
    cache.layout(&mut fonts, 1.0, &label, Some(500.0));

    // When
    later_updates(&mut cache, &mut fonts, &|_| {});

    // Then
    let first: Vec<_> = cache
        .layouts
        .entries
        .values()
        .filter(|entry| entry.asked == 1)
        .collect();
    assert_eq!(
        first.len(),
        0,
        "{} entries from update 1 survive",
        first.len()
    );
}

/// A first update showing `before` runs, then one showing `after`, the
/// first's layouts no longer held.
fn shown_then(before: usize, after: usize) -> (FontSystem, TextCache) {
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    let mut show = |cache: &mut TextCache, update: usize, count: usize| -> Vec<_> {
        (0..count)
            .map(|n| cache.layout(&mut fonts, 1.0, &other(update, n), None))
            .collect()
    };
    cache.begin(0);
    let first = show(&mut cache, 0, before);
    cache.begin(first.len());
    drop(first);
    show(&mut cache, 1, after);
    (fonts, cache)
}

#[test]
fn layouts_a_shrunk_document_no_longer_shows_are_dropped_at_the_next_update() {
    // Given
    let (_fonts, mut cache) = shown_then(30, 2);

    // When
    cache.begin(2);

    // Then
    assert_eq!(cache.layouts.entries.len(), 2);
}

#[test]
fn an_update_that_shows_what_the_map_holds_does_not_sweep_it() {
    // Given
    let (_fonts, mut cache) = shown_then(30, 1);
    let swept_to = cache.layouts.swept_to;

    // When
    cache.begin(31);

    // Then
    assert_eq!(
        cache.layouts.swept_to, swept_to,
        "a sweep ran at the start of the update"
    );
}

#[test]
fn shapings_a_shrunk_document_no_longer_shows_are_dropped_at_the_next_update() {
    // Given
    let (_fonts, mut cache) = shown_then(30, 2);

    // When
    cache.begin(2);

    // Then
    assert_eq!(cache.shapes.entries.len(), 2);
}

#[test]
fn a_shaping_whose_layouts_are_still_asked_for_is_kept() {
    // Given: 30 runs shaped, then asked for again from their layouts
    let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());
    let runs: Vec<_> = (0..30).map(|n| other(0, n)).collect();
    let mut ask = |cache: &mut TextCache| {
        runs.iter()
            .for_each(|run| drop(cache.layout(&mut fonts, 1.0, run, None)));
    };
    cache.begin(0);
    ask(&mut cache);
    cache.begin(30);
    ask(&mut cache);

    // When: the next update, the last paint having shown one of them
    cache.begin(1);

    // Then
    assert_eq!(cache.shapes.entries.len(), 30);
}
