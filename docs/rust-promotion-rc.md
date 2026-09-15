# Rust production promotion candidate

Checkpoint date: 2026-09-15

This is a release-candidate qualification record, not a production switch or
release.  The Python implementation remains the production implementation.
No tag, GitHub Release, `main` merge, production executable replacement, or
system service change is part of this checkpoint.

## Source and branch

- Reference `main` and `origin/main`:
  `f50ecabd4873b45446ff55d7c36eae694f02f272`.
- Qualified migration checkpoint:
  `578cf9bf37c9031301457f62dce41e412c2877d3`.
- Promotion branch: `release/rust-v0.3.0-rc`.
- Python source, Python tests, original install/uninstall scripts, launchers,
  and Python CI are byte-for-byte unchanged relative to `main`.

## Clean build gate

A fresh clone of the promotion branch, with a new local `target` directory,
passed:

- `cargo fmt --check`;
- `cargo clippy --locked --all-targets -- -D warnings`;
- 58/58 Rust test functions (including Python 3.10 differential, PTY,
  provider, installation, update and rollback tests);
- 131/131 unchanged Python regression tests;
- the reachable-Git-history sensitive-pattern scan.

Native CI independently builds and executes the candidate on Linux amd64,
Linux arm64, and macOS arm64.  Linux outputs are stripped musl binaries with
no `NEEDED` entries.  The macOS output is a self-contained application binary
which intentionally links the OS-provided libSystem, libiconv and
CoreFoundation runtime.  macOS is not claimed to be statically linked.

Promotion CI run `34977342741` passed all three native jobs at the qualified
checkpoint.  Its review-artifact SHA-256 checksums are:

- Linux amd64: `84ff43a48e6b3347ed8f64d94707823a79d52b989bca72204214d495576d3075`;
- Linux arm64: `d79186c4d50f217f6b65378823e30d06562812c8663c9162cae1aa55e01a6ad7`;
- macOS arm64: `373cf9cbfe50d1130229ecbb90cee80d913b0b301d65267148e7e303b2fffd60`.

These are CI review artifacts, not published release assets.  A second clean
VPS clone independently produced a stripped/static Linux arm64 binary with
SHA-256
`4831a64c0098a47fc7d0a7cee8a0b1fba703f7ed1d2dfcbb14ead83a968ea4dd`.

The candidate still reports `AIUsage 0.2.2`.  This preserves the existing
single version authority during qualification.  A production Rust replacement
is a structural runtime and distribution change, so the recommended release
version is **0.3.0**.  A coordinated version-source/Cargo metadata update and
post-update rebuild belong to the separately authorized release-preparation
step; this checkpoint does not create a tag or release.

## Isolated VPS candidate

The Linux arm64 candidate is installed at `/opt/aiusage-rc`, separate from
`/usr/local`.  Tests use an isolated `XDG_CONFIG_HOME` and, where appropriate,
a byte-for-byte copy of the current configuration.  The production launcher
and production configuration checksums remained unchanged.

Smoke evidence covers:

- version, help, invalid arguments, exit codes and stdout/stderr separation;
- non-interactive TTY failure;
- 80x24 and narrow PTYs, Chinese and English, white and green themes, Unicode
  rendering, manager/menu entry and terminal restoration;
- read-only real CODEX and GROK discovery/usage reads, and safe unavailable and
  malformed-provider fixtures;
- `system`, `UTC`, `UTC+08`, `UTC-08`, and `UTC+05:30` display timezones;
- normal and missing `HOME`, normal and minimal `PATH`;
- diagnostics for the Rust runtime, terminal, config, providers, timezones and
  GitHub, with secret-marker checks and no production config write;
- serialized refresh/cancellation through an owned fake provider process.

The 300-second discovery and 30-second refresh deadlines use monotonic cadence
tests; a 36-second PTY soak crossed a refresh interval and included 12 manual
refreshes.  RSS stayed bounded at approximately 1.3 MiB, threads stayed at one
to two, the process exited normally and left no zombie.  Twelve repeated
manager-to-Dashboard-to-manager cycles also exited normally.  PTY tests cover
Q, Escape, Ctrl-C and SIGTERM cleanup.

## Upgrade and rollback drill

An isolated temporary prefix completed this sequence:

1. install the unchanged Python implementation;
2. verify Python CLI and an isolated existing configuration;
3. install the Rust binary over the AIUsage-owned temporary installation;
4. verify the Rust CLI reads the same configuration;
5. install the Python implementation over the Rust-owned temporary
   installation;
6. verify Python CLI and configuration integrity again.

Both transitions were accepted by the ownership checks.  The configuration
checksum was unchanged.  No user state had to be recreated.  Separate updater
and installer lifecycle tests cover exact platform asset selection, trusted
GitHub origin, required SHA-256 digest, pre-install version verification,
atomic owned-path publication, failed-publication restoration, config
retention, shared-directory metadata and third-party `ai` preservation.

Rollback procedure for a future production promotion is: retain the matching
Python source release locally, stop invoking the Rust executable, run that
release's unchanged `install.sh` against the existing AIUsage-owned prefix,
verify `aiusage --version` and diagnostics, then resume normal use.  The drill
shows the Rust ownership markers and symlink launcher are accepted directly;
uninstalling first is not required and configuration is external to the
program prefix.

## Qualification verdict

**READY FOR PROMOTION**

Blocking issues: zero.  Known non-blocking compatibility limits remain those
recorded in `docs/rust-migration.md`: Python 3.10 is the behavioral oracle,
arbitrary legacy-codepage transcoding is outside scope, Rust numeric inputs are
bounded, and invalid Codex reset objects are not rendered as arbitrary Python
objects.

This verdict means the implementation is suitable for a separately approved
merge and release-preparation step.  It does not authorize or claim that the
current Python production installation has been replaced.  A formal 0.3.0
release still requires the coordinated version update, a clean rebuild with
new checksums, final CI, tag and GitHub Release under explicit authorization.
