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
        self.assertEqual(list(REGISTRY), ["codex", "grok", "claude", "gemini", "deepseek", "kimi", "glm", "zai"])

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
        self.assertEqual(sum(line.count("┌") for line in lines), 6)
        self.assertLessEqual(len(lines), 24)
        self.assertLessEqual(max(map(len, lines)), 80)
        self.assertIn("[演示]", "\n".join(lines))

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
                board.cursor = list(REGISTRY).index("glm")
                board.key(b" ")
                board.key(b"\r")
                loaded = config.load(target)
            self.assertEqual(loaded.language, "zh")
            self.assertEqual(loaded.position, "bottom-left")
            self.assertIn("glm", loaded.demo_providers)

    def test_real_defaults_only_verified_adapters(self):
        self.assertEqual(config.Config().real_providers, ["codex", "grok"])
        self.assertIsNone(REGISTRY["claude"].reader)

    def test_demo_never_calls_real_adapter(self):
        board = Dashboard(True, config.Config())
        with mock.patch.object(ProviderAdapter, "read", side_effect=AssertionError("real adapter called")):
            board.refresh()
        self.assertEqual(board.states["codex"].availability, Availability.AVAILABLE)


if __name__ == "__main__":
    unittest.main()
