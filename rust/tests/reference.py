"""Offline test oracle. Never imports or invokes a real provider reader."""
import dataclasses
import json
import pathlib
import sys
import tempfile
import datetime as dt
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "src"))
from aiusage import config, timezones
from aiusage.providers import remaining_from_used
from aiusage import render, demo
from aiusage.models import ProviderUsage, RateLimitWindow, Availability


def evaluate(case):
    operation = case["op"]
    if operation == "codex":
        import io
        from aiusage.providers import read_codex
        process = mock.Mock()
        process.stdin = io.StringIO()
        process.stdout = io.StringIO(json.dumps({"id":1,"result":{}}) + "\n" + json.dumps({"id":2, **case["reply"]}) + "\n")
        with mock.patch("aiusage.providers.shutil.which", return_value="owned-fixture"), mock.patch("aiusage.providers.subprocess.Popen", return_value=process), mock.patch("aiusage.providers.select.select", side_effect=lambda r,w,e,t: (r,[],[])):
            try:
                return [dataclasses.asdict(v) for v in read_codex()]
            except (ValueError, TypeError, RuntimeError, AttributeError):
                return None
    if operation == "versions":
        from aiusage.updater import version_tuple, is_newer
        return [version_tuple(case["value"]), is_newer(case["value"], "0.2.2")]
    if operation == "menu_utf8":
        import io, os
        from aiusage.manager import Manager
        raw = io.BytesIO()
        output = io.TextIOWrapper(raw, encoding="utf-8")
        menu = Manager(config.Config(language=case["language"]), output=output)
        menu.latest = None
        menu.color = False
        with mock.patch("aiusage.manager.shutil.get_terminal_size", return_value=os.terminal_size((case["width"],24))):
            menu.main_screen()
        output.flush()
        return raw.getvalue().decode("utf-8")
    if operation == "dimensions":
        try:
            result = list(map(int, case["value"].lower().split("x"))) if case["value"] else [80,24]
            if len(result) != 2:
                raise ValueError()
        except (ValueError, TypeError):
            return "aiusage: --size must be WIDTHxHEIGHT"
        return result if min(result) > 0 else "aiusage: --size dimensions must be positive"
    if operation == "args":
        import contextlib, io
        from aiusage.cli import _args
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                result = vars(_args(case["args"]))
                code = None
            except SystemExit as exception:
                result, code = None, exception.code
        return {"args": result, "code": code, "stdout": out.getvalue(), "stderr": err.getvalue()}
    if operation == "timestamp":
        from aiusage.providers import _timestamp
        try:
            return _timestamp(case["value"])
        except (ValueError, OverflowError, OSError):
            return None
    if operation == "epoch":
        try:
            return timezones.from_epoch(case["value"], case["zone"]).isoformat(timespec="microseconds")
        except (ValueError, OverflowError, OSError):
            return None
    if operation == "manager":
        import io
        from aiusage.manager import Manager
        from aiusage.updater import ReleaseInfo
        output = io.StringIO()
        menu = Manager(config.Config(**case["config"]), output=output)
        menu.color = case["color"]
        menu._unicode = lambda: case["unicode"]
        menu.latest = ReleaseInfo(**case["latest"]) if case["latest"] else None
        with mock.patch("aiusage.manager.shutil.get_terminal_size", return_value=__import__("os").terminal_size((case["width"],24))):
            menu.main_screen()
        return output.getvalue()
    if operation == "grok":
        from aiusage.providers import _grok_window
        result = _grok_window(case["config"])
        return dataclasses.asdict(result) if result else None
    if operation == "frame":
        cfg = config.Config(**case["config"])
        with mock.patch("aiusage.demo.time.time", return_value=case["now"]):
            providers = [demo.demo_usage(key) for key in cfg.demo_providers]
        for provider in case.get("providers", []):
            providers.append(ProviderUsage(
                provider["key"], provider["name"], Availability(provider["availability"]),
                tuple(RateLimitWindow(**w) for w in provider["windows"]),
                provider["stale"], provider["error"]))
        original_system = render.system_text
        frozen = dt.datetime.fromtimestamp(case["now"], dt.timezone.utc)
        updated = dt.datetime.fromtimestamp(case["updated"], dt.timezone.utc) if case["updated"] is not None else None
        with mock.patch("aiusage.render.system_text", side_effect=lambda language, timezone: original_system(language, timezone, frozen)):
            return render.dashboard(case["width"], case["height"], providers, updated,
                cfg.language, cfg.position, case["demo"], cfg.theme, case["color"], cfg.timezone, case["notice"])
    if operation == "selector":
        from aiusage.providers import REGISTRY
        cfg = config.Config(**case["config"])
        return render.selector(case["width"], case["height"], REGISTRY, cfg.demo_providers,
            case["cursor"], cfg.language, cfg.theme, case["color"])
    if operation == "zone_selector":
        cfg = config.Config(**case["config"])
        return render.timezone_selector(case["width"], case["height"], case["options"],
            case["cursor"], cfg.language, cfg.theme, case["color"])
    if operation == "config":
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "config.toml"
            path.write_text(case["text"], encoding="utf-8")
            cfg = config.load(path)
            config.save(cfg, path)
            return {"config": dataclasses.asdict(cfg), "saved": path.read_text()}
    if operation == "timezone":
        setting = case["setting"]
        if not timezones.valid_timezone(setting):
            return None
        value = timezones.from_epoch(case["epoch"], setting)
        return [value.strftime("%Y-%m-%d %H:%M:%S"), timezones.offset_label(value)]
    if operation == "remaining":
        return remaining_from_used(case["used"])
    raise AssertionError(operation)


print(json.dumps([evaluate(case) for case in json.load(sys.stdin)], ensure_ascii=False))
