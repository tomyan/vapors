//! `cargo xtask ci` as the command runs it: a share of the gate lock
//! after the quiet lock's wait and the load wait (`share.rs`), and
//! sccache's hit rate over the run.

use std::process::ExitCode;

use crate::share::{self, Sharer};
use crate::{ci, fail, report::Reporter, sccache, steps};

/// `cargo xtask ci`, sharing the gate lock so it never runs beside a
/// gate's exclusive steps; nothing inside a gate's step (`share.rs`).
pub fn run(steps: &[steps::Step], reporter: &Reporter) -> ExitCode {
    match share::unless_in_gate(Sharer::Ci) {
        Ok(_shared) => with_hit_rate(steps, reporter),
        Err(why) => fail("ci", &why),
    }
}

/// `cargo xtask ci`, then sccache's hit rate over the run; sccache must
/// answer before and after.
fn with_hit_rate(steps: &[steps::Step], reporter: &Reporter) -> ExitCode {
    let before = match sccache::stats() {
        Ok(stats) => stats,
        Err(why) => return fail("sccache", &why),
    };
    let code = ci::run(steps, reporter);
    match sccache::stats() {
        Ok(after) => match sccache::hit_rate(before, after) {
            Ok(line) => println!("{line}"),
            Err(why) => return fail("sccache", &why),
        },
        Err(why) => return fail("sccache", &why),
    }
    code
}
