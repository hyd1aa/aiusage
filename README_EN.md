# AIUsage

[简体中文](README.md) | **English**

A lightweight, responsive terminal dashboard for AI CLI usage and rate limits.

View remaining percentages, usage windows, and reset times from clients such as Codex and Grok in one SSH, VPS, or Linux terminal dashboard.

[Introduction](#introduction) · [Preview](#preview) · [Installation](#installation) · [Quick start](#quick-start) · [Support](#provider-support) · [Themes](#themes) · [Shortcuts](#keyboard-shortcuts) · [Configuration](#configuration)

## Introduction

AIUsage runs directly in the terminal without a web panel or background daemon. Launch `aiusage` to get a dashboard that adapts to SSH sessions, tmux windows, and VPS panes.

- Verified real usage readers for Codex and Grok
- Chinese by default for new users, with live English switching
- White and Green foreground themes
- One outer box, centered title, and natural content-driven size
- Responsive one-, two-, and three-column layouts
- Live system clock and 30-second usage refresh
- Unicode progress bars and low-flicker partial redraws
- No telemetry or usage/configuration uploads

## Preview

### Real mode

`aiusage` displays real usage only when a reliable local source exists. This is an English UI example; percentages and times come from your own installed clients:

```text
┌────────────────── AI USAGE ──────────────────┐
│                                              │
│   CODEX                                      │
│   5h     ███████░░░  37% left                │
│   Reset: Sep 03 02:50 CST                    │
│   Week   ███████░░░  35% left                │
│   Reset: Sep 07 10:27 CST                    │
│                                              │
│   GROK                                       │
│   Week   █████████░  53% left                │
│   Reset: Sep 05 23:14 CST                    │
│                                              │
│ System: 2026-09-02 23:43:32 CST              │
│ Usage updated: 23:43:15                      │
│                                              │
│ T Theme L Lang P Pos S Prov R Refresh Q Exit │
│                                              │
└──────────────────────────────────────────────┘
```

There is one outer box; providers do not get individual boxes. The box takes its natural height from the content and is then placed at the selected position instead of filling the terminal.

### Demo mode

`aiusage --demo` uses deterministic fixtures for UI previews, README captures, responsive layout tests, and i18n checks. The dashboard is prominently marked **`[DEMO]`**.

Claude, Gemini, DeepSeek, Kimi, GLM, and z.ai are UI/demo-ready but **do not have real usage readers**. Their demo percentages are not account data. See the full 80×24 text capture at [`docs/screenshots/demo-80x24.txt`](docs/screenshots/demo-80x24.txt).

## Installation

```bash
git clone https://github.com/hyd1aa/aiusage.git
cd aiusage
sudo ./install.sh
aiusage
```

The idempotent installer writes only `/usr/local/bin/aiusage` and `/usr/local/lib/aiusage`. Existing user configuration is preserved.

## Quick start

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
aiusage --version
aiusage --help
aiusage --demo --snapshot --size 80x24
```

## Provider support

| Provider | Real usage | Demo/UI | Status |
| --- | --- | --- | --- |
| Codex | ✅ | ✅ | Supported |
| Grok | ✅ | ✅ | Supported |
| Claude | ❌ | ✅ | UI/demo only |
| Gemini | ❌ | ✅ | UI/demo only |
| DeepSeek | ❌ | ✅ | UI/demo only |
| Kimi | ❌ | ✅ | UI/demo only |
| GLM | ❌ | ✅ | UI/demo only |
| z.ai | ❌ | ✅ | UI/demo only |

Real mode never substitutes demo values. An enabled provider without a verified reader is reported honestly as `Not installed`, `Unavailable`, or `Not supported`.

## Themes

New users start with the **White** theme. Press `T` while AIUsage is running to switch:

```text
White ↔ Green
```

Themes affect foreground elements only:

- Text
- Outer border
- Progress bars

AIUsage always preserves the user's native terminal background. It never sets white, green, gray, or RGB backgrounds and never fills the dashboard with background-colored spaces.

## Language switching

The default language for new users without a configuration file is Chinese. Press `L` to switch to English and save that preference.

An existing `language = "en"` setting remains English after upgrades. Provider brand names are never translated.

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `T` | White / Green theme |
| `L` | Chinese / English |
| `P` | Change dashboard position |
| `S` | Manage providers |
| `R` | Refresh immediately |
| `Q` | Exit |
| `Esc` | Exit |
| `Ctrl+C` | Exit |

In provider management, use arrow keys or `J` / `K` to select, `Space` to toggle, `U` / `D` to reorder, `Enter` to save, and `Esc` to cancel.

## Responsive layout

- 1–2 providers: compact single column
- 3 providers: 3×1 when space allows, otherwise fewer columns
- 4 providers: centered 2×2
- 5–6 providers: centered 3×2
- Narrower terminals: automatically reduce to two or one column

Every layout uses one outer box. The title is centered against the actual box width, while the compact provider grid is centered as one content block. Natural sizing means the box never expands merely to fill the terminal. The 80×24 layout is verified, and smaller terminals respond automatically.

## Provider management

Press `S` to enable, disable, or reorder providers. Real mode defaults to Codex and Grok; demo mode defaults to six providers. Real and demo selections are saved separately, so demo configuration never becomes real usage.

## Dashboard position

Press `P` to cycle and save:

```text
top-left / top-center / top-right / center /
bottom-left / bottom-center / bottom-right
```

Positioning moves the complete natural-size box without stretching its internal content.

## Time and timezones

AIUsage converts reset epochs to the machine's local timezone and displays explicit labels such as `CST`, `EDT`, or `UTC`, preventing reset-time ambiguity between regions.

```text
Chinese: 9月03日 02:50 CST
English: Sep 03 02:50 CST
```

System time includes the local timezone as well. AIUsage does not assume every user is in CST.

## Configuration

Preferences are stored at:

```text
~/.config/aiusage/config.toml
```

Example:

```toml
language = "zh"
theme = "white"
position = "center"
real_providers = ["codex", "grok"]
demo_providers = ["codex", "grok", "deepseek", "claude", "gemini", "kimi"]
```

The file stores only language, theme, position, enabled providers, and ordering—never tokens, cookies, accounts, IP addresses, hostnames, or usage snapshots. Writes are atomic with user-only permissions. Missing, unreadable, or damaged configuration safely falls back. See [`config.example.toml`](config.example.toml).

## Privacy and security

- No usage, token, or configuration uploads
- No account provisioning or automated login
- No reading or sharing saved credentials
- No background daemon
- No telemetry
- Demo mode never invokes real adapters, authentication data, or remote usage APIs

AIUsage is an unofficial community utility. The Codex reader uses the CLI's local app-server rate-limit method. The Grok reader reads a bounded tail of its local structured billing log. Upstream interfaces may change and require updates. AIUsage does not bypass login, share tokens, or imitate client identities. See [`SECURITY.md`](SECURITY.md) for reporting guidance.

## System requirements

- Linux (tested)
- Python 3.10 or newer
- ANSI-capable terminal; UTF-8 recommended
- Corresponding CLI installed and legitimately authenticated by the user for real Codex/Grok usage

macOS is untested and may work. Windows is not currently supported. Set `NO_COLOR` to disable colored foreground styles while retaining structural Unicode borders and progress bars.

## Uninstall

```bash
sudo ./uninstall.sh
```

Uninstall removes AIUsage program files but preserves `~/.config/aiusage/`. Remove that directory explicitly only if you also want to delete preferences.

## Development and contributing

```bash
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -e '.[dev]'
python -m compileall -q src tests tools
python -m unittest discover -s tests -v
python tools/check_sensitive.py
```

Tests are fully offline and require no Codex or Grok login. New real provider adapters require reliable, verifiable sources; fabricated real usage and committed credentials are prohibited. See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`CHANGELOG.md`](CHANGELOG.md).

## License

Released under the [MIT License](LICENSE). Copyright uses the neutral “AIUsage contributors” holder and does not imply an individual's identity.
