//! `cargo xtask setup`: make a fresh machine (or a CI runner) able to run
//! the factory, and say what was done.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::setup_install;
use crate::tools::{self, Tool};

const TOOLS_FILE: &str = ".config/tools.toml";
const HOOK_SOURCE: &str = "hooks/pre-push";

/// Install missing or mismatched tools, and the git hook unless `ci`.
pub fn run(ci: bool) -> ExitCode {
    let tools = match load_tools() {
        Ok(tools) => tools,
        Err(message) => return fail(&message),
    };
    for tool in &tools {
        if let Err(message) = ensure(tool) {
            return fail(&message);
        }
    }
    if let Err(message) = ensure_downloads() {
        return fail(&message);
    }
    if !ci && let Err(message) = install_hook() {
        return fail(&message);
    }
    ExitCode::SUCCESS
}

/// Check every pinned tool is installed at its pinned version. Used by
/// `cargo xtask ci` so a wrong tool version is a named failure.
pub fn check() -> ExitCode {
    let tools = match load_tools() {
        Ok(tools) => tools,
        Err(message) => return fail(&message),
    };
    let mut ok = true;
    for tool in &tools {
        match installed_version(tool) {
            Some(version) if version == tool.version => {}
            found => {
                ok = false;
                println!("{}", mismatch(tool, found.as_deref()));
            }
        }
    }
    if let Err(why) = check_downloads() {
        ok = false;
        println!("FAIL {why}");
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        println!("fix: run `cargo xtask setup`");
        ExitCode::FAILURE
    }
}

fn load_tools() -> Result<Vec<Tool>, String> {
    let text = fs::read_to_string(TOOLS_FILE)
        .map_err(|e| format!("could not read {TOOLS_FILE}: {e}; run from the workspace root"))?;
    tools::parse(&text).map_err(|e| e.to_string())
}

/// `[downloads]`, each fetched and verified unless already verified.
fn ensure_downloads() -> Result<(), String> {
    let home = home()?;
    for download in load_downloads()? {
        let path = crate::download_install::ensure(&download, &home)?;
        println!(
            "ok   {:<18} {} at {} (SHA-256 verified)",
            download.name,
            download.version,
            path.display()
        );
    }
    Ok(())
}

/// `[downloads]`, each installed and matching its pinned hash; never run.
fn check_downloads() -> Result<(), String> {
    let home = home()?;
    for download in load_downloads()? {
        crate::download_install::verified(&download, &home)?;
    }
    Ok(())
}

fn load_downloads() -> Result<Vec<crate::downloads::Download>, String> {
    let text = fs::read_to_string(TOOLS_FILE)
        .map_err(|e| format!("could not read {TOOLS_FILE}: {e}; run from the workspace root"))?;
    crate::downloads::parse(&text).map_err(|e| e.to_string())
}

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is unset; fix: set it".to_owned())
}

fn ensure(tool: &Tool) -> Result<(), String> {
    match installed_version(tool) {
        Some(version) if version == tool.version => {
            println!("ok   {:<18} {version}", tool.crate_name);
            Ok(())
        }
        found => {
            println!("{}", setup_install::before_install(tool, found.as_deref()));
            setup_install::install(tool)
        }
    }
}

fn mismatch(tool: &Tool, found: Option<&str>) -> String {
    let found = found.unwrap_or("not installed");
    format!(
        "FAIL {:<18} expected {} found {found}",
        tool.crate_name, tool.version
    )
}

fn installed_version(tool: &Tool) -> Option<String> {
    let output = Command::new(&tool.binary).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // Tools print `<name> <version>`; some add a hash or date after.
    text.split_whitespace().nth(1).map(str::to_owned)
}

fn install_hook() -> Result<(), String> {
    let hooks_dir = hooks_dir()?;
    let target = hooks_dir.join("pre-push");
    fs::create_dir_all(&hooks_dir)
        .map_err(|e| format!("could not create {}: {e}", hooks_dir.display()))?;
    fs::copy(HOOK_SOURCE, &target)
        .map_err(|e| format!("could not copy {HOOK_SOURCE} to {}: {e}", target.display()))?;
    make_executable(&target)?;
    println!("ok   pre-push hook      installed at {}", target.display());
    Ok(())
}

/// The hooks directory of this repository, which for a worktree is in
/// the main repository, not under the worktree's `.git` file.
fn hooks_dir() -> Result<PathBuf, String> {
    let output = Command::new("git")
        // Git hooks export these; a spawn here must reach only this checkout.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !output.status.success() {
        return Err("not inside a git repository; run from the repository root".into());
    }
    let common = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(Path::new(&common).join("hooks"))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("could not make {} executable: {e}", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn fail(message: &str) -> ExitCode {
    println!("FAIL setup: {message}");
    ExitCode::FAILURE
}
