# AIUsage

A lightweight, responsive terminal dashboard for AI CLI usage and rate limits.

AIUsage shows verified local rate-limit information from supported CLI clients
without running a daemon or sending telemetry. Its deterministic demo mode is
safe for layout development, documentation, and screenshots.

## Demo

```text
AI USAGE [DEMO]  System 12:34:56
┌────────────────────────┐ ┌────────────────────────┐ ┌────────────────────────┐
│ CODEX                  │ │ GROK                   │ │ DEEPSEEK               │
│ 5h     ████████░░  83% │ │ Cycle  ███████░░░  72% │ │ Daily  ███████░░░  66% │
│ Reset: 14:46           │ │ Reset: Sep05           │ │ Reset: 23:34           │
└────────────────────────┘ └────────────────────────┘ └────────────────────────┘

┌────────────────────────┐ ┌────────────────────────┐ ┌────────────────────────┐
│ CLAUDE                 │ │ GEMINI                 │ │ KIMI                   │
│ 5h     █████░░░░░  48% │ │ Daily  █████████░  91% │ │ Monthl ████░░░░░░  37% │
│ Reset: 16:04           │ │ Reset: 20:34           │ │ Reset: Sep12           │
└────────────────────────┘ └────────────────────────┘ └────────────────────────┘
```

This shortened example is explicitly demo data. A complete 80×24 text capture
is kept at [`docs/screenshots/demo-80x24.txt`](docs/screenshots/demo-80x24.txt).
Future PNG captures should be placed in `docs/screenshots/` and must be made
from `aiusage --demo`, without shell prompts, hostnames, or other panes.

## Features

- Verified real-mode readers for Codex and Grok.
- Strictly isolated, deterministic demo mode with a prominent `[DEMO]` label.
- Responsive one-, two-, and three-column layouts, optimized for 80×24.
- English and Chinese UI, live resize handling, and Unicode progress bars.
- Provider selection and ordering plus seven whole-dashboard positions.
- Thirty-second usage refresh, one-second clock updates, stale-data handling,
  low-flicker partial redraws, and careful terminal cleanup.
- Standard-library-only runtime; no background service and no telemetry.

## Supported providers

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

An enabled provider without a verified reader is shown as `Not installed`,
`Unavailable`, or `Not supported`; AIUsage never substitutes mock percentages
in real mode.

## Requirements

- Linux (tested).
- Python 3.10 or newer.
- An ANSI-capable terminal; UTF-8 is recommended.
- Codex CLI installed and already signed in for Codex real usage.
- Grok CLI installed and an existing structured billing snapshot for Grok real
  usage.

macOS has not been tested and may work. Windows is not currently supported.

## Installation

```sh
git clone https://github.com/hyd1aa/aiusage.git
cd aiusage
sudo ./install.sh
aiusage
```

The installer is idempotent, adds `/usr/local/bin/aiusage`, and installs only
AIUsage runtime files under `/usr/local/lib/aiusage`. It does not change user
configuration. To remove the program while retaining settings:

```sh
sudo ./uninstall.sh
```

To also remove settings, explicitly remove `~/.config/aiusage/` afterward.

## Usage

Real mode uses only verified provider readers:

```sh
aiusage
```

Demo mode uses deterministic local fixtures and never invokes a real adapter:

```sh
aiusage --demo
```

Other useful commands:

```sh
aiusage --help
aiusage --version
aiusage --demo --snapshot --size 80x24
```

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `L` | Switch language between English and Chinese |
| `P` | Cycle dashboard position |
| `S` | Open provider selection and ordering |
| `R` | Refresh usage |
| `Q` | Exit |
| `Esc`, `Ctrl+C` | Exit |

Inside provider management, use the arrow keys (or `J`/`K`) to select,
`Space` to enable or disable, `U`/`D` to reorder, `Enter` to save, and `Esc`
to cancel.

## Display and configuration

AIUsage calculates column count from terminal size and minimum card width. Six
providers use a 3×2 grid at 80×24, four use 2×2, and one or two retain a single
boxed column. `P` moves the complete dashboard among top-left, top-center,
top-right, center, bottom-left, bottom-center, and bottom-right.

Settings are written atomically with user-only file permissions to:

```text
~/.config/aiusage/config.toml
```

Only UI preferences are stored: language, position, enabled providers, and
provider order. No token, cookie, credential, or usage snapshot is written.
See [`config.example.toml`](config.example.toml). Missing, unreadable, or
damaged configuration falls back to safe defaults rather than preventing the
dashboard from starting.

Set `NO_COLOR` to request color-free output. Borders and progress bars remain
Unicode because they are structural rather than color; terminal control
sequences are still used for interactive screen management and cleanup.

## Data and privacy

AIUsage reads provider state locally. It does not upload usage data, tokens,
or configuration; it does not start a background daemon; and it has no
telemetry. Demo mode does not inspect installed clients, authentication files,
or network APIs.

AIUsage does not provide accounts, bypass login, share tokens, imitate a
client identity, or perform authentication. Users are responsible for using
provider clients and accounts in accordance with their applicable terms.

## Provider compatibility and disclaimer

AIUsage is an unofficial community utility. Rate-limit retrieval may depend on
interfaces exposed by installed CLI clients and may require updates when
upstream clients change. Codex currently uses the CLI's local app-server
rate-limit method. Grok reads a bounded tail of its locally generated,
structured billing log. Both require the corresponding CLI and a session the
user has already established legitimately.

No guarantee is made that upstream local interfaces remain stable. Unsupported
providers remain UI/registry-ready until a reliable and verifiable source is
implemented.

## Troubleshooting

- `Not installed`: install the provider's official CLI and sign in through its
  normal workflow; AIUsage does not perform login.
- `Unavailable`: refresh with `R`, then verify the installed client itself is
  healthy and has produced rate-limit information.
- `Not supported`: the provider is selectable for UI readiness, but no verified
  real reader exists yet.
- Garbled borders: use a UTF-8 locale and a terminal font with box-drawing
  characters.
- A bad configuration does not block startup; remove or repair only
  `~/.config/aiusage/config.toml` if you want to restore defaults.

## Development

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -e '.[dev]'
python -m compileall -q src tests tools
python -m unittest discover -s tests -v
python tools/check_sensitive.py
```

Tests are offline: real provider processes and credentials are mocked or
avoided. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the adapter contract and
contribution rules.

## License

MIT. See [`LICENSE`](LICENSE). Copyright is held neutrally by AIUsage
contributors; no individual identity is implied.
