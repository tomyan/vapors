use super::*;

fn quad(left: f32) -> Item {
    Item::Quad(Quad {
        rect: Rect::new(left, 0.0, left + 1.0, 1.0),
        background: Color::from_rgba8(1, 2, 3, 255),
    })
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
    assert!(batches.is_empty());
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
        .map(|batch| match batch {
            Batch::Quads(quads) => ('q', quads.len()),
            Batch::Texts(texts) => ('t', texts.len()),
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
    let Batch::Quads(quads) = &batches[0] else {
        panic!("expected quads, got {batches:?}");
    };
    let lefts: Vec<f32> = quads.iter().map(|q| q.rect.left).collect();
    assert_eq!(lefts, [5.0, 7.0]);
}
