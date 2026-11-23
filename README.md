# vapors

A GPU-rendered UI framework in Rust. Components are single files with a
Rust script, CSS and an HTML template, compiled to Rust. Renders with
wgpu on desktop and WebGPU in the browser from one implementation.

Early. The first user is a photo editor; the framework is built one
slice ahead of what that editor needs.

Documentation: [docs/README.md](docs/README.md).

## Feedback and contributions

We do not accept unsolicited code contributions. Use [GitHub Issues](https://github.com/tomyan/vapors/issues) for ideas, feedback and bug reports. For a bug, include what you tried, what you expected and what happened.

## Verification

Vapors uses independent review and a local merge gate run on the proposed revision. The gate posts its result on the pull request. GitHub Actions also runs checks on pushes. Hosted checks are advisory for merging, so consult the individual job results for the platform you intend to use. See [verification details](docs/review.md).

## Working in the repository

```
cargo xtask --help     every factory command
cargo xtask ci         what CI runs, fastest checks first
bacon                  unit tests on every save
```

The toolchain is pinned in `rust-toolchain.toml`; tool versions in
`.config/tools.toml`.

## Licence

MIT; see `LICENSE`.
