import datetime as dt
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from aiusage import config
from aiusage.cli import Dashboard
from aiusage.demo import demo_usage
from aiusage.models import Availability, ProviderUsage, RateLimitWindow
from aiusage.providers import ProviderAdapter, read_grok, remaining_from_used
from aiusage.render import dashboard, reset_text


OLD_START = "2026-08-29T15:14:41.162248+00:00"
OLD_END = "2026-09-05T15:14:41.162248+00:00"
NEW_START = "2026-09-05T15:14:41.162248+00:00"
NEW_END = "2026-09-12T15:14:41.162248+00:00"
OLD_RESET = dt.datetime(2026, 9, 5, 15, 14, 41, tzinfo=dt.timezone.utc).timestamp()
NEW_RESET = dt.datetime(2026, 9, 12, 15, 14, 41, tzinfo=dt.timezone.utc).timestamp()


def billing_record(ts, start, end, used=None):
    payload = {
        "currentPeriod": {"type": "USAGE_PERIOD_TYPE_WEEKLY", "start": start, "end": end},
        "billingPeriodStart": start,
        "billingPeriodEnd": end,
    }
    if used is not None:
        payload["creditUsagePercent"] = used
    return {
        "ts": ts,
        "msg": "billing: fetched credits config",
        "ctx": {"config": payload},
    }


def write_log(path, records):
    path.write_text("".join(json.dumps(record) + "\n" for record in records), encoding="utf-8")


class RemainingFromUsedTests(unittest.TestCase):
    def test_used_zero_is_remaining_one_hundred(self):
        self.assertEqual(remaining_from_used(0), 100)

    def test_used_one_hundred_is_remaining_zero(self):
        self.assertEqual(remaining_from_used(100), 0)

    def test_used_forty_seven_is_remaining_fifty_three(self):
        self.assertEqual(remaining_from_used(47), 53)

    def test_codex_mapping_shares_the_same_helper(self):
        self.assertEqual(remaining_from_used(0), 100)
        self.assertEqual(remaining_from_used(100), 0)
        self.assertEqual(remaining_from_used(47), 53)


class GrokLogParserTests(unittest.TestCase):
    def read(self, records):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "unified.jsonl"
            write_log(path, records)
            with mock.patch("aiusage.providers.GROK_LOG", str(path)):
                return read_grok()

    def test_omitted_used_percent_is_zero_used_and_full_remaining(self):
        windows = self.read([billing_record("2026-09-06T01:28:25.659Z", NEW_START, NEW_END)])
        self.assertEqual(windows[0].remaining_percent, 100)
        self.assertEqual(windows[0].label, "Week")

    def test_explicit_used_percent_zero(self):
        windows = self.read([billing_record("2026-09-06T01:28:25.659Z", NEW_START, NEW_END, used=0)])
        self.assertEqual(windows[0].remaining_percent, 100)

    def test_used_percent_one_hundred(self):
        windows = self.read([billing_record("2026-09-05T15:14:17.915Z", OLD_START, OLD_END, used=100)])
        self.assertEqual(windows[0].remaining_percent, 0)

    def test_used_percent_forty_seven(self):
        windows = self.read([billing_record("2026-09-04T12:00:00Z", OLD_START, OLD_END, used=47)])
        self.assertEqual(windows[0].remaining_percent, 53)

    def test_weekly_rollover_replaces_old_reset_and_percent(self):
        windows = self.read([
            billing_record("2026-09-05T15:14:17.915Z", OLD_START, OLD_END, used=100),
            billing_record("2026-09-05T15:14:47.920Z", NEW_START, NEW_END),
        ])
        self.assertEqual(len(windows), 1)
        self.assertEqual(windows[0].remaining_percent, 100)
        self.assertAlmostEqual(windows[0].reset_at, NEW_RESET, places=0)
        self.assertNotAlmostEqual(windows[0].reset_at, OLD_RESET, places=0)

    def test_old_reset_cannot_survive_successful_new_cycle_fetch(self):
        windows = self.read([
            billing_record("2026-09-05T15:13:47.919Z", OLD_START, OLD_END, used=100),
            billing_record("2026-09-05T15:14:17.915Z", OLD_START, OLD_END, used=100),
            billing_record("2026-09-06T01:20:13.130Z", NEW_START, NEW_END),
            billing_record("2026-09-06T01:28:25.659Z", NEW_START, NEW_END),
        ])
        self.assertEqual(windows[0].remaining_percent, 100)
        self.assertAlmostEqual(windows[0].reset_at, NEW_RESET, places=0)

    def test_malformed_used_percent_is_skipped_not_treated_as_zero(self):
        with self.assertRaises(RuntimeError):
            self.read([{
                "ts": "2026-09-06T01:28:25.659Z",
                "ctx": {"config": {
                    "creditUsagePercent": "full",
                    "billingPeriodEnd": NEW_END,
                }},
            }])


def grok_usage(remaining, reset, stale=False, error=None, availability=Availability.AVAILABLE):
    return ProviderUsage(
        "grok", "Grok", availability,
        (RateLimitWindow("Week", remaining, reset),) if availability == Availability.AVAILABLE else (),
        stale, error,
    )


