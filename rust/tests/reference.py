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
