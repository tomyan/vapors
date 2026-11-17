//! The transform table's entry (`table.rs`, `docs/plans/flat-scene-f2.md`):
//! a 4x4 matrix. Entry 0 is the identity, held for good.

/// A transform: a 4x4 matrix, column-major, as WGSL's `mat4x4<f32>`.
pub type Matrix = [f32; 16];

/// The identity.
pub const IDENTITY: Matrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// A translation by `[dx, dy]` device pixels.
#[must_use]
pub fn translation([dx, dy]: [f32; 2]) -> Matrix {
    let mut matrix = IDENTITY;
    (matrix[12], matrix[13]) = (dx, dy);
    matrix
}

impl crate::table_entries::EntryValue for Matrix {
    const BYTES: u64 = 64;
    const FIRST: Self = IDENTITY;
    // a frame entry given back keeps its value: F2 rebuilds the bytes that
    // name an entry when its index changes (`flat-scene-f2.md`)
    const POISON: Option<Self> = None;

    fn write_bytes(&self, out: &mut Vec<u8>) {
        out.extend(self.iter().flat_map(|v| v.to_le_bytes()));
    }

    fn holding(limit: &'static str) -> crate::Holding {
        crate::Holding::TransformEntries { limit }
    }
}

#[cfg(test)]
#[path = "transform_entries_tests.rs"]
mod tests;
