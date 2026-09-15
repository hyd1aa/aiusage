# Rust behavioral migration

Reference: Python 0.2.2, commit `f50ecabd4873b45446ff55d7c36eae694f02f272`.
The existing Python source, tests, launchers and installer remain unchanged.
Baseline: 131 Python unittest tests passed before implementation.

## Current checkpoint — restored 2026-09-15

Read-only recovery found local HEAD and the actual remote migration branch at
`31d2aeb0be7986ca0ef4ebb7f883e45a6f588402`, clean, with no later commits.
Local/remote main remained `f50ecabd4873b45446ff55d7c36eae694f02f272`.
Evidence: this document, `rust/README.md`, seven migration commits, `git
ls-remote`, Python CI `34938151157` and native Rust CI `34938151150` (all green).
This document is the existing checkpoint; do not restart the migration.

| Gate | Recovered evidence / follow-up |
| --- | --- |
| Config / HOME / discovery / refresh | Implemented; legacy config, private writes, HOME fallback, opt-outs, 30s/300s cadence, cancelled owned processes and serialized real-mode fixture reads tested. |
| Dashboard / timezone / theme / language | 1,008 frozen frames, selectors, system DST/timezones; no redesign. |
| CLI streams / exit / malformed arguments | New 702-case parser differential matrix plus process stdout/stderr/exit checks; fixed pre-action ambiguity detection, short help clusters, option-like values, Unicode and underscore integers. |
| Timestamp common/boundary formats | New isolated-TZ oracle matrix; fixed hour/minute precision, leap-second rejection, microsecond carry and Python year 1..9999 limits before/after conversion. |
| Menu / diagnostics | UTF-8 bytes compared with real Python TextIOWrapper in zh/en at 10/20/39/40/80 columns; live width already tested. Diagnostics tested with empty synthetic PATH/HOME and a redaction canary. Rust identifies its runtime honestly. |
| Providers / updater | Existing Grok fixtures retained; 54 new Codex reply cases use a fake Python Popen/select, never a real account. Version tuple comparison now preserves signed/Unicode integers. Verified binary assets intentionally replace Python source archives. |
| ISO parser version-specific behavior | **OPEN**: Python 3.10 rejects nine fractional digits; Python 3.11+ accepts/truncates them and adds basic/week date syntax. One explicit ignored gate retains these cases. It was run manually and failed on Python 3.10 (Rust accepts nine digits). Not a passing gate. |
| Native release artifacts | Previous head verified on all three native runners; follow-up CI required for this patch. No install, main merge, tag or release. |

Current local result: **57 passed, 1 explicitly ignored/open gate**; clippy
and fmt pass. The open test is `version_sensitive_iso_acceptance_gate` and is
run with `cargo test --test compatibility version_sensitive_iso_acceptance_gate
-- --ignored --nocapture`. Standard CI green must never be described as full
parity while it remains open.

Next action: resolve the reference-version choice (current environment Python
3.10 recommended; Python 3.12 alternative), implement that ISO acceptance
contract, remove the ignore, add matching version-specific native CI coverage,
and rerun the gate. Never silently drop the divergent fixtures.

Known scope notes: Rust emits UTF-8 independently of Python-only encoding
environment flags; arbitrary legacy-codepage transcoding is not validated.
Rust integers are bounded (unlike Python bigint). Invalid Codex reset types
are not rendered as arbitrary Python objects. These must remain explicit
compatibility limitations, not claims of universal input equivalence.

Safety: Python reference diff remains empty; all provider data are fixtures;
installation/update tests use temporary prefixes. Production entrypoints,
credentials, services and Airport Monitor are untouched. No parity completion
or promotion is claimed.

## Audited contracts

