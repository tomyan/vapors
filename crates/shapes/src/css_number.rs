//! A CSS `<number>` as SVG's attributes take it: the one reading of a
//! bare number for the elements' checks and the compiler's (an svg's
//! `width` and `viewBox`, a `<rect>`'s `x`).

/// `token` as a CSS `<number>`, finite as an f32; `None` otherwise.
/// `f32::from_str` also takes `16.`, `1.e1` and `inf`, which Chromium
/// drops from a `width` or `viewBox`, so the shape is checked first.
#[must_use]
pub fn css_number(token: &str) -> Option<f32> {
    if !is_css_number(token) {
        return None;
    }
    token.parse::<f32>().ok().filter(|n| n.is_finite())
}

/// Whether `token` is `[+-]? (digits | digits? '.' digits)
/// ([eE] [+-]? digits)?`.
fn is_css_number(token: &str) -> bool {
    let unsigned = token.strip_prefix(['+', '-']).unwrap_or(token);
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (unsigned, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole.is_empty() || digits(whole)) && digits(fraction),
        None => digits(mantissa),
    };
    mantissa_ok && exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)))
}

fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}
