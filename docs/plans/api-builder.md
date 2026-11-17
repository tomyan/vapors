# Sandboxed API builder (`cargo xtask api-build`)

A slice built by a headless `claude -p --bare` session billed to API
credits, run so the session can change only its own clone and reach only
the Anthropic API and the harness. Approved by Tom 2026-11-17; the design
answers are the lead's (queue rev 8, item 3). It replaces
`~/.cache/handoffs/api-builder.sh`, which ran the session with the key in
its environment, the network open and the whole home directory writable.

## Shape

- **Outside** (trusted, built from main): `cargo xtask api-build <brief>
  <budget>` makes the builder's clone, starts the harness server on one
  Unix socket, runs the session in the sandbox, and handles everything
  that needs the network or the key.
- **The sandbox**: bwrap with its own network, pid, ipc, uts and cgroup
  namespaces, its own session, a cleared environment, stdin /dev/null,
  and no inherited descriptor (xtask refuses to start one while it holds
  any); the home directory a tmpfs; writable only the clone, its target
  directory, its cargo home and the sandboxes' own sccache directory
  (`~/.cache/sccache-sandbox`), never the host's shared one, whose
  entries host builds link; the toolchain, `~/.cargo`'s registry, the
  factory's pinned downloads, the CLI and the main repository's objects
  (the clone's alternates) read-only; the GPU's render nodes, not its
  card nodes, and `/sys` read-only for the GPU tier. Cargo runs
  `--offline`.
- **The one way in**: the session's `ANTHROPIC_BASE_URL` is
  `http://127.0.0.1:<port>`, where `xtask api-relay-inner` (the trusted binary,
  read-only inside) relays each connection to the socket. The session's
  key is a dummy.
- **The harness server** (outside, on the socket): forwards
  `POST /v1/messages` and `/v1/messages/count_tokens` to the API with the
  real key (curl, the key on its stdin), and refuses and logs anything
  else; `HEAD /api/hello` waits on the lead's decision (F4). It sums the usage in each
  response against the budget. The MCP tools `ask_lead` and
  `request_push` (served inside by `xtask api-relay-inner`'s MCP mode) post to it.

## Smoke test (2026-11-17)

`claude` 2.1.323, `-p --bare`, a dummy key and the base URL set to a
loopback recorder: it honours both, sends the dummy key as `x-api-key`,
and makes `HEAD /api/hello` once (no key) and
`POST /v1/messages?beta=true`, retried against a 401 until the 60 s
timeout. Nothing else reached the base URL.

## The clone is untrusted

Once a session has run, everything in the clone is the session's to
write: `.git/config` (`core.fsmonitor`, `core.sshCommand`,
`core.hooksPath`), `.git/hooks`, `.cargo/config.toml`, build scripts, and
the `target` symlink. So after a session starts, the host never runs
git, cargo or any other tool with the clone as its working directory or
repository. The way back (slice 4) is a fetch into a separate bare
repository made for the push, with hooks off (`-c core.hooksPath=/dev/null`),
`-c protocol.file.allow=always`, `--no-recurse-submodules`, and a cleared
environment; the diff is checked there, `cargo xtask ci` runs on it in a
fresh sandbox, and the push goes from the bare repository. A slice 4
probe: a clone whose `.git/config` sets `core.fsmonitor` to a script that
writes a marker leaves no marker after the fetch. The fences' own push
probe runs host git in the clone only before any session, on a clone
`make` has just created.

## Fences, proven on every run

Before a session starts, `api-build` runs each probe inside the sandbox
and refuses to start if any escape succeeds. Each probe's action is run
outside first as its control, so a probe that cannot detect its escape
fails the run instead of passing it: write outside the clone (through
the hidden home, into the host's shared sccache directory, and into a
read-only mount), read the caller's environment, reach the network
directly, read the key, see a process outside, and push. An inside run
that never says the sentinel did not run in the sandbox, and fails the
run. What only an inside check can show is checked too: namespaces and
a session of its own, only the bound home entries, only the slice's
branch and `origin/main` in the clone, and no inherited descriptor. The way in is probed
too: a request through the relay must reach the harness server, and
cargo must resolve the workspace offline inside. Each fence was also
broken in the sandbox's arguments and the probe seen to fail (slice 1's
commit).

## Slices

1. **Sandbox and relay**, in three PRs after #467's review: (a) the
   bwrap arguments, the builder's paths, the host lookups and the
   inherited-descriptor check; (b) the clone, `xtask api-relay-inner`
   (the relay and the session's parent) and the harness server, with its
   caps; (c) the fences and `cargo xtask api-fences <name>`, which proves
   them.
2. **Proxy, key and budget**: the harness server's API route, the price
   table in `.config/factory.toml` (a model with no price is refused),
   `cargo xtask api-build` running the session; the key probe against the
   live key file.
3. **`ask_lead`**: the MCP server inside; `q<N>.md` written and its path
   logged, `a<N>.md` read, 30 minutes, then `BLOCKED.md` and stop.
4. **`request_push`**: `cargo xtask ci` in a fresh sandbox, the path and
   400-line checks, the push from outside with the hook skipped (declared
   at the call), `gh pr create`; a branch whose `build.rs` writes outside
   the clone fails inside.
