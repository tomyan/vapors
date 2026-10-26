//! Standard base64 (RFC 4648, section 4, with `=` padding): the frames
//! sent to the model, and the key's encodings the output guard looks for.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` in standard base64.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = chunk
            .iter()
            .enumerate()
            .fold(0u32, |w, (i, b)| w | u32::from(*b) << (16 - 8 * i));
        for slot in 0..4 {
            if slot <= chunk.len() {
                out.push(char::from(
                    ALPHABET[(word >> (18 - 6 * slot) & 63) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rfc_4648_test_vectors_encode_as_published() {
        let vectors = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, encoded) in vectors {
            assert_eq!(encode(plain.as_bytes()), encoded, "{plain}");
        }
    }

    #[test]
    fn high_bytes_use_the_last_two_characters() {
        assert_eq!(encode(&[0xfb, 0xff]), "+/8=");
    }
}
