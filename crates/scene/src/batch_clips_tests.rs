use super::Clips;
use crate::display::Rect;

fn rect(left: f32) -> Option<Rect> {
    Rect::new(left, 0.0, left + 10.0, 10.0).into()
}

#[test]
fn items_under_one_clip_make_one_run() {
    let mut clips = Clips::one(rect(0.0));
    clips.push(1, rect(0.0));
    clips.push(2, rect(0.0));
    assert_eq!(clips.uniform(), Some(rect(0.0)));
}

#[test]
fn each_item_reads_the_clip_of_its_run() {
    // Given: items 0-1 under a, 2 unclipped, 3-4 under b
    let mut clips = Clips::one(rect(0.0));
    for (at, clip) in [(1, rect(0.0)), (2, None), (3, rect(20.0)), (4, rect(20.0))] {
        clips.push(at, clip);
    }

    // Then
    assert_eq!(clips.runs(), [(0, rect(0.0)), (2, None), (3, rect(20.0))]);
    let each: Vec<Option<Rect>> = (0..5).map(|at| clips.at(at)).collect();
    assert_eq!(each, [rect(0.0), rect(0.0), None, rect(20.0), rect(20.0)]);
    assert_eq!(clips.uniform(), None);
}

#[test]
fn the_union_holds_every_clip_and_is_unclipped_when_an_item_is() {
    let mut clips = Clips::one(rect(0.0));
    clips.push(1, rect(20.0));
    assert_eq!(clips.union(), Some(Rect::new(0.0, 0.0, 30.0, 10.0)));
    clips.push(2, None);
    assert_eq!(clips.union(), None);
}