| Area | Reference and observable behavior |
| --- | --- |
| CLI | `cli.py`: `--demo`, `--menu`, `--snapshot`, `--size WIDTHxHEIGHT`, `--version`, argparse help/errors; non-TTY exit 2; menu wins over snapshot. |
| Config | `config.py`: XDG path; permissive line parser (NOT standard TOML), last key wins, comments stripped even inside quotes, invalid values fall back, legacy disabled-provider migration, ordered deduplication, atomic 0600 writes. |
| Models | Four availability states, zero or more windows, stale/error separated from reliable quota. |
| Registry | Ordered codex/grok/minimax/qoder/qodercn/codebuddy/traecode/zcode. Only Codex and Grok have real readers; executable aliases and readiness are distinct from installation. |
| Codex | Local `codex app-server --stdio`; initialize/initialized/rateLimits JSON-RPC; one 8s deadline; primary then secondary; terminate/reap child; never read stored credentials. |
| Grok | Bounded 4 MiB JSONL tail; last valid billing record in file order; currentPeriod with billing fallback; omitted proto3 usage is legitimately zero only with reliable reset; malformed usage is rejected. |
| Numeric | Python ties-to-even rounding BEFORE clamping usage; duration labels Daily/Week/hours/Cycle; no inferred reset. |
| Discovery | Startup before refresh, 300s cadence independent of 30s refresh; bounded parallel 2s discovery; explicit disables respected; previous discovery retained on transient failure; notice 5s. |
| Refresh | Only enabled readers; failed read retains old windows with stale flag; success replaces/clears stale; demo never calls discovery/readiness/auth/real readers. |
| Render | One natural-size box, 1/2/3-column layout, six demo providers at 80x24, seven positions, Unicode cell width, foreground-only white/green, language-specific labels and reset dates. |
| Time | UTC and UTC±HH[:MM], -12..+14, minute precision; system DST at the actual epoch; numeric offset labels; custom selector 15-minute increments. |
| Input | T/L/P/S/Z/R/Q, arrows/J/K, space, U/D, Enter/Escape; config shared with manager; cbreak, resize, differential painting; terminal restoration on all exits. |
| Manager | `ai` and `aiusage --menu`; six actions; return from dashboard; explicit update/uninstall confirmation; zh/en and settings persistence. |
| Diagnostics | Sanitized installation/readiness/usage checks, terminal encoding and timezones; no credentials. Rust runtime row must honestly identify Rust, not pretend Python is required. |
| Update | Official stable GitHub release, 2s check/6h cache, confirmation, safe archive extraction, tag/source version check, install then verify version. Binary-release integration requires its own safety tests before replacing the Python installer. |
| Installation | Preserve shared directory metadata and third-party ai; AIUsage ownership checks; config retained; original scripts remain reference-only. |

## Stages and release gates

1. Audit + models/config/timezone/version primitives; Rust unit and Python differential tests.
2. Pure rendering/i18n/demo; frozen-clock golden matrix including 80x24, small terminals, themes, positions and DST.
3. CLI, key state machine, refresh/discovery; fake clock/readers and PTY cleanup tests.
4. Real adapters; fixture/fake-process protocol tests, malformed data, timeout and stale retention.
5. Manager/diagnostics/updater; scripted input, local HTTP/archive fixtures, no live install or credentials.
6. Native Linux amd64/arm64 and macOS arm64 CI, Linux musl builds, install/update/uninstall lifecycle, parity review.

Do not replace production entrypoints, remove Python, publish a release, or
claim functional parity until all stages pass. A cross-compiled artifact is
not evidence that native terminal and timezone behavior passed on that OS.
Linux musl aims for static linkage; macOS uses system libSystem (self-contained
application binary, not fully static). User-installed provider CLIs and system
timezone data remain intentional external resources.

## Compatibility tests

Python is the oracle, invoked only by the test harness with synthetic data and
isolated config/cache/home. The Rust runtime must not invoke Python. Preserve
reference quirks deliberately; document security-sensitive differences rather
than reproducing unsafe behavior silently. Keep independent Rust assertions
alongside differential tests so shared fixture mistakes cannot prove parity.

## Progress

- Audit of all current source modules, eight test modules, installer/uninstaller,
  launchers and CI completed; 131/131 baseline tests passed.
- Stage 1 passed: 8 Rust unit tests and 3 differential suites (config, all
  quarter-hour offsets and date boundaries, half-integer rounding). Rust 1.90.0
  runs on native Linux arm64. Python reference remains unchanged.
- Stage 2 passed: 1,008 frozen-clock Dashboard comparisons plus 96 selector
  comparisons (80x24 included). Layout, labels and foreground styles match.
