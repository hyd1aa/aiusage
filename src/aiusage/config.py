import os
from dataclasses import dataclass, field
from pathlib import Path

from .providers import REGISTRY

POSITIONS = ("top-left", "top-center", "top-right", "center", "bottom-left", "bottom-center", "bottom-right")
DEMO_DEFAULT = ["codex", "grok", "deepseek", "claude", "gemini", "kimi"]


@dataclass
class Config:
    language: str = "en"
    position: str = "center"
    real_providers: list[str] = field(default_factory=lambda: ["codex", "grok"])
    demo_providers: list[str] = field(default_factory=lambda: list(DEMO_DEFAULT))


def config_path() -> Path:
    base = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    return base / "aiusage" / "config.toml"


def _array(value):
    if not value.startswith("[") or not value.endswith("]"):
        return []
    return [part.strip().strip('"').strip("'") for part in value[1:-1].split(",") if part.strip()]


def load(path: Path | None = None) -> Config:
    cfg = Config()
    target = path or config_path()
    try:
        lines = target.read_text(encoding="utf-8").splitlines()
    except FileNotFoundError:
        return cfg
    values = {}
    for raw in lines:
        line = raw.split("#", 1)[0].strip()
        if "=" in line:
            key, value = line.split("=", 1)
            values[key.strip()] = value.strip()
    cfg.language = values.get("language", '"en"').strip('"')
    cfg.position = values.get("position", '"center"').strip('"')
    for attr in ("real_providers", "demo_providers"):
        if attr in values:
            setattr(cfg, attr, _array(values[attr]))
    if cfg.language not in ("en", "zh"):
        cfg.language = "en"
    if cfg.position not in POSITIONS:
        cfg.position = "center"
    cfg.real_providers = _valid(cfg.real_providers) or ["codex", "grok"]
    cfg.demo_providers = _valid(cfg.demo_providers) or list(DEMO_DEFAULT)
    return cfg


def _valid(items):
    return list(dict.fromkeys(item for item in items if item in REGISTRY))


def save(cfg: Config, path: Path | None = None):
    target = path or config_path()
    target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    def quoted(items):
        return ", ".join(f'"{item}"' for item in items)
    body = (
        f'language = "{cfg.language}"\n'
        f'position = "{cfg.position}"\n'
        f'real_providers = [{quoted(_valid(cfg.real_providers))}]\n'
        f'demo_providers = [{quoted(_valid(cfg.demo_providers))}]\n'
    )
    temporary = target.with_suffix(".tmp")
    temporary.write_text(body, encoding="utf-8")
    os.chmod(temporary, 0o600)
    os.replace(temporary, target)

