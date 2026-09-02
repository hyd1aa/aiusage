import os
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

from aiusage import config
from aiusage.cli import Dashboard
from aiusage.models import Availability, ProviderUsage, RateLimitWindow
from aiusage.providers import ProviderAdapter, REGISTRY
from aiusage.render import column_count, dashboard, reset_text, visible_len


class ProviderEdgeCaseTests(unittest.TestCase):
    def test_missing_codex_command_is_not_installed(self):
        adapter = ProviderAdapter("codex", "Codex", lambda: (), lambda: False)
        self.assertEqual(adapter.read().availability, Availability.NOT_INSTALLED)

    def test_missing_grok_command_is_not_installed(self):
        adapter = ProviderAdapter("grok", "Grok", lambda: (), lambda: False)
        self.assertEqual(adapter.read().availability, Availability.NOT_INSTALLED)

    def test_adapter_timeout_is_unavailable(self):
        def timeout():
            raise TimeoutError("bounded timeout")
        state = ProviderAdapter("test", "Test", timeout, lambda: True).read()
        self.assertEqual(state.availability, Availability.UNAVAILABLE)
        self.assertNotIn("token", state.error.lower())

    def test_malformed_adapter_output_is_unavailable(self):
        state = ProviderAdapter("test", "Test", lambda: ({"remaining": 50},), lambda: True).read()
        self.assertEqual(state.availability, Availability.UNAVAILABLE)

    def test_unknown_rate_window_renders(self):
        state = ProviderUsage("test", "Test", Availability.AVAILABLE, (RateLimitWindow("Unknown window", 50),))
        output = "\n".join(dashboard(40, 12, [state], None))
        self.assertIn("Unknow", output)

    def test_disabled_provider_is_not_called(self):
        board = Dashboard(False, config.Config(real_providers=["codex"]))
        with mock.patch.object(ProviderAdapter, "read", autospec=True) as reader:
            reader.return_value = ProviderUsage("codex", "Codex", Availability.UNAVAILABLE)
            board.refresh()
        self.assertEqual(reader.call_count, 1)
        self.assertEqual(reader.call_args.args[0].key, "codex")


