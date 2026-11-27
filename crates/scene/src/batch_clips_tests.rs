use super::Clips;
use crate::clip::Clip;
use crate::display::Rect;

fn rect(left: f32) -> Clip {
    Rect::new(left, 0.0, left + 10.0, 10.0).into()
}

#[test]
fn items_under_one_clip_make_one_run() {
    let mut clips = Clips::one(Some(rect(0.0)));
    clips.push(1, Some(rect(0.0)));
    clips.push(2, Some(rect(0.0)));
    assert_eq!(clips.uniform(), Some(Some(rect(0.0))));
}

#[test]
fn each_item_reads_the_clip_of_its_run() {
    // Given: items 0-1 under a, 2 unclipped, 3-4 under b
    let mut clips = Clips::one(Some(rect(0.0)));
    for (at, clip) in [
        (1, Some(rect(0.0))),
        (2, None),
        (3, Some(rect(20.0))),
        (4, Some(rect(20.0))),
    ] {
        clips.push(at, clip);
    }

    // Then
    assert_eq!(
        clips.runs(),
        [(0, Some(rect(0.0))), (2, None), (3, Some(rect(20.0)))]
    );
    let each: Vec<Option<Clip>> = (0..5).map(|at| clips.at(at)).collect();
    assert_eq!(
        each,
        [
            Some(rect(0.0)),
            Some(rect(0.0)),
            None,
            Some(rect(20.0)),
            Some(rect(20.0))
        ]
    );
    assert_eq!(clips.uniform(), None);
}

#[test]
#[should_panic(expected = "given item 1: items are pushed in order")]
fn an_item_pushed_out_of_order_is_refused() {
    let mut clips = Clips::one(Some(rect(0.0)));
    clips.push(2, Some(rect(20.0)));
    clips.push(1, Some(rect(0.0)));
}

#[test]
fn the_union_holds_every_clip_and_is_unclipped_when_an_item_is() {
    let mut clips = Clips::one(Some(rect(0.0)));
    clips.push(1, Some(rect(20.0)));
    assert_eq!(clips.union(), Some(Rect::new(0.0, 0.0, 30.0, 10.0)));
    clips.push(2, None);
    assert_eq!(clips.union(), None);
}

#[test]
fn the_union_holds_a_cut_rounded_clip_by_its_bounds() {
    // Given: a rounded clip cut to its left half on the page
    let rounded = Clip::rounded(Rect::new(0.0, 0.0, 20.0, 10.0), [4.0; 4]);
    let clips = Clips::one(Some(rounded.cut(Rect::new(0.0, 0.0, 10.0, 10.0))));

    // Then
    assert_eq!(clips.union(), Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
}
