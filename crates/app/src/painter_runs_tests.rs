use vapors_scene::{Clips, Rect, Run};

use super::{scissor_clip, table_clip};

#[test]
fn each_run_is_cut_by_its_clip_entry_or_its_scissor_never_both_nor_neither() {
    // Given: a batch's clip, and every kind of run
    let clip = Some(Rect::new(0.0, 0.0, 10.0, 10.0));
    let on_the_table = [
        Run::Quads(Vec::new()),
        Run::Shadows(Vec::new()),
        Run::Shapes(Vec::new()),
        Run::Texts(Vec::new()),
        Run::Icons(Vec::new()),
        Run::Images(Vec::new()),
        Run::Surfaces(Vec::new()),
    ];
    let scissored = [Run::Meshes(Vec::new())];

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
            (None, clip),
            "{run:?}"
        );
    }
}

#[test]
#[should_panic(expected = "batch the scissor clips has items under several clips")]
fn a_scissored_batch_under_several_clips_is_refused() {
    let mut clips = Clips::one(None);
    clips.push(1, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
    let _ = scissor_clip(&Run::Meshes(Vec::new()), &clips);
}
