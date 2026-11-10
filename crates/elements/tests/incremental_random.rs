//! A seeded randomized differential test of incremental updates (F2c
//! S7 review). Each step makes one random change and updates; then the
//! document forgets everything kept between updates and updates from
//! scratch; `support::oracle` compares the two. CI runs `SEEDS` seeds; set
//! `VAPORS_INCREMENTAL_SEEDS=<n>` to run more. A failure names the
//! seed, the step and the change.

mod support;

use support::changes::{Random, change, page};
use support::oracle::{assert_matches_full, snapshot};
use support::viewport;

const SEEDS: u64 = 6;
const STEPS: usize = 40;

fn run(seed: u64) {
    let mut random = Random(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let doc = page(&mut random);
    let mut viewport = viewport(640.0, 400.0);
    snapshot(&doc, viewport);
    for step in 0..STEPS {
        let what = change(&doc, &mut random, &mut viewport, step);
        assert_matches_full(
            &doc,
            viewport,
            &format!("seed {seed}, step {step}: {what}; reproduce: VAPORS_INCREMENTAL_SEED={seed}"),
        );
    }
}

#[test]
fn incremental_updates_equal_updates_from_scratch_for_random_changes() {
    if let Some(seed) = std::env::var("VAPORS_INCREMENTAL_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        return run(seed);
    }
    let seeds = std::env::var("VAPORS_INCREMENTAL_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(SEEDS);
    for seed in 0..seeds {
        run(seed);
    }
}
