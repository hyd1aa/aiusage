import datetime as dt
import json
import os
import select
import shutil
import subprocess
import time
from collections import OrderedDict
from dataclasses import dataclass
from typing import Callable

from .models import Availability, ProviderUsage, RateLimitWindow

CODEX_TIMEOUT = 8
GROK_LOG = os.path.expanduser("~/.grok/logs/unified.jsonl")


def _label(minutes, fallback="Limit"):
    if not isinstance(minutes, (int, float)):
        return fallback
    if minutes == 1440:
        return "Daily"
    if minutes == 10080:
        return "Week"
    if minutes % 60 == 0 and minutes < 1440:
        return f"{int(minutes / 60)}h"
    return fallback


def _timestamp(value):
    if isinstance(value, (int, float)):
        return float(value)
    if not isinstance(value, str):
        return None
    try:
        return dt.datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()
    except ValueError:
        return None


def read_codex() -> tuple[RateLimitWindow, ...]:
    executable = shutil.which("codex")
    if not executable:
        raise FileNotFoundError("Codex is not installed")
    proc = subprocess.Popen(
        [executable, "app-server", "--stdio"], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1,
    )

    def send(value):
        proc.stdin.write(json.dumps(value, separators=(",", ":")) + "\n")
        proc.stdin.flush()

    def response(request_id, deadline):
        while time.monotonic() < deadline:
            ready, _, _ = select.select([proc.stdout], [], [], max(0, deadline-time.monotonic()))
            if not ready:
                break
            line = proc.stdout.readline()
            if not line:
                break
            try:
                item = json.loads(line)
            except json.JSONDecodeError:
                continue
            if item.get("id") == request_id:
                return item
        raise TimeoutError("Codex usage timeout")

    try:
        deadline = time.monotonic() + CODEX_TIMEOUT
        send({"id": 1, "method": "initialize", "params": {"clientInfo": {"name": "aiusage", "version": "2"}}})
        if "error" in response(1, deadline):
            raise RuntimeError("Codex initialization failed")
        send({"method": "initialized", "params": {}})
        send({"id": 2, "method": "account/rateLimits/read", "params": {}})
        reply = response(2, deadline)
        if "error" in reply:
            raise RuntimeError("Codex usage unavailable")
        snapshot = reply.get("result", {}).get("rateLimits", {})
        windows = []
        for key, fallback in (("primary", "Primary"), ("secondary", "Secondary")):
            window = snapshot.get(key)
            if not isinstance(window, dict) or "usedPercent" not in window:
                continue
            used = max(0, min(100, int(round(window["usedPercent"]))))
            windows.append(RateLimitWindow(_label(window.get("windowDurationMins"), fallback), 100-used, window.get("resetsAt")))
        if not windows:
            raise RuntimeError("No Codex rate-limit windows")
        return tuple(windows)
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=1)
        except subprocess.TimeoutExpired:
            proc.kill()


def read_grok() -> tuple[RateLimitWindow, ...]:
    latest = None
    with open(GROK_LOG, "rb") as stream:
        stream.seek(0, os.SEEK_END)
        size = stream.tell()
        stream.seek(max(0, size - 4 * 1024 * 1024))
        if size > 4 * 1024 * 1024:
            stream.readline()
        for raw in stream:
            if b"creditUsagePercent" not in raw:
                continue
            try:
                item = json.loads(raw)
            except (json.JSONDecodeError, UnicodeDecodeError):
                continue
            config = item.get("ctx", {}).get("config", {})
            used = config.get("creditUsagePercent") if isinstance(config, dict) else None
            reset = _timestamp(config.get("billingPeriodEnd")) if isinstance(config, dict) else None
            start = _timestamp(config.get("billingPeriodStart")) if isinstance(config, dict) else None
            if not isinstance(used, (int, float)) or reset is None:
                continue
            duration = (reset-start)/60 if start is not None else None
            latest = RateLimitWindow(_label(round(duration) if duration else None, "Cycle"), 100-max(0, min(100, int(round(used)))), reset)
    if latest is None:
        raise RuntimeError("No reliable Grok billing snapshot")
    return (latest,)


@dataclass(frozen=True)
class ProviderAdapter:
    key: str
    name: str
    reader: Callable[[], tuple[RateLimitWindow, ...]] | None
    installed: Callable[[], bool]

    def read(self) -> ProviderUsage:
        if not self.installed():
            return ProviderUsage(self.key, self.name, Availability.NOT_INSTALLED)
        if self.reader is None:
            return ProviderUsage(self.key, self.name, Availability.NOT_SUPPORTED)
        try:
            windows = self.reader()
            if not isinstance(windows, (tuple, list)) or not windows:
                raise ValueError("Malformed rate-limit response")
            if any(
                not isinstance(window, RateLimitWindow)
                or not isinstance(window.label, str)
                or not isinstance(window.remaining_percent, (int, float))
                for window in windows
            ):
                raise ValueError("Malformed rate-limit window")
            return ProviderUsage(self.key, self.name, Availability.AVAILABLE, tuple(windows))
        except Exception as exc:
            return ProviderUsage(self.key, self.name, Availability.UNAVAILABLE, error=str(exc))


def _command(*names):
    return lambda: any(shutil.which(name) for name in names)


REGISTRY = OrderedDict((adapter.key, adapter) for adapter in (
    ProviderAdapter("codex", "Codex", read_codex, _command("codex")),
    ProviderAdapter("grok", "Grok", read_grok, _command("grok")),
    ProviderAdapter("claude", "Claude", None, _command("claude")),
    ProviderAdapter("gemini", "Gemini", None, _command("gemini")),
    ProviderAdapter("deepseek", "DeepSeek", None, _command("deepseek")),
    ProviderAdapter("kimi", "Kimi", None, _command("kimi")),
    ProviderAdapter("glm", "GLM", None, _command("glm")),
    ProviderAdapter("zai", "z.ai", None, _command("zai", "z.ai")),
))


def demo_usage(key: str) -> ProviderUsage:
    # Pure local fixtures: this function intentionally has no adapter access.
    values = {
        "codex": (("5h", 83, 2.2), ("Week", 61, 74)),
        "grok": (("Cycle", 72, 51),),
        "claude": (("5h", 48, 3.5),),
        "gemini": (("Daily", 91, 8),),
        "deepseek": (("Daily", 66, 11),),
        "kimi": (("Monthly", 37, 240),),
        "glm": (("Daily", 79, 13),),
        "zai": (("Cycle", 55, 36),),
    }
    now = time.time()
    adapter = REGISTRY[key]
    windows = tuple(RateLimitWindow(label, remaining, now + hours*3600) for label, remaining, hours in values[key])
    return ProviderUsage(key, adapter.name, Availability.AVAILABLE, windows)
