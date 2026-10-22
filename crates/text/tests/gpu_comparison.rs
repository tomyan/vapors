//! GPU tier: the comparison page's 13 px body has a mean ink within the
//! stated band of headless Chromium's rendering of the same HTML. A loose
//! guard against the renderer going blank, bold or misplaced, not a
//! quality judge; needs Chromium (the failure names the fix).

use std::path::PathBuf;

use vapors_gpu_harness::gpu;
use vapors_text::comparison::{
    Chromium, FONT_FILES, INK_BAND, PAGE_HEIGHT, PAGE_WIDTH, SCHEMES, body_measure_rect, html,
    ink_within_band, mean_ink, render_page,
};
use vapors_text::{BlendParams, FontSystem};

#[test]
fn gpu_body_paragraph_ink_is_within_the_band_of_chromiums() {
    // Given: the page rendered by both
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("text_comparison");
    std::fs::create_dir_all(&dir).expect("tmp dir");
    for (name, bytes) in FONT_FILES {
        std::fs::write(dir.join(name), bytes).expect("font file");
    }
    let html_path = dir.join("page.html");
    std::fs::write(&html_path, html()).expect("html");
    let gpu = gpu();
    let mut system = FontSystem::bundle_only();
    let ours = render_page(&gpu.device, &gpu.queue, &mut system, BlendParams::default());
    let chromium = Chromium::find().unwrap_or_else(|e| panic!("{e}"));
    let theirs = chromium
        .screenshot(
            &html_path,
            &dir.join("chromium.png"),
            PAGE_WIDTH,
            PAGE_HEIGHT,
        )
        .unwrap_or_else(|e| panic!("{e}"));

    // When
    let rect = body_measure_rect();
    let background = SCHEMES[0].background.to_rgba8();
    let (a, b) = (
        mean_ink(&ours, rect, background),
        mean_ink(&theirs, rect, background),
    );

    // Then
    assert!(
        ink_within_band(a, b),
        "13 px body mean ink: vapors {a:.4}, chromium {b:.4}, ratio {:.3} outside {INK_BAND:?}. \
         Fix: run `cargo run -p vapors-text --example text_comparison` and look at \
         target/text_comparison.png",
        a / b
    );
}
