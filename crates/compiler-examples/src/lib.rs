//! Worked `.wisp` components, compiled by `build.rs` with
//! `vapors_compiler::build::compile_dir("src")`. Each file under `src`
//! becomes a module here: `counter.wisp` is [`counter::Counter`].

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));
