//! Reading sccache's statistics, and the hit rate over a run.

use super::*;

/// `--show-stats --stats-format json` with these counts and cache.
fn stats_json(hits: u64, misses: u64, location: &str, max: u64) -> String {
    format!(
        r#"{{"stats":{{"compile_requests":9,"cache_read_errors":0,"cache_write_errors":0,"cache_errors":{{"counts":{{}},"adv_counts":{{}}}},"cache_hits":{{"counts":{{"Rust":{hits}}},"adv_counts":{{}}}},"cache_misses":{{"counts":{{"Rust":{misses}}},"adv_counts":{{}}}}}},"cache_location":"{location}","max_cache_size":{max},"version":"0.18.0"}}"#
    )
}

const LOCAL: &str = r#"Local disk: \"/home/x/.cache/sccache\""#;

#[test]
fn hits_and_misses_are_read_from_the_json_stats() {
    // When
    let stats = parse(&stats_json(7, 3, LOCAL, CACHE_SIZE_BYTES)).expect("stats");

    // Then
    assert_eq!(
        stats,
        Stats {
            hits: 7,
            misses: 3,
            errors: 0
        }
    );
}

#[test]
fn a_local_cache_of_another_size_is_refused_naming_the_fix() {
    // When
    let refusal = parse(&stats_json(0, 0, LOCAL, 10 * 1024 * 1024 * 1024)).expect_err("10 GiB");

    // Then
    assert!(refusal.contains("not 50G"), "{refusal}");
    assert!(refusal.contains("sccache --stop-server"), "{refusal}");
}

#[test]
fn a_remote_cache_is_not_held_to_the_local_cap() {
    assert!(parse(&stats_json(1, 1, "GHA, name: sccache-", 0)).is_ok());
}

#[test]
fn stats_of_another_shape_are_refused() {
    assert!(parse("sccache: error: connection refused").is_err());
    assert!(parse(r#"{"stats":{}}"#).is_err());
}

#[test]
fn the_hit_rate_is_over_the_run_alone() {
    // Given
    let before = Stats {
        hits: 100,
        misses: 50,
        errors: 2,
    };
    let after = Stats {
        hits: 190,
        misses: 60,
        errors: 2,
    };

    // When
    let line = hit_rate(before, after).expect("no new errors");

    // Then
    assert!(
        line.starts_with("sccache: 90% hits (90 of 100 compilations"),
        "{line}"
    );
    assert!(
        hit_rate(after, after)
            .expect("none")
            .contains("no compilations")
    );
}

#[test]
fn every_build_xtask_starts_goes_through_sccache_without_incremental() {
    // Given
    let mut command = std::process::Command::new("cargo");

    // When
    command.envs(ENV);

    // Then
    let env: Vec<_> = command
        .get_envs()
        .map(|(k, v)| (k.to_owned(), v.map(ToOwned::to_owned)))
        .collect();
    assert!(env.contains(&("RUSTC_WRAPPER".into(), Some("sccache".into()))));
    assert!(env.contains(&("CARGO_INCREMENTAL".into(), Some("0".into()))));
}

#[test]
fn cache_errors_during_the_run_fail_it() {
    // Given: sccache compiled one crate without the cache after an error
    let before = Stats {
        hits: 1,
        misses: 1,
        errors: 0,
    };
    let after = Stats {
        hits: 1,
        misses: 2,
        errors: 1,
    };

    // When
    let refusal = hit_rate(before, after).expect_err("an error");

    // Then
    assert!(refusal.contains("1 cache errors this run"), "{refusal}");
}
