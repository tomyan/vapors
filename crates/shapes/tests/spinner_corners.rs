//! The gui-bench live spinner's rounded square at saved clock times
//! (`tests/fixtures/spinner_paths.tsv`: milliseconds, a tab, the path the
//! spinner generates then; `gui-bench-app` pins the file to its generator).
//! The arc that closes the outline ends a few 1e-4 off the subpath's start
//! in lyon's cubics, which left a sliver there whose float-noise winding
//! turned the outline reflex under a 170 degree wedge.

use vapors_shapes::{FillRule, IconPath, Paint, TOLERANCE_PX, check_mesh, tessellate};

const FIXTURE: &str = include_str!("fixtures/spinner_paths.tsv");

fn rows() -> Vec<(&'static str, &'static str)> {
    FIXTURE
        .lines()
        .map(|line| line.split_once('\t').expect("a millisecond, a tab, a path"))
        .collect()
}

#[test]
fn the_spinner_at_every_saved_time_tessellates_at_every_scale() {
    // Given: the saved paths, each drawn at 1x, 2x and 4x
    let fill = Paint::Fill(FillRule::NonZero);
    for (ms, data) in rows() {
        let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{ms} ms {data:?}: {e}"));
        for scale in [1.0, 2.0, 4.0] {
            // When
            let site = format!("spinner at {ms} ms, scale {scale}");
            let mesh = tessellate(&site, &path, fill, TOLERANCE_PX / scale);

            // Then
            let mesh = mesh.unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(check_mesh(&mesh.triangles), Ok(()), "{site}");
        }
    }
}
