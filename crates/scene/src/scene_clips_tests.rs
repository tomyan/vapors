use crate::{Clip, Color, Item, Quad, Rect, Scene};

fn quad() -> Item {
    Item::Quad(Quad::fill(Rect::new(0.0, 0.0, 1.0, 1.0), Color::default()))
}

fn clip(left: f32) -> Clip {
    Rect::new(left, 0.0, left + 4.0, 4.0).into()
}

#[test]
fn each_item_reads_the_clip_it_was_pushed_under() {
    // Given: none, a, b, a again
    let mut scene = Scene::new();
    for each in [None, Some(clip(0.0)), Some(clip(10.0)), Some(clip(0.0))] {
        scene.set_rounded_clip(each);
        scene.push(quad());
    }

    // Then
    let clips: Vec<_> = scene.clips().collect();
    assert_eq!(
        clips,
        [None, Some(clip(0.0)), Some(clip(10.0)), Some(clip(0.0))]
    );
    assert_eq!(scene.clip_at(2), Some(clip(10.0)));
    assert_eq!(scene.clip_at(9), None);
}

#[test]
fn a_clip_set_again_is_named_once() {
    // Given
    let mut scene = Scene::new();
    for each in [
        Some(clip(0.0)),
        Some(clip(10.0)),
        Some(clip(0.0)),
        Some(clip(10.0)),
    ] {
        scene.set_rounded_clip(each);
        scene.push(quad());
    }

    // Then: two clips in the list for four items
    assert_eq!(scene.palette().len(), 2);
}

#[test]
fn scenes_with_their_clips_listed_in_other_orders_are_equal() {
    // Given: b's list holds a clip a never sets, before the one both use
    let (mut a, mut b) = (Scene::new(), Scene::new());
    a.set_rounded_clip(Some(clip(0.0)));
    a.push(quad());
    b.set_rounded_clip(Some(clip(10.0)));
    b.set_rounded_clip(Some(clip(0.0)));
    b.push(quad());

    // Then
    assert_eq!(a, b);
}

#[test]
fn every_clip_set_at_once_covers_the_items_and_the_pen() {
    // Given
    let mut scene = Scene::new();
    scene.set_rounded_clip(Some(clip(0.0)));
    scene.push(quad());

    // When
    scene.set_every_clip(Some(clip(10.0)));
    scene.push(quad());

    // Then
    let clips: Vec<_> = scene.clips().collect();
    assert_eq!(clips, [Some(clip(10.0)), Some(clip(10.0))]);
}

#[test]
fn every_clip_set_at_once_is_the_only_clip_listed() {
    // Given: a kept box's scene shifted under one clip after another
    let mut scene = Scene::new();
    scene.set_rounded_clip(Some(clip(0.0)));
    scene.push(quad());

    // When
    scene.set_every_clip(Some(clip(10.0)));
    scene.set_every_clip(Some(clip(20.0)));

    // Then
    assert_eq!(scene.palette(), [clip(20.0)]);
    scene.set_every_clip(None);
    assert_eq!(scene.palette(), []);
}

#[test]
fn scenes_that_would_push_under_different_clips_are_unequal() {
    // Given
    let (mut a, mut b) = (Scene::new(), Scene::new());
    a.set_rounded_clip(Some(clip(0.0)));
    b.set_rounded_clip(Some(clip(10.0)));

    // Then
    assert_ne!(a, b);
}

#[test]
fn a_scene_is_no_larger_than_an_option_rect_clip_made_it() {
    // Then: items, the clip names, the list (a boxed slice), the pen
    assert!(
        std::mem::size_of::<Scene>() <= 72,
        "{}",
        std::mem::size_of::<Scene>()
    );
}
