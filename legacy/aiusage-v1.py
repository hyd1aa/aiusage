#!/usr/bin/env python3
"""Codex + Grok quota dashboard."""

import datetime as dt
import json
import os
import select
import shutil
import signal
import subprocess
import sys
import termios
import threading
import time
import tty

REFRESH_SECONDS = 30
CODEX_TIMEOUT = 8
GROK_LOG = os.path.expanduser("~/.grok/logs/unified.jsonl")


def local_now():
    return dt.datetime.now().astimezone()


def label_for_minutes(minutes, fallback="Limit"):
    if not isinstance(minutes, (int, float)):
        return fallback
    if minutes == 60 * 24:
        return "Daily"
    if minutes == 60 * 24 * 7:
        return "Week"
    if minutes % 60 == 0 and minutes < 60 * 24:
        return f"{int(minutes / 60)}h"
    return fallback


def read_codex():
    """Read the official Codex app-server rate-limit snapshot."""
    proc = subprocess.Popen(
        ["/usr/bin/codex", "app-server", "--stdio"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        text=True, bufsize=1,
    )

    def send(obj):
        proc.stdin.write(json.dumps(obj, separators=(",", ":")) + "\n")
        proc.stdin.flush()

    def response(request_id, deadline):
        while time.monotonic() < deadline:
            ready, _, _ = select.select(
                [proc.stdout], [], [], max(0, deadline - time.monotonic())
            )
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
        send({"id": 1, "method": "initialize", "params": {
            "clientInfo": {"name": "aiusage", "version": "1"}
        }})
        init = response(1, deadline)
        if "error" in init:
            raise RuntimeError("Codex initialization failed")
        send({"method": "initialized", "params": {}})
        send({"id": 2, "method": "account/rateLimits/read", "params": {}})
        reply = response(2, deadline)
        if "error" in reply:
            raise RuntimeError("Codex usage unavailable")
        snapshot = reply.get("result", {}).get("rateLimits", {})
        rows = []
        for key, fallback in (("primary", "Primary"), ("secondary", "Secondary")):
            window = snapshot.get(key)
            if not isinstance(window, dict) or "usedPercent" not in window:
                continue
            used = max(0, min(100, int(round(window["usedPercent"]))))
            rows.append({
                "name": label_for_minutes(window.get("windowDurationMins"), fallback),
                "remaining": 100 - used,
                "reset": window.get("resetsAt"),
            })
        if not rows:
            raise RuntimeError("No Codex rate-limit windows")
        return rows
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=1)
        except subprocess.TimeoutExpired:
            proc.kill()


def parse_timestamp(value):
    if isinstance(value, (int, float)):
        return float(value)
    if not isinstance(value, str):
        return None
    try:
        return dt.datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()
    except ValueError:
        return None


def read_grok():
    """Read Grok's structured, server-populated billing snapshot from its log."""
    latest = None
    with open(GROK_LOG, "rb") as stream:
        # Quota records are small and regularly repeated. A bounded tail avoids
        # scanning an indefinitely growing log every 30 seconds.
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
            if not isinstance(config, dict):
                continue
            used = config.get("creditUsagePercent")
            reset = parse_timestamp(config.get("billingPeriodEnd"))
            start = parse_timestamp(config.get("billingPeriodStart"))
            if not isinstance(used, (int, float)) or reset is None:
                continue
            source_time = parse_timestamp(
                item.get("timestamp") or item.get("ts") or item.get("time")
            )
            duration = ((reset - start) / 60) if start is not None else None
            latest = ({
                "name": label_for_minutes(round(duration) if duration else None, "Cycle"),
                "remaining": 100 - max(0, min(100, int(round(used)))),
                "reset": reset,
            }, source_time)
    if latest is None:
        raise RuntimeError("No reliable Grok billing snapshot")
    return [latest[0]]


def reset_text(epoch, ultra=False):
    if not isinstance(epoch, (int, float)):
        return "?"
    value = dt.datetime.fromtimestamp(epoch).astimezone()
    now = local_now()
    if value.date() == now.date():
        return value.strftime("%H:%M")
    if ultra:
        return value.strftime("%b%d")
    return value.strftime("%b %d %H:%M")


def shorten(text, width):
    aliases = {"Session": "Sess", "Secondary": "Sec", "Primary": "Pri",
               "Daily": "Day", "Cycle": "Cyc"}
    text = aliases.get(text, text)
    return text[:max(1, width)]


def display_name(value):
    return "Weekly" if value == "Week" else str(value)


def compact_lines(width, height, codex, grok, updated):
    """Last-resort view for terminals that genuinely cannot fit the card."""
    now = local_now()
    lines = [f"AI USAGE  {now:%H:%M:%S}"[:width]]
    for provider, state in (("CODEX", codex), ("GROK", grok)):
        rows, stale = state
        lines.append((provider + (" !" if stale else ""))[:width])
        if not rows:
            lines.append("unavailable"[:width])
            continue
        for row in rows:
            label = shorten(display_name(row["name"]), 7)
            lines.append(f'{label:<7} {row["remaining"]:>3}% left'[:width])
    upd = updated.astimezone().strftime("%H:%M:%S") if updated else "--:--:--"
    lines.extend([f"Updated {upd}"[:width], "q / Esc / Ctrl+C : exit"[:width]])
    return lines[:height]


