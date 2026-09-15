# AIUsage Rust implementation

This directory is the v0.3.0 published runtime. The Python source remains the
Python 3.10 behavior oracle and the rollback implementation. Do not replace a
running Python production prefix unless that cutover has been explicitly
authorized.

## Build and test

From this directory, with Rust 1.90.0 and Python 3.10+ available:

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release --bin aiusage
./target/release/aiusage --version
./target/release/aiusage --demo --snapshot --size 80x24
```

Python is needed by differential/PTY tests only. The compiled application does
not execute Python. Avoid changing preferences during experiments against your
normal config: set `XDG_CONFIG_HOME` to a disposable directory for interactive
tests. Demo never discovers providers or reads quota/authentication data.

## Implemented and tested so far

- Configuration load/save, legacy selections, atomic private writes.
- Typed quota/availability, ties-to-even rounding and stale retention.
- Fixed/system timezone formatting, DST, zh/en, white/green foreground styles.
- Pure Dashboard and selectors, 1,008 Dashboard and 96 selector oracle cases.
- Codex JSON-RPC and Grok billing-log adapters with synthetic inputs only.
- Discovery/readiness, explicit opt-outs, 300s discovery and 30s refresh.
- Dashboard keys, snapshot/version/help, non-TTY errors, terminal cleanup.

## Distribution

Manager/settings, diagnostics, explicit update/uninstall and an isolated Rust
installer are implemented. They do not invoke Python. Native Linux amd64,
Linux arm64 and macOS arm64 CI builds the published binaries. The Python
manager remains in the repository for differential tests and rollback.

The Rust updater expects an official release asset named
`aiusage-vVERSION-linux-amd64`, `aiusage-vVERSION-linux-arm64` or
`aiusage-vVERSION-macos-arm64`, with a SHA-256 digest in the GitHub release
metadata. It validates origin, digest and binary version before installation.
A Python-only release is not silently installed over a Rust binary. This is an
intentional distribution-format change, not a quota/config/UI change.

The `install.sh` in this directory is separate from the Python installer.
Do not install into a production prefix unless that cutover is authorized.

The canonical Python 3.10 parity review now covers malformed CLI/ISO timestamp
inputs, UTF-8 zh/en menu bytes, Unicode decimal timezones and live manager
width. Slow provider cancellation and serialized manual refreshes use owned
synthetic processes. Legacy non-UTF-8 transcoding and unbounded Python integers
remain documented limits rather than release blockers.

Python 3.10 is the canonical differential oracle: it is the minimum supported
version and the audited local reference runtime. Python 3.11+ differs in
argparse action ordering and accepts a broader ISO grammar, so native Rust CI
pins 3.10 while the unchanged Python suite still runs on 3.10–3.13. The ISO
gate is active, not ignored. UTF-8 menu bytes, CLI error ordering, Unicode
dimension inputs and timestamp boundaries have differential tests. Publishing
this runtime does not itself replace a running Python production installation.

See [the migration audit and phase gates](../docs/rust-migration.md).
