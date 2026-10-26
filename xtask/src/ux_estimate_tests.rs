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

fn model(name: &str) -> &'static Model {
    crate::ux_models::find(name).expect("listed")
}

#[test]
fn text_is_counted_at_one_token_per_byte() {
    assert_eq!(text_tokens(0), 0);
    assert_eq!(text_tokens(7), 7);
}

#[test]
fn a_tile_model_counts_its_base_and_tiles() {
    // Given 412 x 915 on gpt-4o-mini: two tiles at 2,833 + 5,667 each

    // When
    let tokens = image_tokens(model("gpt-4o-mini"), (412, 915), "high");

    // Then
    assert_eq!(tokens, 14_167);
}

#[test]
fn a_tile_model_at_low_detail_counts_its_base_alone() {
    assert_eq!(image_tokens(model("gpt-5"), (1440, 900), "low"), 70);
}

#[test]
fn a_patch_model_counts_patches_times_its_multiplier() {
    // Given 1440 x 900 on gpt-5.6-terra: 45 x 29 = 1,305 patches, times 1.2

    // When
    let tokens = image_tokens(model("gpt-5.6-terra"), (1440, 900), "high");

    // Then
    assert_eq!(tokens, 1566);
}

#[test]
fn a_patch_model_holds_a_large_image_to_its_budget() {
    // Given 4000 x 4000, fitted to 2048: 64 x 64 = 4,096 patches, over 2,500

    // When
    let tokens = image_tokens(model("gpt-5.4"), (4000, 4000), "high");

    // Then: the budget times 1.2
    assert_eq!(tokens, 3000);
}

#[test]
fn a_patch_model_at_low_detail_uses_its_own_fit() {
    // Given 1440 x 900: gpt-5.6-terra fits it to 512 x 320 (16 x 10
    // patches); gpt-5.4 keeps it under 2048 (45 x 29) with a 6,144 budget

    // When
    let terra = image_tokens(model("gpt-5.6-terra"), (1440, 900), "low");
    let five_four = image_tokens(model("gpt-5.4"), (1440, 900), "low");

    // Then
    assert_eq!((terra, five_four), (192, 1566));
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