- Adapter and Dashboard stages: Codex fake-process handshake, malformed data,
  bounded timeout/child cleanup; Grok rollover/4 MiB tail/differential fixtures;
  opt-outs/discovery retention, stale refresh, pure key effects and monotonic
  cadence implemented. No live provider data or credentials used.
- CLI: snapshot/version/help and common errors compare against Python; actual
  PTY tests verify Q/Escape/Ctrl-C/SIGTERM and consecutive input with termios
  restoration. Fixed a buffering/poll mismatch exposed by consecutive keys.
- Foundation checkpoint tests: 28 Rust test functions, including 7 differential suites and
  5 PTY scenarios. System timezone checked in isolated processes for UTC,
  Shanghai, New York (summer/winter), Kathmandu and Adelaide.
- Native Linux arm64 release build passes and prints `AIUsage 0.2.2` without
  Python. Initial GNU build links libc/libgcc; static-musl packaging is being
  checked separately and is not a substitute for completing the manager.
- Manager/diagnostics/updater implemented: six menu actions, 96 main-screen
  differential cases, scripted settings/cancel/order/confirmation tests.
  Diagnostics reports Rust rather than a fabricated Python requirement.
- Separate Rust installer supports binary + `ai` symlink entry; temporary-PREFIX
  lifecycle tests cover shared metadata, third-party commands, reserved paths,
  verified-binary update, digest failure and no-op update.
- Distribution change: Rust updates require an official target-specific binary
  asset with [GitHub SHA-256 digest metadata](https://docs.github.com/en/rest/releases/assets).
  No archive extraction or Python fallback; stable release, confirmation,
  trusted source, version verification and config retention remain mandatory.
  Version cache remains readable by the Python reference. HTTPS/repository
  validation is intentionally stricter than the old substring check.
- Manager/update checkpoint: 45 Rust test functions pass, including 8
  differential suites. Failed publication restores the previous owned install;
  digest/version mismatch fails before installation. Installer scripts are
  tested only against temporary prefixes.
- Added independent native CI for Linux amd64, Linux arm64 and macOS arm64,
  including musl release builds on Linux and PTY tests of release artifacts.
  CI uploads review artifacts only, never tags or GitHub Releases.
- First native macOS arm64 CI passed tests, build and release PTY checks.
  Linux CI invocation initially used the Rust working directory for reference
  tests; corrected it to repository root without modifying Python tests.
- Linux arm64 musl binary built locally with no dynamic section and verified
  `--version`. Current local suite: 46 Rust tests; slow Codex reads can be
  cancelled and reaped, and manual/periodic reads share one worker to prevent
  response reordering. Manual refresh preserves periodic deadlines.
- Native matrix at `4d3b3d0` passed on Linux amd64, Linux arm64 and macOS arm64
  (GitHub Actions Rust parity run `34917428145`); Python CI also passed.
- Follow-up: argparse terminator/negative-argument compatibility, passwd home
  fallback when HOME is unset, executable access checks, sanitized locale
  diagnostics. A real PTY with an isolated fake Codex proves refresh
  serialization. The PTY harness now synchronizes original termios capture
  before child startup, avoiding a test-only race. Full Rust suite: 48 tests.
- Follow-up native matrix `34918015234` passed all three platforms at
  `aa694ad`; Python CI `34918015206` passed too. Its arm64 musl binary passed
  both PTY harnesses locally and has no dynamic section.
- Final review follow-up adds Unicode decimal timezone digits (Unicode 15
  blocks, tested against fullwidth/Arabic/mathematical input) and live manager
  width checks. The Python reference truncates without ellipsis; the resize
  test asserts that exact behavior. Full Rust suite: 49 test functions.
- Next action: verify this review follow-up in native CI, then audit remaining
  malformed CLI/ISO timestamp inputs and manager output-encoding behavior.
  No production install or promotion yet. Existing tests establish the covered
  contracts, not equivalence for every possible input. Provider cancellation,
  serialized refresh and live menu width now have explicit tests.
- No release, tag, production installation, or Python removal is authorized
  by this checkpoint. Python remains the current production implementation.
