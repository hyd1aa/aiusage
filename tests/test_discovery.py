import io
import tempfile
import time
import types
import unittest
from collections import OrderedDict
from pathlib import Path
from unittest import mock

from aiusage import config
from aiusage.cli import Dashboard
from aiusage.diagnostics import collect
from aiusage.manager import Manager
from aiusage.models import RateLimitWindow
from aiusage.providers import DiscoveryResult, ProviderAdapter, REGISTRY, bounded_discover


READY = DiscoveryResult(True, True, True, True, "ready")
NOT_INSTALLED = DiscoveryResult(False, False, True, False, "not_installed")
NEEDS_LOGIN = DiscoveryResult(True, False, True, False, "needs_login")


def result_for(key, value):
    return {key: value}


class DiscoveryContractTests(unittest.TestCase):
    def adapter(self, *, installed=True, ready=True, reader=True):
        usage_reader = (lambda: (RateLimitWindow("Day", 50),)) if reader else None
        return ProviderAdapter("test", "Test", usage_reader, lambda: installed, lambda: ready)

    def test_installed_but_not_ready(self):
        result = self.adapter(ready=False).discover()
        self.assertTrue(result.installed)
        self.assertFalse(result.usable)
        self.assertEqual(result.reason, "needs_login")

    def test_installed_but_unsupported(self):
        result = self.adapter(reader=False).discover()
        self.assertTrue(result.installed)
        self.assertFalse(result.usage_supported)
        self.assertEqual(result.reason, "unsupported")

    def test_discovery_timeout_is_bounded(self):
        adapter = ProviderAdapter("slow", "Slow", lambda: (), lambda: True, lambda: time.sleep(0.2) or True)
        started = time.monotonic()
        result = bounded_discover(adapter, timeout=0.01)
        self.assertEqual(result.reason, "timeout")
        self.assertLess(time.monotonic() - started, 0.1)

    def test_malformed_discovery_fails_closed(self):
        adapter = types.SimpleNamespace(reader=lambda: (), discover=lambda: {"usable": True})
        result = bounded_discover(adapter, timeout=0.1)
        self.assertFalse(result.usable)
        self.assertEqual(result.reason, "malformed")


