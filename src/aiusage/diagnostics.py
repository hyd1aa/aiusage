import datetime as dt
import locale
import platform

from . import __version__
from .providers import REGISTRY, discover_all
from .timezones import convert, offset_label


def collect(cfg, github_ok=None):
    rows = [
        ("AIUsage", True, f"v{__version__}"),
        ("Python", tuple(map(int, platform.python_version_tuple()[:2])) >= (3, 10), platform.python_version()),
        ("Terminal", bool((locale.getpreferredencoding(False) or "").lower().replace("-", "").startswith("utf8")), locale.getpreferredencoding(False)),
        ("Config", True, "readable"),
    ]
    discovery = discover_all(REGISTRY)
    for key, adapter in REGISTRY.items():
        state = discovery[key]
        disabled = key in cfg.disabled_providers
        rows.append((adapter.name, state.usable and not disabled, "disabled_by_user" if disabled else state.reason))
    for key in ("codex", "grok"):
        adapter = REGISTRY[key]
        discovered = discovery[key]
        state = adapter.read() if discovered.usable else None
        readable = bool(state and state.windows)
        rows.append((f"{adapter.name} usage", readable, "readable" if readable else "unavailable"))
    instant = dt.datetime.now(dt.timezone.utc)
    now = convert(instant, "system")
    display = convert(instant, cfg.timezone)
    rows.extend([
        ("System timezone", True, offset_label(now)),
        ("Display timezone", True, offset_label(display)),
        ("GitHub", github_ok is True, "available" if github_ok is True else "unknown" if github_ok is None else "unavailable"),
    ])
    return rows


def sanitized_text(rows):
    return "\n".join(f"{name}: {'OK' if ok else 'WARN'} ({detail})" for name, ok, detail in rows)
