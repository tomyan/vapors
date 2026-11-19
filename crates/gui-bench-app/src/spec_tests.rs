//! The vendored spec stays byte-identical to gui-bench at `f0644e8`
//! (docs/gui-bench/spec/NOTICE.md): an edit here would stop an upstream
//! change applying as a diff.

/// FNV-1a over the bytes: enough to notice an edit, with no dependency.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn the_vendored_spec_is_unchanged_from_upstream() {
    // Given
    let files: [(&str, &[u8], u64); 3] = [
        (
            "dashboard.rs",
            include_bytes!("../../../docs/gui-bench/spec/dashboard.rs"),
            0x2a8f_b87c_8908_6fb2,
        ),
        (
            "cli.rs",
            include_bytes!("../../../docs/gui-bench/spec/cli.rs"),
            0xb201_7666_2eba_e6c0,
        ),
        (
            "IDIOMS.md",
            include_bytes!("../../../docs/gui-bench/spec/IDIOMS.md"),
            0x3c99_344b_bb42_ecd2,
        ),
    ];

    // Then
    for (name, bytes, expected) in files {
        assert_eq!(
            fnv1a(bytes),
            expected,
            "docs/gui-bench/spec/{name} differs from gui-bench f0644e8; fix: restore it, or update all \
             three from a newer gui-bench checkout as NOTICE.md says and record the new commit and \
             hashes here; reproduce: cargo test -p vapors-gui-bench-app"
        );
    }
}
