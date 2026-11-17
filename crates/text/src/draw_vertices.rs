//! The text pipeline's vertex bytes (`draw.rs`): each vertex's floats,
//! its row's index and its frame's entry, padded as the kept buffer takes
//! them.

use crate::pipeline::{FLOATS_PER_VERTEX, VERTEX_SIZE};

/// Little-endian bytes of vertex floats, each vertex in row 0 of frame
/// entry 0 (the page), padded
/// to a multiple of 16 bytes as the kept buffer takes them
/// (`vapors-renderer`'s retained.rs); the padding, under one vertex, is
/// never drawn (`vertex_count` rounds down).
pub(crate) fn vertex_bytes(floats: &[f32]) -> Vec<u8> {
    rowed_vertex_bytes(floats, (&[floats.len() / FLOATS_PER_VERTEX], 0, &[]))
}

/// As [`vertex_bytes`], every vertex in the transform table's entry
/// `frame`, draw `i`'s `vertices[i]` vertices in row `rows[i]` of the
/// batch's range, or every vertex in row 0 for an empty `rows`.
///
/// # Panics
///
/// When the floats are not whole vertices, the draws' vertices are not
/// the floats', or `rows` is neither empty nor one row a draw inside the
/// table (`vapors_renderer::assert_rows`).
pub(crate) fn rowed_vertex_bytes(
    floats: &[f32],
    (vertices, frame, rows): (&[usize], u32, &[u32]),
) -> Vec<u8> {
    assert!(
        floats.len().is_multiple_of(FLOATS_PER_VERTEX),
        "{} text vertex floats, not a whole number of {FLOATS_PER_VERTEX}-float vertices: the \
         row index would land inside one (draw.rs); reproduce with `cargo xtask test`",
        floats.len()
    );
    let total: usize = vertices.iter().sum();
    assert_eq!(
        total * FLOATS_PER_VERTEX,
        floats.len(),
        "text: the draws' vertices are the floats': each draw's count is six for each \
         inked glyph or icon mask (quads.rs `vertices_per_draw`); reproduce with `cargo xtask \
         test`"
    );
    vapors_renderer::assert_rows("text", vertices.len(), rows);
    let mut chunks = floats.chunks(FLOATS_PER_VERTEX);
    let mut bytes = Vec::new();
    for (at, &count) in vertices.iter().enumerate() {
        let row = rows.get(at).copied().unwrap_or(0);
        for vertex in chunks.by_ref().take(count) {
            bytes.extend(vertex.iter().flat_map(|f| f.to_le_bytes()));
            bytes.extend(row.to_le_bytes());
            bytes.extend(frame.to_le_bytes());
        }
    }
    bytes.resize(bytes.len().next_multiple_of(16), 0);
    bytes
}

/// How many vertices `bytes` bytes hold.
pub(crate) fn vertex_count(bytes: u64) -> u32 {
    u32::try_from(bytes / VERTEX_SIZE).unwrap_or_else(|_| {
        panic!(
            "{bytes} bytes of text vertices in one draw, more vertices than fit u32: split the \
             run into smaller batches (draw.rs); reproduce with `cargo xtask test`"
        )
    })
}

#[cfg(test)]
#[path = "draw_vertices_tests.rs"]
mod tests;
