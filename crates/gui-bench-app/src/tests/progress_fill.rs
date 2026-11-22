//! The order row's progress fill: one element whose width is bound to the
//! percentage, held to the spec's 120 px track for every 0..=100 and,
//! after a sort, to the order each row still shows.
#![allow(clippy::cast_precision_loss)] // percentages and indexes are small
use super::behaviour_top::centre_of;
use super::support::*;
use crate::app::Mounted;
use crate::dashboard::*;

/// Chromium's layout unit, 1/64 px (the corpus fixture `progress_fill`).
const UNIT: f32 = 1.0 / 64.0;
const COUNT: usize = 101;

fn want(progress: u32) -> f32 {
    progress as f32 * PROGRESS_WIDTH / 100.0
}

/// The fill widths in display order, with the order number beside each.
fn shown(mounted: &Mounted) -> Vec<(String, f32)> {
    let doc = mounted.document.borrow();
    let numbers = by_class(&doc, "order-number");
    let fills = by_class(&doc, "fill");
    assert_eq!(numbers.len(), fills.len(), "one fill per row");
    let pair = |(n, f): (_, _)| (text(&doc, n), rect(&doc, f)[2]);
    numbers.into_iter().zip(fills).map(pair).collect()
}

fn progress_of(number: &str) -> u32 {
    let n: usize = number.trim_start_matches('#').parse().expect("a number");
    order(n - 10200).progress
}

#[test]
fn the_fill_is_one_element_with_no_bit_segments() {
    let mounted = with_rows(COUNT);
    let doc = mounted.document.borrow();
    assert_eq!(by_class(&doc, "fill").len(), COUNT);
    for k in 0..7 {
        assert!(by_class(&doc, &format!("b{k}")).is_empty(), "segment b{k}");
    }
    assert_eq!(doc.children(by_class(&doc, "fill")[0]).len(), 0);
}

#[test]
fn every_percentage_is_its_share_of_the_track() {
    let widths = shown(&with_rows(COUNT));
    let mut seen: Vec<u32> = widths.iter().map(|(n, _)| progress_of(n)).collect();
    seen.sort_unstable();
    assert_eq!(seen, (0..=100).collect::<Vec<u32>>());
    for (number, width) in widths {
        let p = progress_of(&number);
        assert!(
            (width - want(p)).abs() <= UNIT,
            "{number}: {width} for {p} %"
        );
    }
}

#[test]
fn the_empty_and_full_endpoints_are_zero_and_the_whole_track() {
    for (number, width) in shown(&with_rows(COUNT)) {
        match progress_of(&number) {
            0 => assert!(width.abs() < 1e-4, "{number} empty: {width}"),
            100 => assert!(
                (width - PROGRESS_WIDTH).abs() < 1e-4,
                "{number} full: {width}"
            ),
            _ => {}
        }
    }
}

#[test]
fn a_sort_keeps_each_fill_with_its_order() {
    let mounted = with_rows(COUNT);
    let before = shown(&mounted);
    click(&mounted, centre_of(&mounted, "sort", 0));
    let after = shown(&mounted);
    assert_ne!(
        after.iter().map(|r| &r.0).collect::<Vec<_>>(),
        before.iter().map(|r| &r.0).collect::<Vec<_>>(),
        "the sort moved the rows"
    );
    assert_eq!(after.len(), COUNT);
    for (number, width) in after {
        let kept = before.iter().find(|(n, _)| *n == number).expect("kept");
        assert!((width - kept.1).abs() <= UNIT, "{number} changed its fill");
        assert!(
            (width - want(progress_of(&number))).abs() <= UNIT,
            "{number}"
        );
    }
}
