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

use crate::deadline::{Finished, run_with_deadline};
use crate::pixels::{centre_pixel, parse_rgb};
use crate::xvfb::Xvfb;

/// How long the example may take from launch to writing its PNG.
const EXAMPLE_DEADLINE: Duration = Duration::from_secs(120);

/// Run `example` from the `vapors-platform` crate, save its first frame to
/// `out`, and when `expect` is given fail unless the centre pixel is that
/// colour.
pub fn run(example: &str, out: &Path, expect: Option<&str>, example_args: &[String]) -> ExitCode {
    let expected = match expect.map(parse_rgb) {
        Some(Err(message)) => return fail(&message),
        Some(Ok(rgb)) => Some(rgb),
        None => None,
    };
    let xvfb = match Xvfb::start() {
        Ok(xvfb) => xvfb,
        Err(message) => return fail(&message),
    };
    let result = run_example(example, out, &xvfb.display, example_args);
    drop(xvfb);
    if let Err(message) = result {
        return fail(&message);
    }
    report(out, expected)
}

fn run_example(
    example: &str,
    out: &Path,
    display: &str,
    example_args: &[String],
) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command
        .args([
            "run",
            "-q",
            "-p",
            "vapors-platform",
            "--example",
            example,
            "--",
        ])
        .arg("--screenshot")
        .arg(out)
        .args(example_args)
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
        Finished::Exited { output, .. } => Err(format!("example `{example}` failed:\n{output}")),
        Finished::TimedOut => Err(format!(
            "example `{example}` did not finish within {}s; it never wrote its screenshot",
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
