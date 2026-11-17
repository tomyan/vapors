//! Text and icon vertices kept between frames (IU4 PR 3,
//! `docs/plans/iu4.md` section 2): a batch's vertex bytes, in device
//! pixels, built once with each glyph or icon mask they name pinned in
//! the atlas (`atlas_pins.rs`), written into the pipeline's kept buffer
//! and drawn from it on targets of any viewport, until released. Bytes
//! built at an atlas generation behind the atlas's name moved glyphs:
//! drawing them panics; build them again.

use vapors_renderer::{KeptRange, TransformTable};

use crate::atlas::{AtlasError, GlyphAtlas};
use crate::atlas_pins::AtlasPins;
use crate::draw::{RenderTarget, Vertices, rowed_vertex_bytes};
use crate::draw_icon::{IconDraw, icon_quads_counted};
use crate::pipeline::{BlendParams, TextPipeline};
use crate::quads::{TextDraw, quads_counted};

/// `floats` built with `pins` taken, the pins marked with the atlas
/// generation the build ended at. A failed build fails in its first
/// pass, before any pin is taken.
fn built<T>(
    atlas: &GlyphAtlas,
    floats: Result<T, AtlasError>,
    pins: &mut AtlasPins,
) -> Result<T, AtlasError> {
    pins.generation = atlas.generation();
    floats
}

impl TextPipeline {
    /// The vertex bytes of `texts` as [`TextPipeline::draw_texts`] draws
    /// them, and the pins on every glyph they name.
    ///
    /// # Errors
    ///
    /// As [`TextPipeline::draw_texts`]; nothing is pinned then.
    pub fn text_bytes(
        atlas: &mut GlyphAtlas,
        texts: &[TextDraw<'_>],
    ) -> Result<(Vec<u8>, AtlasPins), AtlasError> {
        Self::text_bytes_in(atlas, texts, (0, &[]))
    }

    /// As [`TextPipeline::text_bytes`], every vertex in the transform
    /// table's entry `frame`, text `i`'s in row `rows[i]` of the batch's
    /// range, or every text in row 0 for an empty `rows`.
    ///
    /// # Errors
    ///
    /// As [`TextPipeline::text_bytes`].
    ///
    /// # Panics
    ///
    /// When `rows` is neither empty nor one row a text inside the table.
    pub fn text_bytes_in(
        atlas: &mut GlyphAtlas,
        texts: &[TextDraw<'_>],
        (frame, rows): (u32, &[u32]),
    ) -> Result<(Vec<u8>, AtlasPins), AtlasError> {
        let mut pins = atlas.new_pins();
        let counted = quads_counted(atlas, texts, Some(&mut pins));
        let (floats, vertices) = built(atlas, counted, &mut pins)?;
        Ok((rowed_vertex_bytes(&floats, (&vertices, frame, rows)), pins))
    }

    /// The vertex bytes of `icons` as [`TextPipeline::draw_icons`] draws
    /// them, and the pins on every icon mask they name.
    ///
    /// # Errors
    ///
    /// As [`TextPipeline::draw_icons`]; nothing is pinned then.
    pub fn icon_bytes(
        atlas: &mut GlyphAtlas,
        icons: &[IconDraw<'_>],
    ) -> Result<(Vec<u8>, AtlasPins), AtlasError> {
        Self::icon_bytes_in(atlas, icons, (0, &[]))
    }

    /// As [`TextPipeline::icon_bytes`], every vertex in the transform
    /// table's entry `frame`, icon `i`'s in row `rows[i]`, or every icon
    /// in row 0 for an empty `rows`.
    ///
    /// # Errors
    ///
    /// As [`TextPipeline::icon_bytes`].
    ///
    /// # Panics
    ///
    /// When `rows` is neither empty nor one row an icon inside the table.
    pub fn icon_bytes_in(
        atlas: &mut GlyphAtlas,
        icons: &[IconDraw<'_>],
        (frame, rows): (u32, &[u32]),
    ) -> Result<(Vec<u8>, AtlasPins), AtlasError> {
        let mut pins = atlas.new_pins();
        let counted = icon_quads_counted(atlas, icons, Some(&mut pins));
        let (floats, vertices) = built(atlas, counted, &mut pins)?;
        Ok((rowed_vertex_bytes(&floats, (&vertices, frame, rows)), pins))
    }

    /// Writes `bytes` ([`TextPipeline::text_bytes`] or
    /// [`TextPipeline::icon_bytes`]) into the kept buffer, for
    /// [`TextPipeline::draw_kept`] in this and later frames. `None`
    /// when the buffer they need is over the device's `max_buffer_size`:
    /// the refusal is on the target's encoder.
    ///
    /// # Panics
    ///
    /// If `bytes` is empty: a batch with no ink has nothing to keep.
    pub fn keep(&mut self, target: &RenderTarget<'_>, bytes: &[u8]) -> Option<KeptRange> {
        assert!(
            !bytes.is_empty(),
            "text: kept bytes of a batch with no ink; keep its pins and draw nothing \
             (draw_kept.rs); reproduce with `cargo xtask test`"
        );
        let (frame, queue) = (target.encoder, target.queue);
        self.kept.keep(frame, queue, &self.work, bytes)
    }

    /// Uploads what changed in `atlas`'s page and draws the vertices
    /// `kept` holds, built with `pins`, into `target` with `params`, each
    /// at its frame's entry in `table` and its row's in the batch's range
    /// from `rows_base`, in one draw call; not when the params slot is
    /// refused (the refusal is on the encoder).
    ///
    /// # Panics
    ///
    /// If the atlas's generation moved on since the bytes were built
    /// (their glyphs may have moved: build them again), if `kept` was
    /// kept by another pipeline, if `table` is not the table this
    /// pipeline's kept draws bind (one table a pipeline), or as
    /// [`TextPipeline::draw`] for the atlas.
    pub fn draw_kept(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        (kept, pins): (&KeptRange, &AtlasPins),
        (params, table, rows_base): (BlendParams, &mut TransformTable, u32),
    ) {
        let (built, now) = (pins.generation(), atlas.generation());
        assert_eq!(
            built, now,
            "text: kept bytes built at atlas generation {built} drawn at {now}; build them again \
             when the generation moves on (draw_kept.rs); reproduce with `cargo xtask test`"
        );
        let id = *self.table_id.get_or_insert(table.id());
        assert_eq!(
            id,
            table.id(),
            "text: a kept draw binds transform table {}, after table {id}: a pipeline draws from \
             one table (draw_kept.rs); give each painter its own pipelines; reproduce with \
             `cargo xtask test`",
            table.id()
        );
        let Some(params) = self.prepare(target, atlas, (params, rows_base)) else {
            return;
        };
        table.prepare(target.queue);
        self.record_draw(target, Vertices::Kept(kept), (Some(table), params));
    }

    /// Gives `kept`'s bytes back. Draws of it already recorded still
    /// read them: they are reused once a newer encoder writes or draws.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn release(&mut self, kept: KeptRange) {
        self.kept.release(kept);
    }

    /// Bytes held in the kept buffer.
    #[must_use]
    pub fn retained_bytes(&self) -> u64 {
        self.kept.live_bytes()
    }
}
