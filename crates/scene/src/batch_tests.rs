use super::*;
use crate::{Color, TextId};

fn quad(left: f32) -> Item {
    Item::Quad(Quad::fill(
        Rect::new(left, 0.0, left + 1.0, 1.0),
        Color::from_rgba8(1, 2, 3, 255),
    ))
}

fn text(id: u32) -> Item {
    Item::Text {
        text: TextId(id),
        origin: [0.0, 0.0],
    }
}

#[test]
fn an_empty_scene_has_no_batches() {
    // Given
    let scene = Scene::new();

    // When
    let batches = scene.batches();

    // Then
    assert_eq!(batches, []);
}

#[test]
fn consecutive_items_of_one_kind_share_a_batch_in_painting_order() {
    // Given: two quads, a text, a quad, two texts
    let mut scene = Scene::new();
    for item in [quad(0.0), quad(1.0), text(0), quad(2.0), text(1), text(2)] {
        scene.push(item);
    }

    // When
    let batches = scene.batches();

    // Then
    let shape: Vec<(char, usize)> = batches
        .iter()
        .map(|batch| match &batch.run {
            Run::Shadows(shadows) => ('s', shadows.len()),
            Run::Quads(quads) => ('q', quads.len()),
            Run::Texts(texts) => ('t', texts.len()),
        })
        .collect();
    assert_eq!(shape, [('q', 2), ('t', 1), ('q', 1), ('t', 2)]);
}

#[test]
fn a_batch_keeps_its_items_in_order() {
    // Given
    let mut scene = Scene::new();
    scene.push(quad(5.0));
    scene.push(quad(7.0));

    // When
    let batches = scene.batches();

    // Then
    let Run::Quads(quads) = &batches[0].run else {
        panic!("expected quads, got {batches:?}");
    };
    let lefts: Vec<f32> = quads.iter().map(|q| q.rect.left).collect();
    assert_eq!(lefts, [5.0, 7.0]);
}

#[test]
fn a_change_of_clip_starts_a_batch_that_carries_the_clip() {
    // Given: a quad, two quads under a clip, a quad with the clip lifted
    let clip = Rect::new(0.0, 0.0, 4.0, 4.0);
    let mut scene = Scene::new();
    scene.push(quad(0.0));
    scene.set_clip(Some(clip));
    scene.push(quad(1.0));
    scene.push(quad(2.0));
    scene.set_clip(None);
    scene.push(quad(3.0));

    // When
    let batches = scene.batches();

    // Then
    let shape: Vec<(Option<Rect>, usize)> = batches
        .iter()
        .map(|batch| match &batch.run {
            Run::Quads(quads) => (batch.clip, quads.len()),
            other => panic!("expected quads, got {other:?}"),
        })
        .collect();
    assert_eq!(shape, [(None, 1), (Some(clip), 2), (None, 1)]);
}

#[test]
fn each_item_keeps_the_clip_it_was_pushed_under() {
    // Given
    let clip = Rect::new(1.0, 2.0, 3.0, 4.0);
    let mut scene = Scene::new();
    scene.push(text(0));
    scene.set_clip(Some(clip));
    scene.push(text(1));

    // When
    let clips = scene.clips();

    // Then
    assert_eq!(clips, [None, Some(clip)]);
}
