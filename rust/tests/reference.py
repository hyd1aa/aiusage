"""Offline test oracle. Never imports or invokes a real provider reader."""
import dataclasses
import json
import pathlib
import sys
import tempfile

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "src"))
from aiusage import config, timezones
from aiusage.providers import remaining_from_used


def evaluate(case):
    operation = case["op"]
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
