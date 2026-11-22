//! `cargo xtask test` as the command runs it: holding a share of the gate
//! lock without a ci slot, so a gate's exclusive steps wait for it and it
//! waits for them; nothing inside a gate's step (`share.rs`).

use std::process::ExitCode;

use crate::cli::Tier;
use crate::report::Reporter;
use crate::share::{self, Sharer};
use crate::{ci, fail, steps};

/// `cargo xtask test --tier <tier>`.
pub fn run(tier: Tier, reporter: &Reporter) -> ExitCode {
    let _share = match share::unless_in_gate(Sharer::Test) {
        Ok(share) => share,
        Err(why) => return fail("test", &why),
    };
    let steps = match tier {
        Tier::Unit => steps::unit_tests(),
        Tier::Gpu => steps::gpu_tests(),
        Tier::Host => steps::host_tests(),
        Tier::Pmu => steps::pmu_tests(),
    };
    ci::run(&steps, reporter)
}