class DashboardDiscoveryTests(unittest.TestCase):
    def test_auto_discover_defaults_true_and_legacy_config_uses_true(self):
        self.assertTrue(config.Config().auto_discover)
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_text('language = "en"\n')
            self.assertTrue(config.load(target).auto_discover)

    def test_legacy_manual_hide_is_migrated(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_text('real_providers = ["codex"]\n')
            loaded = config.load(target)
            self.assertEqual(loaded.real_providers, ["codex"])
            self.assertIn("grok", loaded.disabled_providers)

    def test_startup_discovery_precedes_usage_refresh(self):
        board = Dashboard(False, config.Config())
        board.stop = mock.Mock()
        board.stop.wait.return_value = True
        calls = []
        board.discover = lambda: calls.append("discover")
        board.refresh = lambda: calls.append("refresh")
        board.worker()
        self.assertEqual(calls, ["discover", "refresh"])

    def test_auto_discover_false_skips_scan(self):
        board = Dashboard(False, config.Config(auto_discover=False))
        with mock.patch("aiusage.cli.discover_all") as discover:
            self.assertEqual(board.discover(), [])
        discover.assert_not_called()

    def test_new_provider_is_appended_and_order_preserved(self):
        board = Dashboard(False, config.Config(real_providers=["codex", "grok"]))
        with mock.patch("aiusage.cli.discover_all", return_value=result_for("minimax", READY)), mock.patch("aiusage.cli.config.save"):
            self.assertEqual(board.discover(), ["MiniMax"])
        self.assertEqual(board.enabled, ["codex", "grok", "minimax"])
        self.assertIn("MiniMax", board.notice)
        self.assertIn("已发现新服务：MiniMax", "\n".join(board.frame(80, 24)))

    def test_disabled_by_user_is_not_reenabled(self):
        board = Dashboard(False, config.Config(real_providers=["codex", "grok"]))
        board.selecting = True
        board.draft = ["codex"]
        with mock.patch("aiusage.cli.config.save"), mock.patch.object(board, "refresh"):
            board.key(b"\r")
        self.assertIn("grok", board.cfg.disabled_providers)
        with mock.patch("aiusage.cli.discover_all", return_value=result_for("grok", READY)), mock.patch("aiusage.cli.config.save"):
            board.discover()
        self.assertNotIn("grok", board.enabled)

    def test_manual_reenable_clears_disabled_marker(self):
        cfg = config.Config(real_providers=["codex"], disabled_providers=["grok"])
        board = Dashboard(False, cfg)
        board.selecting = True
        board.draft = ["codex", "grok"]
        with mock.patch("aiusage.cli.config.save"), mock.patch.object(board, "refresh"):
            board.key(b"\r")
        self.assertEqual(board.enabled, ["codex", "grok"])
        self.assertNotIn("grok", board.cfg.disabled_providers)

    def test_r_runs_discovery_then_usage_refresh(self):
        board = Dashboard(False, config.Config())
        calls = []
        board.discover = lambda: calls.append("discover")
        board.refresh = lambda: calls.append("refresh")
        board.key(b"R")
        self.assertEqual(calls, ["discover", "refresh"])

    def test_periodic_discovery_runs_separately_from_refresh(self):
        board = Dashboard(False, config.Config())
        board.stop = mock.Mock()
        board.stop.wait.side_effect = [False, True]
        calls = []
        board.discover = lambda: calls.append("discover")
        board.refresh = lambda: calls.append("refresh")
        with mock.patch("aiusage.cli.DISCOVERY_SECONDS", 0):
            board.worker()
        self.assertEqual(calls, ["discover", "refresh", "discover", "refresh"])

    def test_cli_removed_or_session_lost_keeps_provider_order(self):
        board = Dashboard(False, config.Config(real_providers=["codex", "grok", "minimax"]))
        for state in (NOT_INSTALLED, NEEDS_LOGIN):
            with mock.patch("aiusage.cli.discover_all", return_value=result_for("minimax", state)):
                board.discover()
            self.assertEqual(board.enabled, ["codex", "grok", "minimax"])

    def test_timeout_preserves_previous_discovery_state(self):
        board = Dashboard(False, config.Config())
        board.discovery_states["codex"] = READY
        timeout = DiscoveryResult(False, False, True, False, "timeout")
        with mock.patch("aiusage.cli.discover_all", return_value=result_for("codex", timeout)):
            board.discover()
        self.assertEqual(board.discovery_states["codex"], READY)

    def test_demo_never_calls_discovery(self):
        board = Dashboard(True, config.Config())
        with mock.patch("aiusage.cli.discover_all") as discover:
            board.discover()
            board.refresh()
        discover.assert_not_called()

    def test_narrow_layout_survives_discovered_provider(self):
        board = Dashboard(False, config.Config(real_providers=["codex", "grok"]))
        with mock.patch("aiusage.cli.discover_all", return_value=result_for("minimax", READY)), mock.patch("aiusage.cli.config.save"):
            board.discover()
        lines = board.frame(32, 20)
        self.assertLessEqual(max(map(len, lines)), 32)


class DiscoveryManagementTests(unittest.TestCase):
    def test_manager_toggles_auto_discovery(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            choices = iter(("6", "0"))
            manager = Manager(config.Config(), input_fn=lambda _="": next(choices), output=io.StringIO())
            with mock.patch.object(config, "config_path", return_value=target):
                manager.settings()
            self.assertFalse(config.load(target).auto_discover)

    def test_diagnostics_reports_discovery_states_without_details(self):
        states = {key: DiscoveryResult(False, False, adapter.reader is not None, False, "not_installed") for key, adapter in REGISTRY.items()}
        states["codex"] = READY
        with mock.patch("aiusage.diagnostics.discover_all", return_value=states), mock.patch("aiusage.providers.ProviderAdapter.read") as read:
            read.return_value = mock.Mock(windows=(RateLimitWindow("Day", 50),))
            rows = collect(config.Config(), github_ok=False)
        rendered = "\n".join(f"{name}:{detail}" for name, _ok, detail in rows)
        self.assertIn("Codex:ready", rendered)
        self.assertIn("MiniMax:not_installed", rendered)
        self.assertNotRegex(rendered.lower(), r"token|cookie|authorization|password")


if __name__ == "__main__":
    unittest.main()
