//! The debug build's check on kept batch bytes (`painter_kept.rs`):
//! on every draw from kept bytes, the batch's bytes built afresh equal
//! those kept.

use vapors_renderer::{ClosedFormPipeline, MeshPipeline, QuadPipeline, ShadowPipeline};
use vapors_scene::Run;

/// Panics, naming batch `at`, the instance and the attribute (vertex,
/// for image and surface quads) of the first byte that differs, when
/// `kept` is not `fresh`.
pub(super) fn assert_kept(kept: &[u8], fresh: &[u8], (run, at): (&Run, usize)) {
    if kept == fresh {
        return;
    }
    let (kind, stride, (part, part_size)) = described(run);
    let mut pairs = kept.iter().zip(fresh);
    let first = pairs.position(|(kept, fresh)| kept != fresh);
    let byte = first.unwrap_or_else(|| kept.len().min(fresh.len()));
    panic!(
        "batch {at} ({kind}) drew kept bytes that differ from its bytes built afresh at \
         instance {}, {part} {} (byte {byte}; {} bytes kept, {} afresh): a key was kept for \
         a batch whose run changed (kept_batches_keys.rs), or text bytes were kept for \
         moved glyphs or another layout (painter_kept_text.rs); reproduce with `cargo xtask test`",
        byte / stride,
        byte % stride / part_size,
        kept.len(),
        fresh.len()
    );
}

/// `run`'s kind, the bytes of one instance (a quad's six vertices for
/// images, surfaces, glyphs and icon masks), and what the parts of one
/// are and their bytes. The instanced kinds' sizes are their pipelines'
/// own, which the pipelines build their instances to.
fn described(run: &Run) -> (&'static str, usize, (&'static str, usize)) {
    match run {
        Run::Shadows(_) => ("shadows", ShadowPipeline::INSTANCE_BYTES, ("attribute", 16)),
        Run::Quads(_) => ("quads", QuadPipeline::INSTANCE_BYTES, ("attribute", 16)),
        Run::Shapes(_) => (
            "shapes",
            ClosedFormPipeline::INSTANCE_BYTES,
            ("attribute", 16),
        ),
        Run::Meshes(_) => ("meshes", MeshPipeline::INSTANCE_BYTES, ("attribute", 16)),
        Run::Images(_) => ("images", 96, ("vertex", 16)),
        Run::Surfaces(_) => ("surfaces", 96, ("vertex", 16)),
        Run::Texts(_) => ("texts", 192, ("vertex", 32)),
        Run::Icons(_) => ("icons", 192, ("vertex", 32)),
    }
}

#[cfg(test)]
mod tests {
    use vapors_renderer::QuadPipeline;
    use vapors_scene::{Color, Quad, Rect, Run};

    use super::assert_kept;

    fn quads() -> Run {
        let quad = Quad::fill(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Color::from_rgba8(1, 2, 3, 255),
        );
        Run::Quads(vec![quad, quad])
    }

    fn quads_of(run: &Run) -> Vec<Quad> {
        match run {
            Run::Quads(quads) => quads.clone(),
            other => unreachable!("a quads run, not {other:?}"),
        }
    }

    #[test]
    fn equal_bytes_pass() {
        // Given
        let bytes = QuadPipeline::bytes(&quads_of(&quads()));

        // When / Then
        assert_kept(&bytes, &bytes, (&quads(), 3));
    }

    #[test]
    #[should_panic(
        expected = "batch 3 (quads) drew kept bytes that differ from its bytes \
                               built afresh at instance 1, attribute 2 (byte 176;"
    )]
    fn the_first_differing_byte_is_named_by_instance_and_attribute() {
        // Given: the second quad's third attribute differs, in the bytes
        // the quad pipeline builds
        let kept = QuadPipeline::bytes(&quads_of(&quads()));
        let mut fresh = kept.clone();
        fresh[QuadPipeline::INSTANCE_BYTES + 32] ^= 1;

        // When
        assert_kept(&kept, &fresh, (&quads(), 3));
    }

    #[test]
    #[should_panic(
        expected = "batch 1 (images) drew kept bytes that differ from its bytes \
                               built afresh at instance 1, vertex 3 (byte 144;"
    )]
    fn an_image_quads_first_differing_byte_is_named_by_quad_and_vertex() {
        // Given: two image quads, the second's fourth vertex differs
        let kept = vec![7; 192];
        let mut fresh = kept.clone();
        fresh[144] = 8;

        // When
        assert_kept(&kept, &fresh, (&Run::Images(Vec::new()), 1));
    }

    #[test]
    #[should_panic(
        expected = "batch 2 (texts) drew kept bytes that differ from its bytes \
                               built afresh at instance 1, vertex 1 (byte 224;"
    )]
    fn a_text_runs_first_differing_byte_is_named_by_glyph_and_vertex() {
        // Given: two glyph quads, the second's second vertex differs
        let kept = vec![7; 384];
        let mut fresh = kept.clone();
        fresh[224] = 8;

        // When
        assert_kept(&kept, &fresh, (&Run::Texts(Vec::new()), 2));
    }

    #[test]
    #[should_panic(expected = "at instance 2, attribute 0 (byte 288; 288 bytes kept, 432 afresh)")]
    fn bytes_of_another_length_are_named_where_the_shorter_ends() {
        // When: two quads kept, three built afresh
        assert_kept(&[7; 288], &[7; 432], (&quads(), 0));
    }
}
