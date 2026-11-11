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
        cache.begin();
        for n in 0..=cache.entries.len() {
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
    cache.begin();
    let first = cache.layout(&mut fonts, 1.0, &run(text, Align::Start), None);
    let first = held.then_some(first);
    later_updates(&mut cache, &mut fonts, &|_| {});
    cache.begin();
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
    cache.begin();
    let _held: Vec<_> = (0..5)
        .map(|n| cache.layout(&mut fonts, 1.0, &other(0, n), None))
        .collect();

    // When, Then
    later_updates(&mut cache, &mut fonts, &|cache| {
        let bound = 2 * cache.swept_to.max(1);
        assert!(
            cache.entries.len() <= bound,
            "{} > {bound}",
            cache.entries.len()
        );
    });
}
