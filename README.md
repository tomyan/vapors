# vapors

A GPU-rendered UI framework in Rust. Components are single files with a
Rust script, CSS and an HTML template, compiled to Rust. Renders with
wgpu on desktop and WebGPU in the browser from one implementation.

Early. The first user is a photo editor; the framework is built one
slice ahead of what that editor needs.

## Working in the repository

```
cargo xtask --help     every factory command
cargo xtask ci         what CI runs, fastest checks first
bacon                  unit tests on every save
```

The toolchain is pinned in `rust-toolchain.toml`; tool versions in
`.config/tools.toml`.

## Licence

MIT or Apache-2.0, at your option.
