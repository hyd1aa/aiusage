# AIUsage Rust migration (experimental)

This directory is an incremental implementation, **not yet a replacement** for
the Python release. Do not install it over existing `aiusage`/`ai` commands.
The reference source and its installer are unchanged.

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

## Not complete

Manager/settings, diagnostics, explicit update/uninstall and an isolated Rust
installer are now implemented. They do not invoke Python. The existing Python
manager remains fully usable and installed entrypoints have not been changed.
Native validation on all three release platforms and the full parity review
remain pending. There is no published Rust release.

The Rust updater expects an official release asset named
`aiusage-vVERSION-linux-amd64`, `aiusage-vVERSION-linux-arm64` or
`aiusage-vVERSION-macos-arm64`, with a SHA-256 digest in the GitHub release
metadata. It validates origin, digest and binary version before installation.
A Python-only release is not silently installed over a Rust binary. This is an
intentional distribution-format change, not a quota/config/UI change.

The new `install.sh` in this directory is separate from the Python installer.
Do not install into a production prefix before release gates have passed.

Known parity-review work: broader malformed CLI/Unicode inputs, cancellation
during slow provider I/O, concurrent manual/background refresh ordering and
exact manager behavior. Passing the current tests is not full parity evidence.

See [the migration audit and phase gates](../docs/rust-migration.md).
