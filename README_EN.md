# AIUsage

[简体中文](README.md) | **English**

A lightweight, responsive terminal dashboard for AI CLI usage and rate limits.

View remaining percentages, usage windows, and reset times from AI CLI clients such as Codex and Grok in one SSH, VPS, or Linux terminal dashboard.

[Introduction](#introduction) · [Preview](#preview) · [Installation](#installation) · [Usage](#usage) · [Provider support](#provider-support) · [Shortcuts](#keyboard-shortcuts) · [Configuration](#configuration) · [License](#license)

## Introduction

AIUsage runs directly in your terminal. It needs no web panel and no background daemon: launch `aiusage` and it adapts to SSH sessions, tmux windows, and VPS panes.

- Verified real usage readers for Codex and Grok
- Live switching between Chinese and English
- White theme by default, with a green theme available through `T`
- Provider selection and ordering
- Responsive one-, two-, and three-column layouts
- Live system clock and 30-second usage refresh
- Unicode progress bars and low-flicker partial redraws
- No telemetry and no usage/configuration uploads

## Preview

### Real mode

Run `aiusage`. Real mode displays only providers with reliable local data sources. The following illustrates the layout; percentages and reset times come from your own installed clients.

```text
┌─────────────────── AI USAGE ───────────────────┐
│                                                │
│    CODEX                                       │
│    5h     ████████░░  83% left                 │
│    Reset: Sep 03 14:46 CST                     │
│                                                │
│    GROK                                        │
│    Weekly ███████░░░  72% left                 │
│    Reset: Sep 05 23:14 CST                     │
│                                                │
│ System: 2026-09-02 12:34:56 CST                │
└────────────────────────────────────────────────┘
```

### Demo mode

`aiusage --demo` uses deterministic local fixtures to preview six providers in a 3×2 layout. The dashboard is prominently marked **`[DEMO]`**. Claude, Gemini, DeepSeek, Kimi, and other demo values are not real usage.

```text
┌─────────────────────────────── AI USAGE [DEMO] ───────────────────────────────┐
│ CODEX                     GROK                      DEEPSEEK                  │
│ 5h     ████████░░  83%    Cycle  ███████░░░  72%    Daily  ███████░░░  66%    │
│                                                                               │
│ CLAUDE                    GEMINI                    KIMI                      │
│ 5h     █████░░░░░  48%    Daily  █████████░  91%    Monthl ████░░░░░░  37%    │
└───────────────────────────────────────────────────────────────────────────────┘
```

The complete 80×24 text capture is available at [`docs/screenshots/demo-80x24.txt`](docs/screenshots/demo-80x24.txt). A future PNG can be placed at `docs/screenshots/aiusage-demo.png`; captures must use demo mode and exclude hostnames, IP addresses, shell prompts, and unrelated panes.

## Installation

### Linux

```bash
git clone https://github.com/hyd1aa/aiusage.git
cd aiusage
sudo ./install.sh
aiusage
```

The idempotent installer adds `/usr/local/bin/aiusage` and the runtime package under `/usr/local/lib/aiusage`. It does not overwrite existing user configuration.

Uninstall while preserving preferences:

```bash
sudo ./uninstall.sh
```

Remove `~/.config/aiusage/` explicitly afterward only if you also want to delete saved preferences.

## Usage

Real mode:

```bash
aiusage
```

Demo mode:

```bash
aiusage --demo
```

Other commands:

```bash
aiusage --help
aiusage --version
aiusage --demo --snapshot --size 80x24
```

New users without a configuration file start with the Chinese UI. Press `L` to switch to English and save that preference. Existing `language = "en"` settings are preserved during upgrades.

## Provider support

| Provider | Real usage | Demo/UI | Status |
| --- | --- | --- | --- |
| Codex | Yes | Yes | Supported |
| Grok | Yes | Yes | Supported |
| Claude | No | Yes | UI ready |
| Gemini | No | Yes | UI ready |
| DeepSeek | No | Yes | UI ready |
| Kimi | No | Yes | UI ready |
| GLM | No | Yes | UI ready |
| z.ai | No | Yes | UI ready |

Real mode never substitutes demo values. An enabled provider without a verified reader is reported honestly as `Not installed`, `Unavailable`, or `Not supported`.

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `T` | Switch between white and green themes |
| `L` | Switch between Chinese and English |
| `P` | Cycle the whole-dashboard position |
| `S` | Select, enable, and reorder providers |
| `R` | Refresh usage immediately |
| `Q` | Exit |
| `Esc`, `Ctrl+C` | Exit |

In provider management, use arrow keys or `J` / `K` to select, `Space` to toggle, `U` / `D` to reorder, `Enter` to save, and `Esc` to cancel.

## Responsive layout

AIUsage calculates the layout from terminal dimensions and minimum card width. Six providers use 3×2 at 80×24, four use 2×2, three use one row when space permits, and one or two retain the single-column boxed layout.

Press `P` to cycle the complete dashboard through top-left, top-center, top-right, center, bottom-left, bottom-center, and bottom-right.

## Configuration

Preferences are stored at:

```text
~/.config/aiusage/config.toml
```

Only language, theme, position, enabled providers, and ordering are stored—never tokens, cookies, credentials, or usage snapshots. New users start with the white theme; an existing `theme = "green"` preference is preserved. Writes are atomic with user-only permissions. Missing, unreadable, or damaged configuration safely falls back without blocking startup.

See [`config.example.toml`](config.example.toml). Set `NO_COLOR` to disable color styling; Unicode borders and progress bars remain as structural UI elements.

## Data and privacy

- No usage, token, or configuration uploads
- No account provisioning or automated login
- No reading or sharing saved credentials
- No background daemon
- No telemetry
- Demo mode never invokes real adapters, authentication data, or remote usage APIs

## Compatibility and disclaimer

AIUsage is an unofficial community utility. Real usage retrieval depends on CLI clients installed and legitimately authenticated by the user, plus local structured interfaces or state currently exposed by those clients.

The Codex reader uses the CLI's local app-server rate-limit method. The Grok reader reads a bounded tail of its local structured billing log. Upstream client interfaces may change and require corresponding updates. AIUsage does not provide accounts, bypass login, share tokens, or imitate client identities.

Requirements:

- Linux (tested)
- Python 3.10 or newer
- ANSI-capable terminal; UTF-8 recommended
- Corresponding installed and user-authenticated CLI for real Codex/Grok usage

macOS is untested and may work. Windows is not currently supported.

## Troubleshooting

- **Not installed:** install the provider's official CLI through its normal workflow; AIUsage does not perform login.
- **Unavailable:** press `R`, then verify the client is healthy and has produced usage state.
- **Not supported:** the provider is available for UI/demo work but has no verified real reader.
- **Broken borders:** use a UTF-8 locale and a font with box-drawing characters.
- **Damaged configuration:** repair or remove `~/.config/aiusage/config.toml` to restore defaults.

## Development

```bash
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -e '.[dev]'
python -m compileall -q src tests tools
python -m unittest discover -s tests -v
python tools/check_sensitive.py
```

Tests are fully offline and do not require Codex or Grok login. New real provider adapters require reliable, verifiable sources; fabricated real usage and committed credentials are prohibited. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

Released under the [MIT License](LICENSE). Copyright uses the neutral “AIUsage contributors” holder and does not imply an individual's identity.
