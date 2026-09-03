# Changelog

All notable changes to this project are documented here. This project follows
the structure of [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

- Added a persistent `timezone` preference with live system-zone following,
  fixed UTC offsets, quarter-hour custom offsets, and the `Z` selector.

### Changed

- Reset and system timestamps now use one selected display timezone and show
  unambiguous numeric UTC offsets instead of timezone abbreviations.

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
