//! The gui-bench spinner's rounded square, turned to the angles where
//! lyon's flattened arc ends off the corner point and leaves a
//! sliver beside the outline (`fixtures/spinner_paths.txt`, one
//! `<ms> <path d>` per line, made by the app's `spinner_path`): it
//! builds, at every tolerance a dashboard may use.

use crate::mesh_check::check_mesh;
use crate::path::IconPath;
use crate::{FillRule, Paint, tessellate};

const FIXTURE: &str = include_str!("../fixtures/spinner_paths.txt");

#[test]
fn the_turned_spinner_builds_at_every_fixture_angle_and_tolerance() {
    for line in FIXTURE.lines() {
        let (ms, d) = line.split_once(' ').expect("`<ms> <d>` per line");
        let path = IconPath::parse(d).unwrap_or_else(|e| panic!("{ms} ms: {e}"));
        for tolerance in [0.01, 0.05, 0.1] {
            let paint = Paint::Fill(FillRule::NonZero);
            let built = tessellate("spinner:1", &path, paint, tolerance);
            let mesh = built.unwrap_or_else(|e| panic!("{ms} ms, tolerance {tolerance}: {e}"));
            assert_eq!(check_mesh(&mesh.triangles), Ok(()), "{ms} ms, {tolerance}");
        }
    }
}
