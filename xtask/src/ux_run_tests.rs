//! The walk's run directory read, and anything else in it refused.

use std::path::Path;

use super::*;

fn run_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-ux-run-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("made");
    dir
}

fn write_png(path: &Path, width: u32, height: u32) {
    let file = File::create(path).expect("create png");
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let data = vec![0u8; (width * height * 3) as usize];
    encoder
        .write_header()
        .expect("header")
        .write_image_data(&data)
        .expect("data");
}

/// A viewport with one step, `01-first-open`.
fn viewport_dir(run: &Path, name: &str) -> PathBuf {
    let dir = run.join(name);
    std::fs::create_dir_all(&dir).expect("made");
    write_png(&dir.join("01-first-open.png"), 8, 16);
    std::fs::write(dir.join("01-first-open.txt"), "Open").expect("text");
    let facts = r#"{"steps": [{"step": "first-open", "frame": "01-first-open.png", "text": "01-first-open.txt"}]}"#;
    std::fs::write(dir.join("facts.json"), facts).expect("facts");
    dir
}

#[test]
fn each_viewport_is_read_with_its_frames_sizes_and_texts() {
    // Given
    let run = run_dir("good");
    viewport_dir(&run, "phone");
    viewport_dir(&run, "desktop");

    // When
    let viewports = read(&run).expect("read");

    // Then
    assert_eq!(
        viewports
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        ["desktop", "phone"]
    );
    let frame = &viewports[1].frames[0];
    assert_eq!(
        (frame.name.as_str(), frame.size, frame.text.as_str()),
        ("phone/01-first-open.png", (8, 16), "Open")
    );
}

#[test]
fn a_directory_that_is_not_a_viewport_is_refused() {
    // Given
    let run = run_dir("stray");
    viewport_dir(&run, "phone");
    std::fs::create_dir_all(run.join("fixture")).expect("made");

    // When
    let refused = read(&run).expect_err("refused");

    // Then
    assert!(refused.contains("fixture is not a viewport"), "{refused}");
}

#[test]
fn a_missing_run_directory_is_refused_naming_the_fix() {
    // When
    let refused = read(Path::new("/nonexistent/ux")).expect_err("refused");

    // Then
    assert!(refused.contains("fix: run the walk first"), "{refused}");
}

#[test]
fn a_frame_that_is_missing_or_not_a_png_is_refused() {
    // Given
    let run = run_dir("bad-frame");
    let dir = viewport_dir(&run, "phone");
    std::fs::write(dir.join("01-first-open.png"), "not a png").expect("written");

    // When
    let refused = read(&run).expect_err("refused");

    // Then
    assert!(
        refused.contains("01-first-open.png is not a PNG"),
        "{refused}"
    );
}

#[test]
fn a_step_naming_a_path_outside_its_viewport_is_refused() {
    // Given
    let run = run_dir("outside");
    let dir = viewport_dir(&run, "phone");
    let facts =
        r#"{"steps": [{"step": "x", "frame": "../other.png", "text": "01-first-open.txt"}]}"#;
    std::fs::write(dir.join("facts.json"), facts).expect("facts");

    // When
    let refused = read(&run).expect_err("refused");

    // Then
    assert!(
        refused.contains("is not a file name beside it"),
        "{refused}"
    );
}
