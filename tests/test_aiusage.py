import tempfile
import unittest
from pathlib import Path
from unittest import mock

from aiusage import config
from aiusage.cli import Dashboard
from aiusage.demo import demo_usage
from aiusage.models import Availability
from aiusage.providers import REGISTRY, ProviderAdapter
from aiusage.render import column_count


class AiUsageTests(unittest.TestCase):
    def test_registry_is_complete(self):
        self.assertEqual(list(REGISTRY), ["codex", "grok", "minimax", "qoder", "qodercn", "codebuddy", "traecode", "zcode"])

    def test_demo_is_local_and_available(self):
        for key in REGISTRY:
            state = demo_usage(key)
            self.assertEqual(state.availability, Availability.AVAILABLE)
            self.assertTrue(state.windows)

    def test_demo_80x24_is_three_by_two(self):
        board = Dashboard(True, config.Config())
        board.refresh()
        lines = board.frame(80, 24)
        self.assertEqual(column_count(6, 80), 3)
        self.assertEqual(sum(line.count("┌") for line in lines), 1)
        self.assertEqual(sum(line.count("└") for line in lines), 1)
        self.assertLessEqual(len(lines), 24)
        self.assertLessEqual(max(map(len, lines)), 80)
        rendered = "\n".join(lines)
        self.assertIn("[演示]", rendered)
        for name in ("CODEX", "GROK", "MINIMAX", "QODER", "CODEBUDDY", "TRAECODE"):
            self.assertIn(name, rendered)
        self.assertNotIn("GEMINI", rendered)
        self.assertNotIn("ANTIGRAVITY", rendered)

    def test_two_providers_preserve_single_column(self):
        self.assertEqual(column_count(2, 80), 1)

    def test_four_providers_are_balanced_two_by_two(self):
        self.assertEqual(column_count(4, 80), 2)

    def test_language_position_and_selector_persist(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            with mock.patch.object(config, "config_path", return_value=target):
                board = Dashboard(True, config.Config(language="en"))
                board.key(b"L")
                board.key(b"P")
                board.key(b"S")
                board.cursor = list(REGISTRY).index("qodercn")
                board.key(b" ")
                board.key(b"\r")
                loaded = config.load(target)
            self.assertEqual(loaded.language, "zh")
            self.assertEqual(loaded.position, "bottom-left")
            self.assertIn("qodercn", loaded.demo_providers)

    def test_white_theme_is_default(self):
        self.assertEqual(config.Config().theme, "white")

    def test_system_timezone_is_default(self):
        self.assertEqual(config.Config().timezone, "system")

    def test_legacy_config_without_timezone_defaults_to_system(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_text('language = "en"\ntheme = "green"\n')
            self.assertEqual(config.load(target).timezone, "system")

    def test_invalid_timezone_config_falls_back_to_system(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_text('timezone = "Asia/Shanghai"\n')
            self.assertEqual(config.load(target).timezone, "system")

    def test_theme_toggle_persists(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            with mock.patch.object(config, "config_path", return_value=target):
                board = Dashboard(True, config.Config())
                board.key(b"T")
            self.assertEqual(config.load(target).theme, "green")

    def test_real_defaults_only_verified_adapters(self):
        self.assertEqual(config.Config().real_providers, ["codex", "grok"])
        self.assertIsNone(REGISTRY["minimax"].reader)

    def test_demo_never_calls_real_adapter(self):
        board = Dashboard(True, config.Config())
        with mock.patch.object(ProviderAdapter, "read", side_effect=AssertionError("real adapter called")):
            board.refresh()
        self.assertEqual(board.states["codex"].availability, Availability.AVAILABLE)


if __name__ == "__main__":
    unittest.main()
