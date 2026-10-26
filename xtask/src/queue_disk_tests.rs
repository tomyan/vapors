use std::path::PathBuf;

use super::*;

#[test]
fn df_output_gives_the_bytes_available() {
    // Given
    let text = "       Avail\n238125666304\n";

    // When
    let free = parse_df(text);

    // Then
    assert_eq!(free, Ok(238_125_666_304));
}

#[test]
fn df_output_of_another_shape_is_refused_quoting_it() {
    for text in ["", "Avail\n", "Avail\nlots\n", "Avail\n1\n2\n"] {
        let why = parse_df(text).expect_err(text);
        assert!(why.starts_with("df's output is not "), "{why}");
        assert!(why.contains(&format!("{text:?}")), "{why}");
    }
}

#[test]
fn du_output_is_read_largest_first() {
    // Given
    let text = "100\t/t/a\n300\t/t/b\n200\t/t/c\n";

    // When
    let sizes = parse_du(text).expect("read");

    // Then
    let paths: Vec<_> = sizes
        .iter()
        .map(|(size, path)| (*size, path.clone()))
        .collect();
    assert_eq!(
        paths,
        [
            (300, PathBuf::from("/t/b")),
            (200, PathBuf::from("/t/c")),
            (100, PathBuf::from("/t/a"))
        ]
    );
}

#[test]
fn a_du_line_without_a_size_is_refused_quoting_it() {
    let why = parse_du("100\t/t/a\nbig /t/b\n").expect_err("refused");
    assert_eq!(
        why,
        "du's line is not a size, a tab and a path: \"big /t/b\""
    );
}

#[test]
fn the_description_names_the_free_space_and_the_five_largest_targets() {
    // Given
    let largest: Vec<_> = (1..=6)
        .rev()
        .map(|n| (n * 10 * GB, PathBuf::from(format!("/t/{n}"))))
        .collect();

    // When
    let line = describe(41 * GB + 400_000_000, &largest);

    // Then
    assert_eq!(
        line,
        "the disk has 41 GB free; the largest targets: 60 GB /t/6, 50 GB /t/5, 40 GB /t/4, 30 GB /t/3, 20 GB /t/2; fix: empty the ones whose branch merged (find <dir> -mindepth 1 -maxdepth 1 -exec rm -rf {} +)"
    );
}

#[test]
fn the_free_space_is_read_from_df() {
    let free = free_bytes(std::path::Path::new("/")).expect("df answered");
    assert!(free > 0);
}

#[test]
fn the_directories_in_the_roots_are_sized_by_du_largest_first() {
    // Given: a root holding a 1 MB directory and a 1 kB one
    let root = std::env::temp_dir().join(format!("xtask-disk-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (name, bytes) in [("small", 1_000), ("big", 1_000_000)] {
        std::fs::create_dir_all(root.join(name)).expect("dir");
        std::fs::write(root.join(name).join("f"), vec![1u8; bytes]).expect("file");
    }

    // When
    let sizes = largest(std::slice::from_ref(&root));
    let _ = std::fs::remove_dir_all(&root);

    // Then
    let sizes = sizes.expect("du answered");
    let paths: Vec<_> = sizes.iter().map(|(_, path)| path.clone()).collect();
    assert_eq!(paths, [root.join("big"), root.join("small")]);
    assert!(sizes[0].0 >= 1_000_000, "{sizes:?}");
}
