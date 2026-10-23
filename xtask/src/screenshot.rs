//! `cargo xtask screenshot`: run a windowed example under a virtual X
//! display and save what it drew as a PNG. The way an agent on a headless
//! box looks at a window.
//!
//! Every wait here has a deadline. The numbers are first guesses and are
//! replaced from measurement (docs/budgets.md) once there are runs to
//! measure.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::config::Config;
use crate::deadline::{Finished, run_with_deadline};
use crate::pixels::{centre_pixel, parse_rgb};
use crate::screenshot_package;
use crate::xvfb::Xvfb;

/// How long the example may take from launch to writing its PNG.
const EXAMPLE_DEADLINE: Duration = Duration::from_secs(120);

/// What to run: an example or a binary of the configured package.
pub struct Target {
    /// Example or binary name.
    pub name: String,
    /// True for `--bin`, false for `--example`.
    pub is_binary: bool,
}

/// Run `target` from the package in `[screenshot] packages` that has it, save its
/// first frame to `out`, and when `expect` is given fail unless the centre
/// pixel is that colour.
pub fn run(target: &Target, out: &Path, expect: Option<&str>, example_args: &[String]) -> ExitCode {
    let package = match package_for(target) {
        Ok(package) => package,
        Err(message) => return fail(&message),
    };
    let expected = match expect.map(parse_rgb) {
        Some(Err(message)) => return fail(&message),
        Some(Ok(rgb)) => Some(rgb),
        None => None,
    };
    let xvfb = match Xvfb::start() {
        Ok(xvfb) => xvfb,
        Err(message) => return fail(&message),
    };
    let result = run_example(&package, target, out, &xvfb.display, example_args);
    drop(xvfb);
    if let Err(message) = result {
        return fail(&message);
    }
    report(out, expected)
}

/// The listed package that has `target`; see `screenshot_package`.
fn package_for(target: &Target) -> Result<String, String> {
    let listed = screenshot_package::packages(&Config::load().map_err(|e| e.to_string())?)?;
    screenshot_package::choose(&listed, &screenshot_package::workspace_metadata()?, target)
}

fn run_example(
    package: &str,
    target: &Target,
    out: &Path,
    display: &str,
    example_args: &[String],
) -> Result<(), String> {
    let kind = if target.is_binary {
        "--bin"
    } else {
        "--example"
    };
    let mut command = Command::new("cargo");
    command
        .args(["run", "-q", "-p", package, kind, &target.name, "--"])
        .args(example_args)
        .arg("--screenshot")
        .arg(out)
        .env("DISPLAY", display);
    if std::env::var_os("VAPORS_GPU_ADAPTER").is_none() {
        // Software rendering is what a virtual display can present.
        command.env("VAPORS_GPU_ADAPTER", "llvmpipe");
    }
    match run_with_deadline(command, EXAMPLE_DEADLINE)? {
        Finished::Exited {
            success: true,
            output,
        } => {
            for line in output
                .lines()
                .filter(|l| l.starts_with("screenshot written"))
            {
                println!("     {line}");
            }
            Ok(())
        }
        Finished::Exited { output, .. } => Err(format!("`{}` failed:\n{output}", target.name)),
        Finished::TimedOut => Err(format!(
            "`{}` did not finish within {}s; it never wrote its screenshot",
            target.name,
            EXAMPLE_DEADLINE.as_secs()
        )),
    }
}

fn report(out: &Path, expected: Option<[u8; 3]>) -> ExitCode {
    let centre = match centre_pixel(out) {
        Ok(centre) => centre,
        Err(message) => return fail(&message),
    };
    let [r, g, b] = centre;
    match expected {
        Some(want) if want != centre => fail(&format!(
            "centre pixel of {} is {r},{g},{b}; expected {},{},{}",
            out.display(),
            want[0],
            want[1],
            want[2]
        )),
        _ => {
            println!("ok   screenshot {} centre={r},{g},{b}", out.display());
            ExitCode::SUCCESS
        }
    }
}

fn fail(message: &str) -> ExitCode {
    println!("FAIL screenshot: {message}");
    println!("     reproduce: cargo xtask screenshot <example> <out.png>");
    ExitCode::FAILURE
}
