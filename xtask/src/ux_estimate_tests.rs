//! Estimates from above, and the caps that refuse a run before it is
//! sent.

use super::*;

fn caps() -> Caps {
    Caps {
        input_tokens: 60_000,
        output_tokens: 12_000,
        cost_usd: 0.30,
        daily_usd: 3.00,
    }
}

fn prices() -> Prices {
    Prices {
        input: 1.25,
        cached: 0.125,
        output: 10.0,
        source: "read 2026-10-26".to_owned(),
    }
}

#[test]
fn text_is_counted_at_three_bytes_a_token_rounded_up() {
    assert_eq!(text_tokens(0), 0);
    assert_eq!(text_tokens(7), 3);
}

#[test]
fn the_phone_frame_at_high_detail_takes_the_larger_count() {
    // Given 412 x 915: two tiles (425) against 13 x 29 = 377 patches,
    // times 1.2 (453)

    // When
    let tokens = image_tokens((412, 915), "high");

    // Then
    assert_eq!(tokens, 453);
}

#[test]
fn the_desktop_frame_at_high_detail_takes_the_larger_count() {
    // Given 1440 x 900: six tiles once the short side is 768 (1105),
    // against 45 x 29 = 1305 patches, times 1.2 (1566)

    // When
    let tokens = image_tokens((1440, 900), "high");

    // Then
    assert_eq!(tokens, 1566);
}

#[test]
fn a_large_image_at_high_detail_is_held_to_the_patch_budget() {
    // Given 4000 x 4000 px, 15,625 patches before the budget of 2,500

    // When
    let tokens = image_tokens((4000, 4000), "high");

    // Then: the budget times 1.2, above the 1,445 of 8 tiles
    assert_eq!(tokens, 3000);
}

#[test]
fn a_low_detail_image_fits_512_px_before_its_patches_are_counted() {
    // Given 1440 x 900, fitted to 512 x 320: 16 x 10 patches, times 1.2

    // When
    let tokens = image_tokens((1440, 900), "low");

    // Then
    assert_eq!(tokens, 192);
}

#[test]
fn the_worst_case_counts_every_output_token() {
    // Given
    let estimate = Estimate {
        text: 20_000,
        images: 4_000,
    };

    // When
    let worst = estimate.worst_cost(&caps(), &prices());

    // Then: 24,000 x $1.25 + 12,000 x $10, per million
    assert!((worst - 0.15).abs() < 1e-9, "{worst}");
}

#[test]
fn an_input_over_its_cap_is_refused() {
    // Given
    let estimate = Estimate {
        text: 59_000,
        images: 2_000,
    };

    // When
    let refused = estimate.within(&caps(), &prices()).expect_err("refused");

    // Then
    assert!(
        refused.contains("61000 input tokens, over ux.max_input_tokens 60000"),
        "{refused}"
    );
}

#[test]
fn a_worst_case_over_the_run_cap_is_refused() {
    // Given
    let mut caps = caps();
    caps.cost_usd = 0.10;

    // When
    let refused = Estimate {
        text: 20_000,
        images: 4_000,
    }
    .within(&caps, &prices())
    .expect_err("refused");

    // Then
    assert!(
        refused.contains("could cost $0.1500, over ux.max_cost_usd $0.10"),
        "{refused}"
    );
}
