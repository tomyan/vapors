//! `--off-path` beside the other flags of the command line (`args.rs`).

use crate::args::USAGE;

/// The workloads a full run picks from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkloadSet {
    /// The rows pages and the racer (`workloads.rs`).
    #[default]
    Main,
    /// The off-path pages (`off_path.rs`): `--off-path`.
    OffPath,
}

/// `--off-path` picks the workloads of the full run, which the other
/// modes replace.
pub(crate) fn refuse_off_path_with_modes(
    set: WorkloadSet,
    modes: [(&str, bool); 4],
) -> Result<(), String> {
    match modes.iter().find(|(_, given)| *given) {
        Some((mode, _)) if set == WorkloadSet::OffPath => {
            Err(format!("--off-path and {mode} cannot be combined: {USAGE}"))
        }
        _ => Ok(()),
    }
}