class ConfigEdgeCaseTests(unittest.TestCase):
    def test_new_user_defaults_to_chinese(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(config.load(Path(directory) / "missing.toml").language, "zh")

    def test_explicit_english_preference_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_text('language = "en"\n')
            self.assertEqual(config.load(target).language, "en")

    def test_existing_theme_preference_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_text('language = "en"\ntheme = "green"\n')
            loaded = config.load(target)
            self.assertEqual(loaded.language, "en")
            self.assertEqual(loaded.theme, "green")

    def test_broken_config_falls_back(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_bytes(b"\xff\xfe\x00broken")
            self.assertEqual(config.load(target), config.Config())

    def test_malformed_values_fall_back(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_text('language = "xx"\nposition = "outer-space"\nreal_providers = ["bogus"]\n')
            loaded = config.load(target)
            self.assertEqual(loaded.language, "zh")
            self.assertEqual(loaded.position, "center")
            self.assertEqual(loaded.real_providers, ["codex", "grok"])

    def test_missing_config_directory_is_created_privately(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "missing" / "config.toml"
            self.assertTrue(config.save(config.Config(), target))
            self.assertEqual(target.stat().st_mode & 0o777, 0o600)

    def test_unwritable_or_missing_home_does_not_crash(self):
        with mock.patch.object(Path, "mkdir", side_effect=PermissionError("read only")):
            self.assertFalse(config.save(config.Config(), Path("/not-created/config.toml")))


class RenderingEdgeCaseTests(unittest.TestCase):
    def setUp(self):
        self.providers = [
            ProviderUsage(key, adapter.name, Availability.AVAILABLE, (RateLimitWindow("Window", 50),))
            for key, adapter in REGISTRY.items()
        ]

    def test_very_small_terminals_do_not_overflow(self):
        for width, height in ((1, 1), (10, 3), (20, 8), (24, 12)):
            lines = dashboard(width, height, self.providers[:6], None)
            self.assertLessEqual(len(lines), height)
            self.assertTrue(all(visible_len(line) <= width for line in lines))

    def test_resize_changes_layout_from_three_to_one_columns(self):
        self.assertEqual(column_count(6, 80), 3)
        self.assertEqual(column_count(6, 40), 1)

    def test_chinese_wide_characters_do_not_overflow(self):
        lines = dashboard(80, 24, self.providers[:6], None, language="zh", demo=True)
        self.assertTrue(all(visible_len(line) <= 80 for line in lines))
        self.assertIn("[演示]", "\n".join(lines))

    def test_single_outer_box_and_centered_title(self):
        lines = dashboard(80, 24, self.providers[:6], None, language="zh", demo=True)
        output = "\n".join(lines)
        self.assertEqual(output.count("┌"), 1)
        self.assertEqual(output.count("└"), 1)
        top = next(line for line in lines if "┌" in line)
        left, right = top.split("AI USAGE [演示]")
        self.assertLessEqual(abs(left.count("─") - right.count("─")), 1)

    def test_two_four_and_six_provider_layouts(self):
        self.assertEqual(column_count(2, 76), 1)
        self.assertEqual(column_count(4, 76), 2)
        self.assertEqual(column_count(6, 76), 3)
        for count in (2, 4, 6):
            output = "\n".join(dashboard(80, 24, self.providers[:count], None))
            self.assertEqual(output.count("┌"), 1)

    def test_reset_dates_are_localized_and_include_timezone(self):
        epoch = 1788375056
        english = reset_text(epoch, "en")
        chinese = reset_text(epoch, "zh")
        self.assertRegex(english, r"^[A-Z][a-z]{2} \d{2} \d{2}:\d{2} \S+$")
        self.assertRegex(chinese, r"^\d{1,2}月\d{2}日 \d{2}:\d{2} \S+$")
        self.assertNotIn("Sep", chinese)

    @unittest.skipUnless(hasattr(time, "tzset"), "requires POSIX timezone support")
    def test_reset_epoch_uses_selected_local_timezone(self):
        original = os.environ.get("TZ")
        try:
            os.environ["TZ"] = "America/New_York"
            time.tzset()
            self.assertTrue(reset_text(1788375056, "en").endswith("EDT"))
            os.environ["TZ"] = "Asia/Shanghai"
            time.tzset()
            self.assertTrue(reset_text(1788375056, "zh").endswith("CST"))
        finally:
            if original is None:
                os.environ.pop("TZ", None)
            else:
                os.environ["TZ"] = original
            time.tzset()

    def test_white_and_green_themes_emit_distinct_styles(self):
        white = "\n".join(dashboard(80, 24, self.providers[:2], None, theme="white", color=True))
        green = "\n".join(dashboard(80, 24, self.providers[:2], None, theme="green", color=True))
        self.assertIn("\x1b[1;97;40m", white)
        self.assertIn("\x1b[1;92;40m", green)
        self.assertNotEqual(white, green)

    def test_no_color_snapshot_contains_no_color_sgr(self):
        with mock.patch.dict(os.environ, {"NO_COLOR": "1"}):
            output = "\n".join(dashboard(80, 24, self.providers[:2], None))
        self.assertNotIn("\x1b[3", output)


class KeyTests(unittest.TestCase):
    def test_exit_keys(self):
        for key in (b"q", b"Q", b"\x1b", b"\x03"):
            self.assertTrue(Dashboard(True, config.Config()).key(key))

    def test_language_position_and_order_persist(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            with mock.patch.object(config, "config_path", return_value=target):
                board = Dashboard(True, config.Config(language="en"))
                board.key(b"L")
                board.key(b"P")
                board.key(b"S")
                board.cursor = list(REGISTRY).index("grok")
                board.key(b"D")
                board.key(b"\r")
            loaded = config.load(target)
            self.assertEqual(loaded.language, "zh")
            self.assertEqual(loaded.position, "bottom-left")
            self.assertNotEqual(loaded.demo_providers[:2], ["codex", "grok"])


if __name__ == "__main__":
    unittest.main()
