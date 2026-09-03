import argparse
import os
import select
import shutil
import signal
import sys
import termios
import threading
import time
import tty

from . import __version__, config
from .demo import demo_usage
from .models import Availability, ProviderUsage
from .providers import REGISTRY
from .render import dashboard, selector, timezone_selector
from .timezones import MINUTES_MAX, MINUTES_MIN, PRESETS, offset_minutes, offset_setting

REFRESH_SECONDS = 30


class Dashboard:
    def __init__(self, demo=False, cfg=None, color=False):
        self.demo = demo
        self.cfg = cfg or config.load()
        self.color = color
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.updated = None
        self.states = {}
        self.selecting = False
        self.cursor = 0
        self.draft = []
        self.timezone_selecting = False
        self.timezone_cursor = 0
        self.timezone_options = []

    @property
    def enabled(self):
        return self.cfg.demo_providers if self.demo else self.cfg.real_providers

    @enabled.setter
    def enabled(self, value):
        if self.demo:
            self.cfg.demo_providers = value
        else:
            self.cfg.real_providers = value

    def refresh(self):
        success = False
        for key in list(self.enabled):
            if self.demo:
                state = demo_usage(key)
            else:
                fresh = REGISTRY[key].read()
                old = self.states.get(key)
                if fresh.availability == Availability.UNAVAILABLE and old and old.windows:
                    state = ProviderUsage(old.key, old.name, old.availability, old.windows, True, fresh.error)
                else:
                    state = fresh
            self.states[key] = state
            success = success or state.availability == Availability.AVAILABLE
        if success:
            self.updated = time_now()

    def worker(self):
        self.refresh()
        while not self.stop.wait(REFRESH_SECONDS):
            self.refresh()

    def frame(self, width=None, height=None):
        size = shutil.get_terminal_size((80, 24))
        width, height = width or size.columns, height or size.lines
        with self.lock:
            if self.selecting:
                return selector(width, height, REGISTRY, self.draft, self.cursor, self.cfg.language, self.cfg.theme, self.color)
            if self.timezone_selecting:
                return timezone_selector(width, height, self.timezone_options, self.timezone_cursor, self.cfg.language, self.cfg.theme, self.color)
            states = [self.states.get(key, ProviderUsage(key, REGISTRY[key].name, Availability.UNAVAILABLE)) for key in self.enabled]
            return dashboard(width, height, states, self.updated, self.cfg.language, self.cfg.position, self.demo, self.cfg.theme, self.color, self.cfg.timezone)

    def key(self, key):
        if self.timezone_selecting:
            if key == b"\x1b":
                self.timezone_selecting = False
            elif key in (b"j", b"J", b"\x1b[B"):
                self.timezone_cursor = min(len(self.timezone_options) - 1, self.timezone_cursor + 1)
            elif key in (b"k", b"K", b"\x1b[A"):
                self.timezone_cursor = max(0, self.timezone_cursor - 1)
            elif key in (b"h", b"H", b"\x1b[D", b"l", b"L", b"\x1b[C") and self.timezone_cursor == len(self.timezone_options) - 1:
                step = -15 if key in (b"h", b"H", b"\x1b[D") else 15
                current = offset_minutes(self.timezone_options[-1])
                self.timezone_options[-1] = offset_setting(max(MINUTES_MIN, min(MINUTES_MAX, current + step)))
            elif key in (b"\r", b"\n"):
                self.cfg.timezone = self.timezone_options[self.timezone_cursor]
                config.save(self.cfg)
                self.timezone_selecting = False
            return False
        if self.selecting:
            if key in (b"\x1b",):
                self.selecting = False
            elif key in (b"\r", b"\n"):
                self.enabled = list(self.draft)
                config.save(self.cfg)
                self.selecting = False
                self.refresh()
            elif key in (b"j", b"J", b"\x1b[B"):
                self.cursor = min(len(REGISTRY)-1, self.cursor+1)
            elif key in (b"k", b"K", b"\x1b[A"):
                self.cursor = max(0, self.cursor-1)
            elif key == b" ":
                current = list(REGISTRY)[self.cursor]
                if current in self.draft:
                    self.draft.remove(current)
                else:
                    self.draft.append(current)
            elif key in (b"u", b"U", b"d", b"D"):
                current = list(REGISTRY)[self.cursor]
                if current in self.draft:
                    index = self.draft.index(current)
                    step = -1 if key.lower() == b"u" else 1
                    target = max(0, min(len(self.draft)-1, index+step))
                    self.draft[index], self.draft[target] = self.draft[target], self.draft[index]
            return False
        if key in (b"q", b"Q", b"\x1b", b"\x03"):
            return True
        if key in (b"l", b"L"):
            self.cfg.language = "zh" if self.cfg.language == "en" else "en"
            config.save(self.cfg)
        elif key in (b"t", b"T"):
            index = config.THEMES.index(self.cfg.theme)
            self.cfg.theme = config.THEMES[(index+1) % len(config.THEMES)]
            config.save(self.cfg)
        elif key in (b"p", b"P"):
            index = config.POSITIONS.index(self.cfg.position)
            self.cfg.position = config.POSITIONS[(index+1) % len(config.POSITIONS)]
            config.save(self.cfg)
        elif key in (b"s", b"S"):
            self.selecting = True
            self.draft = list(self.enabled)
            self.cursor = 0
        elif key in (b"z", b"Z"):
            custom = self.cfg.timezone if self.cfg.timezone not in PRESETS else "UTC+05:30"
            self.timezone_options = list(PRESETS) + [custom]
            self.timezone_cursor = self.timezone_options.index(self.cfg.timezone) if self.cfg.timezone in self.timezone_options else len(self.timezone_options) - 1
            self.timezone_selecting = True
        elif key in (b"r", b"R"):
            self.refresh()
        return False


