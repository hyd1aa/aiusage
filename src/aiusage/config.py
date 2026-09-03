import os
from dataclasses import dataclass, field
from pathlib import Path

from .providers import REGISTRY
from .timezones import valid_timezone

POSITIONS = ("top-left", "top-center", "top-right", "center", "bottom-left", "bottom-center", "bottom-right")
THEMES = ("white", "green")
DEMO_DEFAULT = ["codex", "grok", "minimax", "qoder", "codebuddy", "traecode"]


@dataclass
class Config:
    language: str = "zh"
    theme: str = "white"
    position: str = "center"
    timezone: str = "system"
    auto_discover: bool = True
    real_providers: list[str] = field(default_factory=lambda: ["codex", "grok"])
    demo_providers: list[str] = field(default_factory=lambda: list(DEMO_DEFAULT))
    disabled_providers: list[str] = field(default_factory=list)


def config_path() -> Path:
    base = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    return base / "aiusage" / "config.toml"


def _array(value):
    if not value.startswith("[") or not value.endswith("]"):
        return []
    return [part.strip().strip('"').strip("'") for part in value[1:-1].split(",") if part.strip()]


def _boolean(value, default=True):
    normalized = value.strip().lower()
    return True if normalized == "true" else False if normalized == "false" else default


def load(path: Path | None = None) -> Config:
    cfg = Config()
    target = path or config_path()
    try:
        lines = target.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError):
        return cfg
    values = {}
    for raw in lines:
        line = raw.split("#", 1)[0].strip()
        if "=" in line:
            key, value = line.split("=", 1)
            values[key.strip()] = value.strip()
    cfg.language = values.get("language", '"zh"').strip('"')
    cfg.theme = values.get("theme", '"white"').strip('"')
    cfg.position = values.get("position", '"center"').strip('"')
    cfg.timezone = values.get("timezone", '"system"').strip('"')
    cfg.auto_discover = _boolean(values.get("auto_discover", "true"))
    for attr in ("real_providers", "demo_providers", "disabled_providers"):
        if attr in values:
            setattr(cfg, attr, _array(values[attr]))
    if cfg.language not in ("en", "zh"):
        cfg.language = "zh"
    if cfg.theme not in THEMES:
        cfg.theme = "white"
    if cfg.position not in POSITIONS:
        cfg.position = "center"
    if not valid_timezone(cfg.timezone):
        cfg.timezone = "system"
    cfg.real_providers = _valid(cfg.real_providers) or ["codex", "grok"]
    cfg.demo_providers = _valid(cfg.demo_providers) or list(DEMO_DEFAULT)
    cfg.disabled_providers = [key for key in _valid(cfg.disabled_providers) if key not in cfg.real_providers]
    if "real_providers" in values and "disabled_providers" not in values:
        cfg.disabled_providers.extend(key for key in ("codex", "grok") if key not in cfg.real_providers)
    return cfg


def _valid(items):
    return list(dict.fromkeys(item for item in items if item in REGISTRY))


def save(cfg: Config, path: Path | None = None) -> bool:
    target = path or config_path()
    def quoted(items):
        return ", ".join(f'"{item}"' for item in items)
    body = (
        f'language = "{cfg.language}"\n'
        f'theme = "{cfg.theme}"\n'
        f'position = "{cfg.position}"\n'
        f'timezone = "{cfg.timezone}"\n'
        f'auto_discover = {str(cfg.auto_discover).lower()}\n'
        f'real_providers = [{quoted(_valid(cfg.real_providers))}]\n'
        f'demo_providers = [{quoted(_valid(cfg.demo_providers))}]\n'
        f'disabled_providers = [{quoted(_valid(cfg.disabled_providers))}]\n'
    )
    temporary = target.with_suffix(".tmp")
    try:
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        temporary.write_text(body, encoding="utf-8")
        os.chmod(temporary, 0o600)
        os.replace(temporary, target)
        return True
    except OSError:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass
        return False
