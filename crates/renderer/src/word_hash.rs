//! The hash uniform values are found by (`uniform_slots.rs`).

/// Hashes a uniform value eight bytes at a time (an `FxHash`-style
/// multiply and rotate): a value is up to a kilobyte, looked up on every
/// draw, and holds no input an attacker chooses. Zero bytes hash to 0.
pub(crate) fn word_hash(bytes: &[u8]) -> u64 {
    let mut hasher = WordHasher::default();
    std::hash::Hasher::write(&mut hasher, bytes);
    hasher.0
}

pub(crate) type WordHashing = std::hash::BuildHasherDefault<WordHasher>;

#[derive(Default)]
pub(crate) struct WordHasher(u64);

impl std::hash::Hasher for WordHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for &word in words {
            self.add(u64::from_le_bytes(word));
        }
        for &byte in rest {
            self.add(u64::from(byte));
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

impl WordHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}