def patch_reads(handlers):
    def fake(self):
        handler = handlers[self.key]
        return handler() if callable(handler) else handler
    return mock.patch.object(ProviderAdapter, "read", autospec=True, side_effect=fake)


class RefreshRetentionTests(unittest.TestCase):
    def board(self):
        return Dashboard(False, config.Config(real_providers=["grok"], auto_discover=False))

    def test_successful_refresh_clears_stale_and_replaces_values(self):
        board = self.board()
        board.states["grok"] = grok_usage(0, OLD_RESET, stale=True, error="previous error")
        with patch_reads({"grok": grok_usage(100, NEW_RESET)}):
            board.refresh()
        state = board.states["grok"]
        self.assertFalse(state.stale)
        self.assertEqual(state.windows[0].remaining_percent, 100)
        self.assertEqual(state.windows[0].reset_at, NEW_RESET)
        self.assertIsNone(state.error)

    def test_failed_refresh_preserves_old_value_and_marks_stale(self):
        board = self.board()
        board.states["grok"] = grok_usage(40, OLD_RESET)
        failed = ProviderUsage("grok", "Grok", Availability.UNAVAILABLE, error="timeout")
        with patch_reads({"grok": failed}):
            board.refresh()
        state = board.states["grok"]
        self.assertTrue(state.stale)
        self.assertEqual(state.windows[0].remaining_percent, 40)
        self.assertEqual(state.windows[0].reset_at, OLD_RESET)
        self.assertEqual(state.error, "timeout")

    def test_r_triggers_actual_grok_fetch(self):
        board = self.board()
        calls = []

        def reader():
            calls.append("grok")
            return grok_usage(100, NEW_RESET)

        with patch_reads({"grok": reader}):
            board.key(b"R")
        self.assertEqual(calls, ["grok"])
        self.assertEqual(board.states["grok"].windows[0].remaining_percent, 100)
        self.assertEqual(board.states["grok"].windows[0].reset_at, NEW_RESET)

    def test_periodic_refresh_triggers_actual_grok_fetch(self):
        board = self.board()
        calls = []

        def reader():
            calls.append("grok")
            return grok_usage(100, NEW_RESET)

        board.stop = mock.Mock()
        board.stop.wait.side_effect = [False, True]
        with patch_reads({"grok": reader}), mock.patch("aiusage.cli.REFRESH_SECONDS", 0):
            board.worker()
        self.assertGreaterEqual(len(calls), 2)

    def test_r_then_successful_new_cycle_replaces_old_window(self):
        board = self.board()
        board.states["grok"] = grok_usage(0, OLD_RESET, stale=True)
        with patch_reads({"grok": grok_usage(100, NEW_RESET)}):
            board.key(b"R")
        state = board.states["grok"]
        self.assertFalse(state.stale)
        self.assertEqual(state.windows[0].remaining_percent, 100)
        self.assertEqual(state.windows[0].reset_at, NEW_RESET)


class TimezoneAndIsolationTests(unittest.TestCase):
    def test_reset_formats_new_cycle_in_utc_plus_eight(self):
        self.assertEqual(reset_text(NEW_RESET, "zh", "UTC+08"), "9月12日 23:14 UTC+08")
        self.assertEqual(reset_text(OLD_RESET, "zh", "UTC+08"), "9月05日 23:14 UTC+08")

    def test_dashboard_shows_new_cycle_remaining_and_reset(self):
        provider = ProviderUsage("grok", "Grok", Availability.AVAILABLE, (RateLimitWindow("Week", 100, NEW_RESET),))
        output = "\n".join(dashboard(80, 24, [provider], None, language="zh", timezone="UTC+08"))
        self.assertIn("GROK", output)
        self.assertIn("100% 剩余", output)
        self.assertIn("9月12日 23:14 UTC+08", output)
        self.assertNotIn("9月05日", output)

    def test_codex_refresh_does_not_call_grok_reader(self):
        board = Dashboard(False, config.Config(real_providers=["codex"], auto_discover=False))
        calls = []

        def fake(self):
            calls.append(self.key)
            if self.key != "codex":
                raise AssertionError("grok reader called")
            return ProviderUsage("codex", "Codex", Availability.AVAILABLE, (RateLimitWindow("5h", 83, NEW_RESET),))

        with mock.patch.object(ProviderAdapter, "read", autospec=True, side_effect=fake):
            board.refresh()
        self.assertEqual(calls, ["codex"])
        self.assertEqual(board.states["codex"].windows[0].remaining_percent, 83)
        self.assertNotIn("grok", board.states)

    def test_demo_isolation_does_not_read_grok_log(self):
        board = Dashboard(True, config.Config(demo_providers=["grok"]))
        with mock.patch("aiusage.providers.read_grok", side_effect=AssertionError("real grok reader called")):
            board.refresh()
        self.assertEqual(board.states["grok"].availability, Availability.AVAILABLE)
        self.assertEqual(board.states["grok"].windows[0].remaining_percent, demo_usage("grok").windows[0].remaining_percent)


if __name__ == "__main__":
    unittest.main()
