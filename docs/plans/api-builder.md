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
- **The sandbox**: bwrap with `--unshare-net`, `--unshare-pid`,
  `--unshare-ipc`; the home directory a tmpfs; writable only the clone,
  its target directory, its cargo home and the shared sccache directory;
  the toolchain, `~/.cargo`'s registry, the CLI and the main repository's
  objects (the clone's alternates) read-only; `/dev/dri` bound for the GPU
  tier. Cargo runs `--offline`.
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

Live, slice 2 (2026-11-17, cents of credit): a one-word brief through the
proxy ended `done` with the proxy's count equal to the CLI's ($0.0041,
then $0.0005 with the prompt cached). The check that holds the two counts
together failed on the second run with the skill's cache-read price for
Sonnet 5.5 ($0.20); the CLI prices it at $0.10, which
`[api_build.prices]` now holds. With the CLI's budget raised, a proxy
budget of $0.001 refused the session's second request.

## Fences, proven on every run

Before a session starts, `api-build` runs each probe inside the sandbox
and refuses to start if any escape succeeds. Each probe's action is run
outside first as its control, so a probe that cannot detect its escape
fails the run instead of passing it: write outside the clone (through
the hidden home and into a read-only mount), reach the network directly,
read the key, see a process outside, and push. The way in is probed
too: a request through the relay must reach the harness server, and
cargo must resolve the workspace offline inside. Each fence was also
broken in the sandbox's arguments and the probe seen to fail (slice 1's
commit).

## Slices

1. **Sandbox and relay** (#467): the bwrap arguments, `xtask
   api-relay-inner` (the relay and the session's parent), the builder's clone,
   and `cargo xtask api-fences <name>`, which proves the fences above.
2. **Proxy, key and budget** (#469): the harness server's API route, the price
   table in `.config/factory.toml` (a model with no price is refused),
   `cargo xtask api-build` running the session; the key probe against the
   live key file.
3. **`ask_lead`** (built; live 2026-11-17: the session asked "What
   colour?", the answer file said `blue`, and the session replied
   `blue`): the MCP server inside; `q<N>.md` written and its path
   logged, `a<N>.md` read, 30 minutes, then `BLOCKED.md` and stop.
4. **`request_push`**: `cargo xtask ci` in a fresh sandbox, the path and
   400-line checks, the push from outside with the hook skipped (declared
   at the call), `gh pr create`; a branch whose `build.rs` writes outside
   the clone fails inside.
