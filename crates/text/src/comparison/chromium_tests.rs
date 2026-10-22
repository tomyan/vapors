//! The runner's checks on plain data: stale files and sizes.

use super::*;

#[test]
fn a_stale_screenshot_is_removed_and_a_missing_one_is_fine() {
    // Given
    let dir = std::env::temp_dir().join(format!("vapors-chromium-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let png = dir.join("stale.png");
    std::fs::write(&png, b"old").expect("write");

    // When
    remove_stale(&png).expect("removed");
    let again = remove_stale(&png);

    // Then
    assert!(!png.exists());
    assert_eq!(again, Ok(()));
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn a_screenshot_of_the_wrong_size_is_refused_naming_both_sizes() {
    // Given
    let image = Rgba {
        width: 10,
        height: 5,
        data: vec![0; 200],
    };

    // When
    let error = check_size(&image, Path::new("x.png"), [20, 10]).expect_err("wrong size");

    // Then
    let message = error.to_string();
    assert!(message.contains("10x5, wanted 20x10"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
}
