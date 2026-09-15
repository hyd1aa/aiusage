# Rust behavioral migration

Reference: Python 0.2.2, commit `f50ecabd4873b45446ff55d7c36eae694f02f272`.
The existing Python source, tests, launchers and installer remain unchanged.
Baseline: 131 Python unittest tests passed before implementation.

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
- Next action: native CI/build verification, terminal edge cases and complete
  parity review. No production install.
- Outstanding parity review: arbitrary Unicode inputs, full argparse edge
  cases, cancellation during slow provider I/O, and concurrent manual/worker
  refresh ordering. Existing tests cover module behavior, not full equivalence.
- No release, tag, production installation, or Python removal is authorized
  by this checkpoint. Python remains the current production implementation.
