use std::io::Read as _;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-precompress-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).expect("dir");
    dir
}

#[test]
fn each_page_file_gets_a_brotli_and_a_gzip_copy_that_decode_to_it() {
    // Given
    let dir = scratch("decode");
    let text = "touché ".repeat(500);
    std::fs::write(dir.join("sub/page.js"), &text).expect("write");

    // When
    let done = precompress(&dir).expect("precompress");

    // Then
    assert_eq!(done.len(), 1, "{done:?}");
    let br = std::fs::read(dir.join("sub/page.js.br")).expect("br");
    let mut from_br = String::new();
    brotli::Decompressor::new(br.as_slice(), 4096)
        .read_to_string(&mut from_br)
        .expect("brotli");
    let gz = std::fs::read(dir.join("sub/page.js.gz")).expect("gz");
    let mut from_gz = String::new();
    flate2::read::GzDecoder::new(gz.as_slice())
        .read_to_string(&mut from_gz)
        .expect("gzip");
    assert_eq!(
        (from_br.as_str(), from_gz.as_str()),
        (text.as_str(), text.as_str())
    );
    assert!(
        done[0].brotli < done[0].bytes && done[0].gzip < done[0].bytes,
        "{done:?}"
    );
}

#[test]
fn images_and_compressed_copies_are_left_alone() {
    // Given
    let dir = scratch("skip");
    std::fs::write(dir.join("icon.png"), b"png").expect("write");
    std::fs::write(dir.join("page.js.br"), b"br").expect("write");

    // When
    let done = precompress(&dir).expect("precompress");

    // Then
    assert!(done.is_empty(), "{done:?}");
    assert!(!dir.join("icon.png.br").exists());
    assert!(!dir.join("page.js.br.br").exists());
}

#[test]
fn an_unreadable_directory_is_refused_naming_it() {
    let missing = std::env::temp_dir().join("xtask-precompress-no-such-dir");
    let refusal = precompress(&missing).expect_err("refused");
    assert!(
        refusal.contains("xtask-precompress-no-such-dir"),
        "{refusal}"
    );
}
