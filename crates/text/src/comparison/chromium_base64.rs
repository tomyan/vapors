//! Standard base64 (RFC 4648 section 4, padded), decoding only:
//! `Page.captureScreenshot` returns its PNG that way, and the workspace
//! has no base64 crate.

/// The bytes `text` encodes.
///
/// # Errors
/// A character outside the alphabet, or a length that is not a multiple
/// of four, naming the position.
pub(super) fn decode(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err(format!(
            "base64 of length {}, not a multiple of 4",
            bytes.len()
        ));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (index, quad) in bytes.chunks(4).enumerate() {
        decode_quad(quad, &mut out)
            .map_err(|at| format!("base64: byte {} is `{}`", index * 4 + at, quad[at] as char))?;
    }
    Ok(out)
}

/// One group of four characters onto `out`; the offset of a bad one.
fn decode_quad(quad: &[u8], out: &mut Vec<u8>) -> Result<(), usize> {
    let padding = quad.iter().rev().take_while(|&&c| c == b'=').count().min(2);
    let mut word = 0u32;
    for (at, &c) in quad[..4 - padding].iter().enumerate() {
        word = word << 6 | u32::from(value(c).ok_or(at)?);
    }
    word <<= 6 * padding;
    let [_, a, b, c] = word.to_be_bytes();
    out.extend(&[a, b, c][..3 - padding]);
    Ok(())
}

fn value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn rfc_4648_test_vectors_decode() {
        // Given: section 10's vectors
        let vectors = [
            ("", ""),
            ("Zg==", "f"),
            ("Zm8=", "fo"),
            ("Zm9v", "foo"),
            ("Zm9vYg==", "foob"),
            ("Zm9vYmE=", "fooba"),
            ("Zm9vYmFy", "foobar"),
        ];

        // When / Then
        for (encoded, plain) in vectors {
            assert_eq!(decode(encoded), Ok(plain.as_bytes().to_vec()), "{encoded}");
        }
    }

    #[test]
    fn a_character_outside_the_alphabet_and_a_short_group_are_refused() {
        // Given / When / Then
        assert_eq!(decode("Zm9-"), Err("base64: byte 3 is `-`".to_owned()));
        assert_eq!(
            decode("Zm9"),
            Err("base64 of length 3, not a multiple of 4".to_owned())
        );
        assert!(decode("Z===").is_err());
    }
}
