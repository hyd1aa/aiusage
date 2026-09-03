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
