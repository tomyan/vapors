//! xtask's counts held to clippy's on a fixture: every function of
//! `xtask/fixtures/quality_probe.rs.txt` (a `.txt`, so the workspace
//! measurement does not count it) against the counts clippy gave for
//! it in `quality_probe.clippy.tsv`, written by
//! `scripts/quality-probe-clippy.sh`, which builds the fixture as a
//! crate and runs clippy with both thresholds at 1. Tolerance: none on
//! the fixture, whose constructs are the ones the count models; on the
//! whole of main the floors are in `quality_clippy.rs`.

use std::collections::BTreeMap;

use super::*;

const SOURCE: &str = include_str!("../fixtures/quality_probe.rs.txt");
const CLIPPY: &str = include_str!("../fixtures/quality_probe.clippy.tsv");

fn clippys_counts() -> BTreeMap<String, (usize, usize)> {
    CLIPPY
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next().expect("name").to_owned();
            let lines = fields.next().expect("lines").parse().expect("a number");
            let complexity = fields
                .next()
                .expect("complexity")
                .parse()
                .expect("a number");
            (name, (lines, complexity))
        })
        .collect()
}

#[test]
fn every_fixture_function_counts_as_clippy_counted_it() {
    // Given
    let expected = clippys_counts();
    let measured = measure_source(SOURCE).expect("the fixture parses");

    // When
    let found: BTreeMap<String, (usize, usize)> = measured
        .functions
        .iter()
        .map(|f| (f.name.clone(), (f.code_lines, f.complexity)))
        .collect();

    // Then: the same functions, each with clippy's two numbers
    assert_eq!(
        found.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "the fixture and its clippy counts name different functions; rerun scripts/quality-probe-clippy.sh"
    );
    for (name, want) in &expected {
        assert_eq!(found[name], *want, "{name}: (lines, complexity)");
    }
}
