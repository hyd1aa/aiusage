import os
import datetime as dt
import re
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

from aiusage import config
from aiusage.cli import Dashboard
from aiusage.models import Availability, ProviderUsage, RateLimitWindow
from aiusage.providers import ProviderAdapter, REGISTRY
from aiusage.render import ANSI, column_count, dashboard, reset_text, system_text, visible_len
from aiusage.timezones import convert, offset_label, parse_timezone, valid_timezone


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

    def test_three_column_metrics_do_not_truncate_left_label(self):
        output = "\n".join(dashboard(80, 24, self.providers[:6], None, language="en"))
        self.assertNotIn("% lef ", output)
        self.assertIn("% left", output)

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
            self.assertTrue(reset_text(1788375056, "en").endswith("UTC-04"))
            os.environ["TZ"] = "Asia/Shanghai"
            time.tzset()
            self.assertTrue(reset_text(1788375056, "zh").endswith("UTC+08"))
        finally:
            if original is None:
                os.environ.pop("TZ", None)
            else:
                os.environ["TZ"] = original
            time.tzset()

    def test_explicit_timezone_conversion_crosses_dates(self):
        epoch = dt.datetime(2026, 9, 3, 18, 50, tzinfo=dt.timezone.utc).timestamp()
        self.assertEqual(reset_text(epoch, "en", "UTC"), "Sep 03 18:50 UTC")
        self.assertEqual(reset_text(epoch, "zh", "UTC+08"), "9月04日 02:50 UTC+08")
        self.assertEqual(reset_text(epoch, "en", "UTC+08"), "Sep 04 02:50 UTC+08")
        self.assertEqual(reset_text(epoch, "en", "UTC-04"), "Sep 03 14:50 UTC-04")

    def test_positive_zone_to_negative_zone_can_move_to_previous_date(self):
        epoch = dt.datetime(2026, 9, 4, 2, 0, tzinfo=dt.timezone.utc).timestamp()
        self.assertTrue(reset_text(epoch, "en", "UTC-05").startswith("Sep 03 21:00"))

    def test_half_hour_and_timezone_validation(self):
        epoch = dt.datetime(2026, 9, 3, 18, 50, tzinfo=dt.timezone.utc).timestamp()
        self.assertEqual(reset_text(epoch, "en", "UTC+05:30"), "Sep 04 00:20 UTC+05:30")
        for value in ("UTC-12", "UTC", "UTC+14", "UTC+05:45", "UTC+09:30"):
            self.assertTrue(valid_timezone(value))
            parse_timezone(value)
        for value in ("UTC+15", "UTC-13", "UTC+05:60", "GMT+08", "UTC+8", "Asia/Shanghai"):
            self.assertFalse(valid_timezone(value))

    @unittest.skipUnless(hasattr(time, "tzset"), "requires POSIX timezone support")
    def test_system_zone_is_live_and_dst_aware_but_label_is_numeric(self):
        original = os.environ.get("TZ")
        try:
            os.environ["TZ"] = "UTC"
            time.tzset()
            summer = dt.datetime(2026, 7, 1, 12, tzinfo=dt.timezone.utc)
            self.assertEqual(offset_label(convert(summer, "system")), "UTC")
            os.environ["TZ"] = "Etc/GMT-8"
            time.tzset()
            self.assertEqual(offset_label(convert(summer, "system")), "UTC+08")
            os.environ["TZ"] = "America/New_York"
            time.tzset()
            winter = dt.datetime(2026, 1, 1, 12, tzinfo=dt.timezone.utc)
            self.assertEqual(offset_label(convert(summer, "system")), "UTC-04")
            self.assertEqual(offset_label(convert(winter, "system")), "UTC-05")
            output = system_text("en", "system", summer)
            self.assertNotRegex(output, r"America/New_York|EDT|EST|CST")
        finally:
            if original is None:
                os.environ.pop("TZ", None)
            else:
                os.environ["TZ"] = original
            time.tzset()

    def test_dashboard_uses_one_display_timezone_for_system_and_reset(self):
        epoch = dt.datetime(2026, 9, 3, 18, 50, tzinfo=dt.timezone.utc).timestamp()
        provider = ProviderUsage("test", "Test", Availability.AVAILABLE, (RateLimitWindow("Day", 50, epoch),))
        output = "\n".join(dashboard(80, 24, [provider], None, language="zh", timezone="UTC+08"))
        self.assertIn("9月04日 02:50 UTC+08", output)
        self.assertIn("UTC+08", next(line for line in output.splitlines() if "系统时间" in line))
        self.assertNotRegex(output, r"Asia/Shanghai|America/New_York|\b(?:CST|EST|EDT)\b")

    def test_three_column_timezone_offset_is_not_truncated(self):
        epoch = dt.datetime(2026, 9, 3, 18, 50, tzinfo=dt.timezone.utc).timestamp()
        providers = [
            ProviderUsage(str(index), f"P{index}", Availability.AVAILABLE, (RateLimitWindow("Day", 50, epoch),))
            for index in range(6)
        ]
        output = "\n".join(dashboard(80, 24, providers, None, language="zh", demo=True, timezone="UTC+08"))
        self.assertEqual(output.count("9月04日 02:50 UTC+08"), 6)
        self.assertNotIn("UTC+ ", output)

    def test_white_and_green_themes_emit_distinct_styles(self):
        white = "\n".join(dashboard(80, 24, self.providers[:2], None, theme="white", color=True))
        green = "\n".join(dashboard(80, 24, self.providers[:2], None, theme="green", color=True))
        self.assertIn("\x1b[1;97m", white)
        self.assertIn("\x1b[1;92m", green)
        self.assertNotEqual(white, green)

    def test_themes_never_emit_background_color(self):
        outputs = [
            "\n".join(dashboard(80, 24, self.providers[:2], None, theme=theme, color=True))
            for theme in ("white", "green")
        ]
        for output in outputs:
            codes = re.findall(r"\x1b\[([0-9;]+)m", output)
            self.assertFalse(any(part.isdigit() and 40 <= int(part) <= 49 for code in codes for part in code.split(";")))
        self.assertEqual(ANSI.sub("", outputs[0]), ANSI.sub("", outputs[1]))

    def test_two_provider_box_height_is_content_driven(self):
        def box_height(terminal_height):
            lines = dashboard(80, terminal_height, self.providers[:2], None)
            top = next(index for index, line in enumerate(lines) if "┌" in line)
            bottom = next(index for index, line in enumerate(lines) if "└" in line)
            return bottom - top + 1
        self.assertEqual(box_height(24), box_height(40))

    def test_two_provider_vertical_spacing_is_compact(self):
        lines = dashboard(80, 24, self.providers[:2], None)
        visible = [line.strip(" │") for line in lines]
        first = next(index for index, line in enumerate(visible) if line.startswith("CODEX"))
        second = next(index for index, line in enumerate(visible) if line.startswith("GROK"))
        system = next(index for index, line in enumerate(visible) if line.startswith("System"))
        self.assertLessEqual(second - first, 4)
        self.assertLessEqual(system - second, 5)

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

    def test_timezone_selector_and_persistence(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            with mock.patch.object(config, "config_path", return_value=target):
                board = Dashboard(True, config.Config())
                board.key(b"Z")
                self.assertTrue(board.timezone_selecting)
                self.assertIn("时区", "\n".join(board.frame(80, 24)))
                board.timezone_cursor = board.timezone_options.index("UTC+08")
                board.key(b"\r")
            self.assertEqual(config.load(target).timezone, "UTC+08")

    def test_custom_timezone_selector_adjusts_by_quarter_hour(self):
        board = Dashboard(True, config.Config())
        board.key(b"Z")
        board.timezone_cursor = len(board.timezone_options) - 1
        self.assertEqual(board.timezone_options[-1], "UTC+05:30")
        board.key(b"\x1b[C")
        self.assertEqual(board.timezone_options[-1], "UTC+05:45")


if __name__ == "__main__":
    unittest.main()
