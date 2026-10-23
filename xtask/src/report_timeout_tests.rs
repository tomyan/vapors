//! The timeout report (.github/scripts/report-timeout.sh), run with a
//! stub `gh` that answers the jobs API: it tells a timeout from an early
//! cancel, names the last step to start, and fails in every case.

use std::path::PathBuf;
use std::process::{Command, Output};

const SCRIPT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../.github/scripts/report-timeout.sh"
);

/// Two running jobs on two runners; ours ran "gpu tier" last. The stub
/// fails like a refused API call when `STUB_FAIL` is set.
const STUB_GH: &str = r#"#!/bin/sh
if [ -n "$STUB_FAIL" ]; then echo "HTTP 403: Resource not accessible by integration" >&2; exit 1; fi
at=$(date -u -d "-$STUB_AGO seconds" +%Y-%m-%dT%H:%M:%SZ)
echo '{"runner_name":"other","status":"in_progress","started_at":"'$at'","steps":[]}'
echo '{"runner_name":"runner-1","status":"in_progress","started_at":"'$at'","steps":[
  {"name":"Set up job","started_at":"'$at'"},
  {"name":"gpu tier","started_at":"'$at'"},
  {"name":"upload","started_at":null},
  {"name":"report a timeout","started_at":"'$at'"}]}'
"#;

fn stub_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-report-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("stub dir");
    let gh = dir.join("gh");
    std::fs::write(&gh, STUB_GH).expect("stub");
    let made = Command::new("chmod")
        .arg("755")
        .arg(&gh)
        .status()
        .expect("chmod");
    assert!(made.success());
    dir
}

/// The script, cancelled `ago` seconds into a 9-minute job.
fn report(test: &str, ago: u32, extra: &[(&str, &str)]) -> Output {
    let stub = stub_dir(test);
    let mut command = Command::new("bash");
    command
        .arg(SCRIPT)
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", stub.display()))
        .env("GH_TOKEN", "token")
        .env("TIMEOUT_MINUTES", "9")
        .env("GITHUB_REPOSITORY", "tomyan/vapors")
        .env("GITHUB_RUN_ID", "7")
        .env("GITHUB_RUN_ATTEMPT", "1")
        .env("RUNNER_NAME", "runner-1")
        .env("STUB_AGO", ago.to_string());
    command.envs(extra.iter().copied());
    command.output().expect("the script")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_job_cancelled_at_its_timeout_says_so_naming_the_last_step() {
    // When
    let output = report("timeout", 9 * 60 + 5, &[]);

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output)
            .contains("this job hit its 9-minute timeout; the last step to start was \"gpu tier\""),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_job_cancelled_before_its_timeout_is_not_called_a_timeout() {
    // When
    let output = report("early", 120, &[]);

    // Then
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1));
    assert!(text.contains("before its 9-minute timeout"), "{text}");
    assert!(text.contains("\"gpu tier\""), "{text}");
    assert!(!text.contains("hit its"), "{text}");
}

#[test]
fn a_refused_api_call_fails_quoting_the_error() {
    // When
    let output = report("api", 600, &[("STUB_FAIL", "1")]);

    // Then
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        text.contains("repos/tomyan/vapors/actions/runs/7/attempts/1/jobs failed"),
        "{text}"
    );
    assert!(text.contains("HTTP 403"), "{text}");
}

#[test]
fn no_running_job_on_this_runner_fails_naming_the_runner() {
    // When
    let output = report("runner", 600, &[("RUNNER_NAME", "ghost")]);

    // Then
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1));
    assert!(text.contains("no running job on runner ghost"), "{text}");
}

#[test]
fn without_the_jobs_timeout_it_refuses_naming_the_variable() {
    // When
    let output = report("unset", 600, &[("TIMEOUT_MINUTES", "")]);

    // Then
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(output.status.code(), Some(0));
    assert!(stderr.contains("TIMEOUT_MINUTES"), "{stderr}");
}
