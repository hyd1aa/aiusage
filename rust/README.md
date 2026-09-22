# AIUsage Rust runtime

AIUsage is a Rust-only project. This directory contains the application,
native regression and golden tests, and the pinned Rust 1.90.0 toolchain
definition.

## Build and test

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --locked --example check_sensitive
cargo build --locked --release --bin aiusage
./target/release/aiusage --version
./target/release/aiusage --demo --snapshot --size 80x24
```

The test suite is offline and uses only synthetic provider processes and
frozen compatibility fixtures. Demo mode never discovers providers, calls
adapters, or reads authentication material.

## Distribution

The repository-root `install.sh` atomically installs a verified native binary
and preserves user configuration. The updater accepts only official release
assets named `aiusage-vVERSION-linux-amd64`,
`aiusage-vVERSION-linux-arm64`, or `aiusage-vVERSION-macos-arm64`, with the
GitHub-provided SHA-256 digest. It validates origin, digest, and embedded
version before installation.

`Cargo.toml` package metadata is the single version source. The executable,
diagnostics, updater, tests, CI packaging, and release artifact validation all
use that Cargo version.

Native CI covers Linux amd64, Linux arm64, and macOS arm64. Linux packages are
stripped musl binaries. macOS packages link only system libraries.

Historical migration audits remain under `docs/`; they describe the v0.2.2 to
v0.3.0 transition and are not current build or runtime instructions.
