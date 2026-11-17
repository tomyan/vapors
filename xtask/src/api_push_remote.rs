//! `request_push`'s way out: the checked commit pushed from this call's
//! bare repository, and the PR opened. The push alone is given the
//! host's GitHub credential (`gh auth git-credential`), with git's
//! terminal prompt off and a deadline, so it can neither stall on a
//! prompt nor read any other config (review of slice 4, H2). The PR's
//! title and body are checked by `api_push_text`.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// The push to GitHub: a slice's pack is well under a megabyte, and a
/// push from this host takes a few seconds; 120 s is room for a slow
/// network before the push is declared hung.
pub const PUSH_DEADLINE: Duration = Duration::from_secs(120);

/// Each gh call (list the open PR, create one): an API round trip of a
/// second or two from this host; 60 s, as the deny step's fetch.
pub const GH_DEADLINE: Duration = Duration::from_secs(60);

/// The credential helper the push is given: the host's gh login.
const HELPER: &str = "!gh auth git-credential";

/// `tip` pushed from `bare` to `branch` on the main repository's
/// `origin`, without the pre-push hook.
///
/// # Errors
///
/// The remote not read or not https, or the push failing or passing
/// [`PUSH_DEADLINE`], naming why.
pub fn pushed(main: &Path, bare: &Path, (tip, branch): (&str, &str)) -> Result<(), String> {
    let deadline = crate::api_push_fetch::LOCAL_GIT_DEADLINE;
    let remote = crate::api_timed::git_within(main, &["remote", "get-url", "origin"], deadline)?;
    let home = crate::api_host::home()?;
    pushed_to(bare, (&remote, &home), (tip, branch), HELPER)
}

/// [`pushed`] to `remote` with credential `helper`, gh finding its login
/// under `home`: the tests give a local remote and helper.
///
/// # Errors
///
/// As [`pushed`].
pub fn pushed_to(
    bare: &Path,
    (remote, home): (&str, &Path),
    (tip, branch): (&str, &str),
    helper: &str,
) -> Result<(), String> {
    let origin = origin_of(remote)?;
    let mut push = Command::new("git");
    push.env_clear().current_dir(bare);
    push.envs([
        ("PATH", "/usr/bin:/bin"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ]);
    push.env("HOME", home);
    let helper_for = format!("credential.{origin}.helper={helper}");
    push.args([
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "credential.helper=",
        "-c",
        &helper_for,
    ]);
    // Relaxed guard, declared (the lead, answer 8): the pre-push hook
    // would run this branch's code (build scripts, tests, its xtask) on
    // the host, so it is skipped; the merge gate runs everything on the
    // host before any merge. Retired when the hook can run its checks
    // in a sandbox.
    let refspec = format!("{tip}:refs/heads/{branch}");
    push.args(["push", "--no-verify", "--quiet", remote, &refspec]);
    let ended = crate::api_timed::within(push, PUSH_DEADLINE)?;
    if !ended.status.success() {
        return Err(format!(
            "pushing {branch}: {}",
            String::from_utf8_lossy(&ended.stderr).trim()
        ));
    }
    Ok(())
}

/// `scheme://host[:port]` of an http(s) remote, for the helper's key.
fn origin_of(remote: &str) -> Result<&str, String> {
    let after = ["https://", "http://"]
        .iter()
        .find_map(|scheme| remote.strip_prefix(scheme).map(|rest| (scheme.len(), rest)));
    let Some((scheme, rest)) = after else {
        return Err(format!(
            "origin {remote:?} is not an http(s) remote: request_push pushes over https"
        ));
    };
    Ok(&remote[..scheme + rest.find('/').unwrap_or(rest.len())])
}

/// The open PR for `branch`, or the one `gh pr create` makes; its URL,
/// from gh's own output (review of slice 4, M5).
///
/// # Errors
///
/// gh failing or passing [`GH_DEADLINE`], or printing no URL.
pub fn opened(main: &Path, branch: &str, (title, body): (&str, &str)) -> Result<String, String> {
    let open = gh(
        main,
        &[
            "pr", "list", "--head", branch, "--state", "open", "--json", "url", "-q", ".[0].url",
        ],
    )?;
    if !open.is_empty() {
        return Ok(open);
    }
    let created = gh(
        main,
        &[
            "pr", "create", "--base", "main", "--head", branch, "--title", title, "--body", body,
        ],
    )?;
    created
        .lines()
        .rev()
        .find(|line| line.starts_with("https://"))
        .map(str::to_owned)
        .ok_or_else(|| format!("gh pr create printed no PR URL: {created:?}"))
}

fn gh(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut gh = Command::new("gh");
    gh.args(args).current_dir(dir);
    for hook_variable in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
        gh.env_remove(hook_variable);
    }
    let ended = crate::api_timed::within(gh, GH_DEADLINE)?;
    if !ended.status.success() {
        return Err(format!(
            "gh {}: {}",
            args[..2].join(" "),
            String::from_utf8_lossy(&ended.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ended.stdout).trim().to_owned())
}

/// Refuses a PR title or body `api_push_text` would not publish.
///
/// # Errors
///
/// The first refusal, naming the part.
pub fn pr_text(title: &str, body: &str) -> Result<(), String> {
    crate::api_push_text::published("the PR title", title)?;
    crate::api_push_text::published("the PR body", body)
}

#[cfg(test)]
#[path = "api_push_remote_tests.rs"]
mod tests;
