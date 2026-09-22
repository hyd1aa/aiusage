# Changelog

All notable changes to this project are documented here. This project follows
the structure of [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

- Added a bounded Provider discovery contract with installation, readiness,
  usage-support, usability, and reason states.
- Added startup discovery, five-minute periodic discovery, `R` discovery, and
  persistent automatic discovery controls.
- Added VPS-oriented discovery candidates for MiniMax, Qoder, Qoder CN,
  CodeBuddy, and TraeCode.
- Added a ZCode UI candidate explicitly marked ineligible until an official
  headless CLI exists.

### Changed

- Explicitly disabled Providers remain hidden across future discovery passes,
  while removed CLIs and expired sessions retain their configured order.
- Diagnostics now report sanitized discovery states for every Provider.
- Removed Gemini and Antigravity, with safe cleanup of stale keys in existing
  enabled, demo, disabled, and ordering configuration.

## [0.3.1] - 2026-09-22

Rust-only maintenance release. This is not a new feature release. v0.3.0
installs upgrade directly, and user configuration stays compatible.

### Removed

- Retired the legacy Python implementation, launchers, packaging, rollback
  runtime, and Python CI after the Rust v0.3.0 production soak.
- Replaced runtime differential tests with frozen golden fixtures and native
  Rust PTY, regression, installer, updater, and sensitive-data tests.

### Changed

- Cargo package metadata is now the sole version source for the runtime,
  diagnostics, updater, installer validation, CI, and release builds.
- The repository, CI, and packaging are Rust-only. Supported release
  platforms remain Linux amd64 (stripped musl), Linux arm64 (stripped musl),
  and macOS arm64 (self-contained, linking OS libraries).
- The repository-root installer installs only a verified Rust binary.
  Ownership checks and the atomic publish path were hardened.
- PTY tests now use a portable terminal size so the same cleanup checks run
  on macOS and Linux.
- Frozen golden fixtures keep the same behavior checks. Only the embedded
  version label moved from 0.3.0 to 0.3.1.

### Compatibility

- Frozen golden fixtures remain the compatibility protection for CLI, menu,
  CODEX, GROK, timestamp, timezone, and Dashboard behavior.
- User configuration from 0.2.2 and 0.3.0 is unchanged by this release.

## [0.3.0] - 2026-09-15

### Changed

- Replaced the published runtime with a behavior-compatible Rust
  implementation. User configuration, CODEX and GROK readers, Dashboard,
  management menu, timezone handling, discovery, and refresh cadence remain
  compatible with the Python 0.2.2 reference.
- Official updates now install platform binaries
  (`aiusage-vVERSION-linux-amd64`, `aiusage-vVERSION-linux-arm64`,
  `aiusage-vVERSION-macos-arm64`) from GitHub Release assets with origin,
  SHA-256, and `--version` verification. Linux artifacts are stripped musl
  static binaries. macOS artifacts are self-contained application binaries
  that link the OS-provided libSystem, libiconv, and CoreFoundation.
- Python 3.10 remains the canonical behavior oracle. The Python source stays
  in the repository as the differential reference and as the rollback
  implementation. Replacing a running Python installation is a separate,
  explicitly authorized step.

### Validation

- Python 3.10 CLI, timestamp, menu, provider, updater, and Dashboard
  differential and golden tests.
- Isolated Python → Rust upgrade and Rust → Python rollback rehearsal.
- Native CI on Linux amd64, Linux arm64, and macOS arm64.

### Compatibility notes

- Arbitrary legacy-codepage transcoding is out of scope.
- Rust integers are bounded; Python bigints are not.
- Invalid Codex reset objects are not rendered as arbitrary Python objects.

## [0.2.2] - 2026-09-06

### Fixed

- Fixed Grok weekly quota rollover when a new billing period starts
  at 0% usage.
- Grok's proto3 JSON may omit `creditUsagePercent` when its value is
  zero; AIUsage now correctly interprets the omitted default as 0%
  used instead of skipping the latest billing snapshot.
- New weekly reset timestamps now replace expired previous-period
  timestamps correctly.
- Manual refresh and 30-second refresh now display the latest parsed
  Grok billing period when present.

### Validation

- 0% used → 100% remaining
- 100% used → 0% remaining
- 47% used → 53% remaining
- Weekly rollover Sep 05 → Sep 12
- UTC+08 reset conversion verified
- Stale retention behavior preserved
- 131 tests passing
- Python 3.10–3.13 CI passing

## [0.2.1] - 2026-09-03

### Fixed

- Installer no longer changes permissions of existing shared directories such
  as `/usr/local/bin` and `/usr/local/lib`.
- Existing directory mode, owner, group, and setgid bits are preserved.
- Installer now safely fails if an expected directory path is actually a
  regular file.
- Uninstaller now verifies AIUsage ownership before removing package and
  uninstaller paths.
- Existing third-party `ai` commands remain protected during install, update,
  and uninstall.

### Tests

- Added installation lifecycle permission regression tests.
- Full suite: 111 tests.
- Python 3.10–3.13 CI.

## [0.2.0] - 2026-09-03

### Added

- Beginner-friendly `ai` management menu.
- Stable `aiusage --menu` management entry.
- Settings management, safe update checker/updater, read-only diagnostics, and
  interactive uninstall.
- Configurable display timezone with the `Z` selector.
- `UTC±HH` and `UTC±HH:MM` offsets and system timezone follow mode.

### Improved

- Reset timestamps convert to the selected display timezone with correct date
  rollover, compact UTC offset labels, and DST-aware system conversion.
- Installer onboarding and Chinese/English documentation.

### Safety

- Existing third-party `ai` commands are never overwritten.
- Updates only use the official `hyd1aa/aiusage` repository.
- Diagnostics never expose credentials.
- User configuration is preserved by default during update and uninstall.

## [0.1.2] - 2026-09-03

### Added

- Configurable display timezone.
- `Z` shortcut for timezone selection.
- `UTC±HH` and `UTC±HH:MM` offset support.
- System timezone auto-follow mode.

### Changed

- Reset timestamps now convert across dates correctly.
- Compact UTC offset labels replace ambiguous CST/EST/EDT and IANA names.
- System timezone conversion is DST-aware.
- Expanded Chinese and English timezone documentation.

### Tests

- Expanded regression coverage to 53 tests.
- Verified the Python 3.10–3.13 CI matrix.

## [0.1.1] - 2026-09-03

### Added

- Added persistent White and Green foreground themes with the `T` shortcut.
- Added explicit timezone labels to reset timestamps and system time.
- Added localized Chinese month and date formatting.

### Changed

- Simplified Chinese is now the default language for new users while saved
  language preferences remain unchanged.
- Restored a compact single-box layout with a centered title and natural,
  content-driven sizing.
- Centered compact 2×2 and 3×2 provider grids without stretching them to fill
  the terminal.
- Expanded regression coverage to 42 tests, including PTY cleanup, timezone,
  theme, responsive layout, and Chinese display-width checks.

### Fixed

- Stopped themes from overriding the terminal background; themes now affect
  foreground text, borders, and progress bars only.
- Improved Chinese wide-character measurement and compact spacing.

## [0.1.0] - 2026-09-02

### Added

- Responsive terminal dashboard with one-, two-, and three-column layouts.
- Verified Codex and Grok rate-limit readers.
- Deterministic, isolated demo mode.
- Extensible provider registry and common usage model.
- English and Chinese interface text.
- Provider selection, ordering, and local configuration.
- Seven whole-dashboard positions.
- Resize handling, low-flicker redraws, and terminal cleanup.