def time_now():
    import datetime as dt
    return dt.datetime.now().astimezone()


def paint(lines, previous):
    height = shutil.get_terminal_size((80, 24)).lines
    parts = []
    for row in range(1, min(height, max(len(previous), len(lines)))+1):
        text = lines[row-1] if row <= len(lines) else ""
        old = previous[row-1] if row <= len(previous) else None
        if text != old:
            parts.append(f"\x1b[{row};1H\x1b[2K{text}")
    if parts:
        sys.stdout.write("".join(parts))
        sys.stdout.flush()
    return lines


def _args(argv=None):
    parser = argparse.ArgumentParser(
        prog="aiusage",
        description="Responsive terminal dashboard for verified AI CLI usage limits.",
    )
    parser.add_argument("--demo", action="store_true", help="use deterministic, isolated demo data")
    parser.add_argument("--snapshot", action="store_true", help="print one non-interactive dashboard snapshot")
    parser.add_argument("--size", metavar="WIDTHxHEIGHT", default="", help="snapshot dimensions (default: 80x24)")
    parser.add_argument("--version", action="version", version=f"AIUsage {__version__}")
    return parser.parse_args(argv)


def main(argv=None):
    args = _args(argv)
    color = not args.snapshot and "NO_COLOR" not in os.environ and os.environ.get("TERM") != "dumb"
    board = Dashboard(args.demo, color=color)
    if args.snapshot:
        board.refresh()
        try:
            width, height = (map(int, args.size.lower().split("x"))) if args.size else (80, 24)
        except (ValueError, TypeError):
            raise SystemExit("aiusage: --size must be WIDTHxHEIGHT")
        if width < 1 or height < 1:
            raise SystemExit("aiusage: --size dimensions must be positive")
        print("\n".join(board.frame(width, height)))
        return 0
    if not sys.stdin.isatty() or not sys.stdout.isatty():
        print("aiusage requires an interactive terminal", file=sys.stderr)
        return 2
    original = termios.tcgetattr(sys.stdin.fileno())
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
                if key == b"\x1b" and (board.selecting or board.timezone_selecting):
                    extra, _, _ = select.select([sys.stdin], [], [], 0.02)
                    if extra:
                        key += os.read(sys.stdin.fileno(), 2)
                if board.key(key):
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
