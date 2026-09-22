# Contributing

AIUsage is a Rust-only project and uses the pinned Rust 1.90.0 toolchain:

```sh
cd rust
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --locked --example check_sensitive
```

Provider adapters return a `ProviderUsage` containing an availability state
and zero or more `RateLimitWindow` values. A real adapter must use a reliable,
verifiable data source. It must never infer or fabricate usage, extract stored
credentials, or log authentication material. UI work may use deterministic
fixtures through demo mode.

Keep changes focused, readable, and compatible with Rust 1.90. Pull requests
should include relevant tests, pass the offline test suite, avoid credentials
and machine-specific data, and update documentation when behavior changes.