def boxed_content(codex, grok, updated):
    lines = [""]
    for provider, state in (("CODEX", codex), ("GROK", grok)):
        rows, stale = state
        lines.append(provider + ("  ! stale" if stale else ""))
        if not rows:
            lines.append("unavailable")
        else:
            for row in rows:
                remaining = max(0, min(100, int(row["remaining"])))
                fill = round(remaining * 12 / 100)
                bar = "█" * fill + "░" * (12 - fill)
                lines.append(
                    f'{display_name(row["name"]):<7} {bar}  {remaining:>3}% left'
                )
                lines.append(f'Reset    {reset_text(row.get("reset"))}')
        lines.append("")
    now = local_now()
    zone = now.tzname() or ""
    lines.append(f"System: {now:%Y-%m-%d %H:%M:%S} {zone}".rstrip())
    upd = updated.astimezone().strftime("%H:%M:%S") if updated else "--:--:--"
    lines.append(f"Usage updated: {upd}")
    lines.extend(["", "q / Esc / Ctrl+C : exit", ""])
    return lines


def render_lines(width, height, codex, grok, updated, color=True):
    width, height = max(1, width), max(1, height)
    content = boxed_content(codex, grok, updated)
    card_width = 42
    required_height = len(content) + 2
    if width < card_width or height < required_height:
        return compact_lines(width, height, codex, grok, updated)

    inner = card_width - 2
    content_width = inner - 2
    left = " " * ((width - card_width) // 2)
    top_pad = min(2, max(0, (height - required_height) // 3))
    gray = "\x1b[97m" if color else ""
    bright = "\x1b[1;97m" if color else ""
    reset = "\x1b[0m" if color else ""
    title = " AI USAGE "
    side = inner - len(title)
    top_line = ("┌" + "─" * (side // 2) +
                (bright + title + reset + gray) +
                "─" * (side - side // 2) + "┐")
    result = [""] * top_pad + [left + gray + top_line + reset]
    for line in content:
        visible = line[:content_width]
        styled = bright + visible + reset if visible.startswith(("CODEX", "GROK")) else gray + visible + reset
        result.append(left + gray + "│ " + reset + styled + " " * (content_width - len(visible)) + gray + " │" + reset)
    result.append(left + gray + "└" + "─" * inner + "┘" + reset)
    return result


class Dashboard:
    def __init__(self):
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.codex = (None, False)
        self.grok = (None, False)
        self.updated = None

    def refresh(self):
        successes = 0
        for attr, reader in (("codex", read_codex), ("grok", read_grok)):
            try:
                rows = reader()
            except Exception:
                with self.lock:
                    old, _ = getattr(self, attr)
                    setattr(self, attr, (old, old is not None))
            else:
                successes += 1
                with self.lock:
                    setattr(self, attr, (rows, False))
        if successes:
            with self.lock:
                self.updated = local_now()

    def worker(self):
        self.refresh()
        while not self.stop.wait(REFRESH_SECONDS):
            self.refresh()

    def frame(self):
        size = shutil.get_terminal_size((40, 12))
        with self.lock:
            return render_lines(size.columns, size.lines,
                                self.codex, self.grok, self.updated,
                                color=os.environ.get("TERM") != "dumb")


def paint(lines, previous):
    height = shutil.get_terminal_size((40, 12)).lines
    parts = []
    extent = min(height, max(len(previous), len(lines)))
    for row in range(1, extent + 1):
        text = lines[row - 1] if row <= len(lines) else ""
        old = previous[row - 1] if row <= len(previous) else None
        if text != old:
            parts.append(f"\x1b[{row};1H\x1b[2K{text}")
    if parts:
        sys.stdout.write("".join(parts))
        sys.stdout.flush()
    return lines


def main():
    if not sys.stdin.isatty() or not sys.stdout.isatty():
        print("aiusage requires an interactive terminal", file=sys.stderr)
        return 2
    original = termios.tcgetattr(sys.stdin.fileno())
    board = Dashboard()
    thread = threading.Thread(target=board.worker, daemon=True)
    quitting = False

    def request_exit(_signum=None, _frame=None):
        nonlocal quitting
        quitting = True

    old_handlers = {sig: signal.getsignal(sig) for sig in (signal.SIGINT, signal.SIGTERM)}
    for sig in old_handlers:
        signal.signal(sig, request_exit)
    signal.signal(signal.SIGWINCH, lambda _s, _f: None)
    try:
        tty.setcbreak(sys.stdin.fileno())
        sys.stdout.write("\x1b[?1049h\x1b[?25l\x1b[?7l")
        sys.stdout.flush()
        thread.start()
        prior = []
        while not quitting:
            prior = paint(board.frame(), prior)
            ready, _, _ = select.select([sys.stdin], [], [], 0.2)
            if ready:
                key = os.read(sys.stdin.fileno(), 1)
                if key in (b"q", b"Q", b"\x1b", b"\x03"):
                    break
    finally:
        board.stop.set()
        termios.tcsetattr(sys.stdin.fileno(), termios.TCSADRAIN, original)
        sys.stdout.write("\x1b[?7h\x1b[?25h\x1b[?1049l\x1b[0m")
        sys.stdout.flush()
        for sig, handler in old_handlers.items():
            signal.signal(sig, handler)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
