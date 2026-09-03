import datetime as dt
import json
import os
import select
import shutil
import subprocess
import threading
import time
from collections import OrderedDict
from dataclasses import dataclass
from typing import Callable

from .models import Availability, ProviderUsage, RateLimitWindow

CODEX_TIMEOUT = 8
GROK_LOG = os.path.expanduser("~/.grok/logs/unified.jsonl")
DISCOVERY_TIMEOUT = 2.0


@dataclass(frozen=True)
class DiscoveryResult:
    installed: bool
    ready: bool
    usage_supported: bool
    usable: bool
    reason: str


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
    readiness: Callable[[], bool] | None = None

    def discover(self) -> DiscoveryResult:
        installed = bool(self.installed())
        supported = self.reader is not None
        if not installed:
            return DiscoveryResult(False, False, supported, False, "not_installed")
        if not supported:
            return DiscoveryResult(True, False, False, False, "unsupported")
        ready = bool(self.readiness()) if self.readiness else False
        return DiscoveryResult(True, ready, True, ready, "ready" if ready else "needs_login")

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


def _codex_ready():
    return bool(os.environ.get("CODEX_API_KEY")) or os.path.isfile(os.path.expanduser("~/.codex/auth.json"))


def _grok_ready():
    return os.path.isfile(GROK_LOG) and os.access(GROK_LOG, os.R_OK)


def bounded_discover(adapter: ProviderAdapter, timeout=DISCOVERY_TIMEOUT) -> DiscoveryResult:
    result = []

    def run():
        try:
            result.append(adapter.discover())
        except Exception:
            result.append(DiscoveryResult(False, False, adapter.reader is not None, False, "unavailable"))

    thread = threading.Thread(target=run, daemon=True)
    thread.start()
    thread.join(timeout)
    if thread.is_alive():
        return DiscoveryResult(False, False, adapter.reader is not None, False, "timeout")
    if not result or not isinstance(result[0], DiscoveryResult):
        return DiscoveryResult(False, False, adapter.reader is not None, False, "malformed")
    return result[0]


def discover_all(registry=None, timeout=DISCOVERY_TIMEOUT):
    registry = REGISTRY if registry is None else registry
    results = {}
    lock = threading.Lock()

    def run(key, adapter):
        value = bounded_discover(adapter, timeout)
        with lock:
            results[key] = value

    threads = [threading.Thread(target=run, args=(key, adapter), daemon=True) for key, adapter in registry.items()]
    for thread in threads:
        thread.start()
    deadline = time.monotonic() + timeout
    for thread in threads:
        thread.join(max(0, deadline - time.monotonic()))
    for key, adapter in registry.items():
        results.setdefault(key, DiscoveryResult(False, False, adapter.reader is not None, False, "timeout"))
    return results


REGISTRY = OrderedDict((adapter.key, adapter) for adapter in (
    ProviderAdapter("codex", "Codex", read_codex, _command("codex"), _codex_ready),
    ProviderAdapter("grok", "Grok", read_grok, _command("grok"), _grok_ready),
    ProviderAdapter("minimax", "MiniMax", None, _command("mmx")),
    ProviderAdapter("qoder", "Qoder", None, _command("qoder")),
    ProviderAdapter("qodercn", "Qoder CN", None, _command("qodercn")),
    ProviderAdapter("codebuddy", "CodeBuddy", None, _command("codebuddy", "cbc")),
    ProviderAdapter("traecode", "TraeCode", None, _command("traecli")),
    # ZCode currently has an official Linux desktop application, but no
    # verified headless CLI. Keep the UI candidate without guessing a binary.
    ProviderAdapter("zcode", "ZCode", None, lambda: False),
))
