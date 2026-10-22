//! `cargo xtask serve`: put `target/web` on the tailnet over https with
//! `tailscale serve` and print the URL, so a phone can open it.

use std::process::ExitCode;

use crate::tailscale::{Target, require_tailscale, served_url, start_serving};
use crate::web::DIST;

/// Serve the dist directory and print its URL.
pub fn run() -> ExitCode {
    match serve() {
        Ok(url) => {
            println!("ok   serve    {url}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            println!("FAIL serve: {message}");
            println!("     reproduce: cargo xtask serve");
            ExitCode::FAILURE
        }
    }
}

fn serve() -> Result<String, String> {
    let tailscale = require_tailscale()?;
    let dist = std::env::current_dir()
        .map_err(|e| format!("no current directory: {e}"))?
        .join(DIST);
    if !dist.join("index.html").is_file() {
        return Err(format!(
            "{} has no index.html; run `cargo xtask web` first",
            dist.display()
        ));
    }
    start_serving(&tailscale, &Target::Directory(&dist))?;
    served_url(&tailscale)
}
