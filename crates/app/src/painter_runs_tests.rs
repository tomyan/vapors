use vapors_scene::{Clip, Clips, Rect, Run};

use super::{scissor_clip, table_clip};

#[test]
fn each_run_is_cut_by_its_clip_entry_or_its_scissor_never_both_nor_neither() {
    // Given: a batch's clip, and every kind of run
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let clip = Some(Clip::square(rect));
    let on_the_table = [
        Run::Quads(Vec::new()),
        Run::Shadows(Vec::new()),
        Run::Shapes(Vec::new()),
        Run::Texts(Vec::new()),
        Run::Icons(Vec::new()),
    ];
    let scissored = [
        Run::Images(Vec::new()),
        Run::Surfaces(Vec::new()),
        Run::Meshes(Vec::new()),
    ];

    // Then: quads, shadows, shapes, text and icons take a clip entry and
    // the frame's scissor only; the rest take entry 0 and the batch's
    // scissor
    for run in &on_the_table {
        assert_eq!(
            (table_clip(run, clip), scissor_clip(run, &Clips::one(clip))),
            (clip, None),
            "{run:?}"
        );
    }
    for run in &scissored {
        assert_eq!(
            (table_clip(run, clip), scissor_clip(run, &Clips::one(clip))),
            (None, Some(rect)),
            "{run:?}"
        );
    }
}

#[test]
#[should_panic(expected = "batch the scissor clips has items under several clips")]
fn a_scissored_batch_under_several_clips_is_refused() {
    let mut clips = Clips::one(None);
    clips.push(1, Some(Clip::square(Rect::new(0.0, 0.0, 10.0, 10.0))));
    let _ = scissor_clip(&Run::Images(Vec::new()), &clips);
}

#[test]
#[should_panic(expected = "is under the rounded clip")]
fn a_scissored_batch_under_a_rounded_clip_is_refused() {
    // Given: an image batch under a rounded clip, which a scissor cannot cut
    let rounded = Clip::rounded(Rect::new(0.0, 0.0, 20.0, 20.0), [4.0; 4]);

    // When, Then
    let _ = scissor_clip(&Run::Images(Vec::new()), &Clips::one(Some(rounded)));
}

#[test]
fn a_rounded_clip_reaches_the_table_for_a_run_on_it() {
    // Given
    let rounded = Some(Clip::rounded(Rect::new(0.0, 0.0, 20.0, 20.0), [4.0; 4]));

    // When
    let entry = table_clip(&Run::Quads(Vec::new()), rounded);

    // Then: the clip with its corners
    assert_eq!(entry, rounded);
}
