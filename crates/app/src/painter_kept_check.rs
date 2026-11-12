//! The debug build's check on kept batch bytes (`painter_kept.rs`):
//! on every draw from kept bytes, the batch's bytes built afresh equal
//! those kept.

use vapors_scene::Run;

/// Panics, naming batch `at`, the instance and attribute of the first
/// byte that differs, when `kept` is not `fresh`.
pub(super) fn assert_kept(kept: &[u8], fresh: &[u8], (run, at): (&Run, usize)) {
    if kept == fresh {
        return;
    }
    let (kind, stride) = described(run);
    let mut pairs = kept.iter().zip(fresh);
    let first = pairs.position(|(kept, fresh)| kept != fresh);
    let byte = first.unwrap_or_else(|| kept.len().min(fresh.len()));
    panic!(
        "batch {at} ({kind}) drew kept bytes that differ from its bytes built afresh at \
         instance {}, attribute {} (byte {byte}; {} bytes kept, {} afresh): a key was kept for \
         a batch whose run changed (kept_batches_keys.rs), or bytes were kept for another \
         viewport (painter_kept.rs); reproduce with `cargo xtask test`",
        byte / stride,
        byte % stride / 16,
        kept.len(),
        fresh.len()
    );
}

/// `run`'s kind, and the bytes of one instance (a quad's six vertices
/// for images and surfaces).
fn described(run: &Run) -> (&'static str, usize) {
    match run {
        Run::Shadows(_) => ("shadows", 112),
        Run::Quads(_) => ("quads", 128),
        Run::Images(_) => ("images", 96),
        Run::Surfaces(_) => ("surfaces", 96),
        Run::Texts(_) => ("texts", 1),
        Run::Icons(_) => ("icons", 1),
    }
}

#[cfg(test)]
mod tests {
    use vapors_scene::{Color, Quad, Rect, Run};

    use super::assert_kept;

    fn quads() -> Run {
        let quad = Quad::fill(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Color::from_rgba8(1, 2, 3, 255),
        );
        Run::Quads(vec![quad, quad])
    }

    #[test]
    fn equal_bytes_pass() {
        // Given
        let bytes = vec![7; 256];

        // When / Then
        assert_kept(&bytes, &bytes, (&quads(), 3));
    }

    #[test]
    #[should_panic(
        expected = "batch 3 (quads) drew kept bytes that differ from its bytes \
                               built afresh at instance 1, attribute 2 (byte 160;"
    )]
    fn the_first_differing_byte_is_named_by_instance_and_attribute() {
        // Given: the second quad's third attribute differs
        let kept = vec![7; 256];
        let mut fresh = kept.clone();
        fresh[160] = 8;

        // When
        assert_kept(&kept, &fresh, (&quads(), 3));
    }

    #[test]
    #[should_panic(expected = "at instance 2, attribute 0 (byte 256; 256 bytes kept, 384 afresh)")]
    fn bytes_of_another_length_are_named_where_the_shorter_ends() {
        // When
        assert_kept(&[7; 256], &[7; 384], (&quads(), 0));
    }
}
