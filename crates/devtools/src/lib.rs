//! Chromium's `DevTools` protocol for checks and oracles: the oracle
//! runner's accessibility trees, input events and frames
//! (docs/design/11-testing-oracles.md, Q1), and touche's walkthrough.
//!
//! Chromium is started with `--remote-debugging-port=0` and a profile of
//! its own. [`Session::attach`] reads the port it chose from the profile,
//! finds the page target and connects. Every wait has a deadline the
//! caller gives. The design named a pipe (`--remote-debugging-pipe`),
//! which needs file descriptors 3 and 4 set in the child: that takes
//! `unsafe` in Rust, which the workspace forbids, so the port is used.

mod discover;
mod session;

pub use discover::{page_socket, port};
pub use session::Session;
