# gui-bench spec, vendored

`dashboard.rs`, `cli.rs` and `IDIOMS.md` are copied unchanged from
[zortax/gui-bench](https://github.com/zortax/gui-bench) at `f0644e8`
(`apps/shared/dashboard.rs`, `apps/shared/cli.rs`, `IDIOMS.md`). The
gui-bench apps declare `license = "Apache-2.0"`; the repository has no
LICENSE file at that commit.

They stay byte-identical to upstream so an upstream change applies as a
diff: `cp` the three files from a newer gui-bench checkout and review
what moved. vapors' benchmark app (`crates/gui-bench-app`, from its first
slice) includes `dashboard.rs` and `cli.rs` by `#[path]`, as the
gui-bench apps do.
