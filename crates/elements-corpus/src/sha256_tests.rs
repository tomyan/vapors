//! A hash is taken within its deadline, and a hasher that outruns it is
//! killed and refused.

use std::time::{Duration, Instant};

use super::*;

#[test]
fn bytes_hash_as_sha256sum_prints_them() {
    // When
    let hash = sha256(b"abc").expect("hashed");

    // Then: the FIPS 180-2 example
    assert_eq!(
        hash,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn a_hasher_that_outruns_its_deadline_is_killed_and_refused() {
    // Given: a "hasher" that sleeps 5 s, held to 100 ms
    let input = tempfile::tempfile().expect("a file");
    let started = Instant::now();

    // When
    let refused =
        digest_with("sleep", &["5"], input, Duration::from_millis(100)).expect_err("refused");

    // Then
    assert!(refused.contains("deadline"), "{refused}");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "waited {:?}",
        started.elapsed()
    );
}
