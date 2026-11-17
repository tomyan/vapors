use std::path::Path;

use super::Builder;

#[test]
fn a_builders_files_are_under_its_own_directory_and_its_target_beside_the_others() {
    // Given
    let builder = Builder::new("svg-v1", Path::new("/home/b")).expect("a plain name");

    // Then
    assert_eq!(
        builder.clone_dir(),
        Path::new("/home/b/.cache/api-build/svg-v1/vapors")
    );
    assert_eq!(
        builder.target(),
        Path::new("/home/b/.cache/vapors-targets/api-svg-v1")
    );
    assert_eq!(
        builder.socket(),
        Path::new("/home/b/.cache/api-build/svg-v1/socket/harness.sock")
    );
}

#[test]
fn a_name_that_could_leave_its_directory_is_refused() {
    for name in ["", "../x", "a/b", "a b"] {
        let refused = Builder::new(name, Path::new("/home/b"));
        assert!(refused.is_err(), "{name:?} was accepted");
    }
}
