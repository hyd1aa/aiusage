import io
import json
import os
import tempfile
import tarfile
import unittest
from pathlib import Path
from unittest import mock

from aiusage import __version__, config
from aiusage.manager import Manager
from aiusage.updater import ReleaseInfo, _parse, _safe_extract, _source_version, check_latest, is_newer


class TtyBuffer(io.StringIO):
    encoding = "utf-8"

    def isatty(self):
        return True


def inputs(*values):
    iterator = iter(values)
    return lambda _prompt="": next(iterator)


class ManagerDisplayTests(unittest.TestCase):
    def test_chinese_and_english_main_menus_use_shared_language(self):
        for language, expected in (("zh", "启动额度看板"), ("en", "Launch dashboard")):
            output = TtyBuffer()
            manager = Manager(config.Config(language=language), output=output)
            manager.main_screen()
            self.assertIn(expected, output.getvalue())
            self.assertIn(f"v{__version__}", output.getvalue())

    def test_newer_version_uses_foreground_highlight_only(self):
        output = TtyBuffer()
        manager = Manager(config.Config(), output=output)
        manager.color = True
        manager.latest = ReleaseInfo("99.0.0", "new", "", "https://github.com/hyd1aa/aiusage/archive/x.tar.gz")
        manager.main_screen()
        rendered = output.getvalue()
        self.assertIn("\x1b[1;92m", rendered)
        self.assertNotRegex(rendered, r"\x1b\[(?:4[0-9]|10[0-7])m")

    def test_narrow_terminal_does_not_overflow(self):
        output = TtyBuffer()
        manager = Manager(config.Config(), output=output)
        with mock.patch("aiusage.manager.shutil.get_terminal_size", return_value=os.terminal_size((24, 12))):
            manager.main_screen()
        plain = output.getvalue().replace("\x1b[37m", "").replace("\x1b[0m", "")
        self.assertTrue(all(len(line) <= 24 for line in plain.splitlines()))

    def test_ctrl_c_exits_cleanly(self):
        manager = Manager(config.Config(), input_fn=mock.Mock(side_effect=KeyboardInterrupt), output=io.StringIO())
        with mock.patch.object(manager, "_background_latest"):
            self.assertEqual(manager.run(), 0)


class VersionTests(unittest.TestCase):
    def response(self, payload):
        response = mock.MagicMock()
        response.__enter__.return_value = io.StringIO(json.dumps(payload))
        return response

    def test_latest_stable_release_and_comparison(self):
        payload = {"tag_name": "v0.1.3", "name": "AIUsage v0.1.3", "body": "short", "tarball_url": "https://api.github.com/repos/hyd1aa/aiusage/tarball/v0.1.3", "draft": False, "prerelease": False}
        with tempfile.TemporaryDirectory() as directory, mock.patch("aiusage.updater.cache_path", return_value=Path(directory) / "cache.json"):
            info = check_latest(opener=mock.Mock(return_value=self.response(payload)))
        self.assertEqual(info.version, "0.1.3")
        self.assertTrue(is_newer("0.1.3", "0.1.2"))

    def test_timeout_and_bad_response_fail_closed(self):
        with self.assertRaises(TimeoutError):
            check_latest(opener=mock.Mock(side_effect=TimeoutError))
        for payload in ({}, {"tag_name": "v9", "tarball_url": "https://evil.example/x", "prerelease": False}):
            with self.assertRaises(ValueError):
                _parse(payload)

    def test_release_archive_rejects_path_traversal(self):
        with tempfile.TemporaryDirectory() as directory:
            archive_path = Path(directory) / "bad.tar"
            with tarfile.open(archive_path, "w") as archive:
                member = tarfile.TarInfo("../outside")
                member.size = 0
                archive.addfile(member, io.BytesIO())
            with tarfile.open(archive_path) as archive, self.assertRaises(ValueError):
                _safe_extract(archive, Path(directory) / "target")

    def test_release_source_version_is_verified_before_install(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)
            package = source / "src" / "aiusage"
            package.mkdir(parents=True)
            (package / "__init__.py").write_text('__version__ = "1.2.3"\n')
            self.assertEqual(_source_version(source), "1.2.3")
            (package / "__init__.py").write_text('__version__ = "wrong"\n')
            with self.assertRaises(ValueError):
                _source_version(source)

    def test_update_menu_requires_confirmation(self):
        latest = ReleaseInfo("9.0.0", "AIUsage 9", "notes", "https://api.github.com/repos/hyd1aa/aiusage/tarball/v9.0.0")
        manager = Manager(config.Config(), input_fn=inputs("n"), output=io.StringIO())
        with mock.patch("aiusage.manager.check_latest", return_value=latest), mock.patch("aiusage.manager.install_release") as install:
            manager.update_menu()
        install.assert_not_called()

    def test_confirmed_update_uses_verified_release(self):
        latest = ReleaseInfo("9.0.0", "AIUsage 9", "notes", "https://api.github.com/repos/hyd1aa/aiusage/tarball/v9.0.0")
        manager = Manager(config.Config(), input_fn=inputs("y"), output=io.StringIO())
        with mock.patch("aiusage.manager.check_latest", return_value=latest), mock.patch("aiusage.manager.install_release", return_value=(True, "9.0.0")) as install:
            manager.update_menu()
        install.assert_called_once_with(latest, __version__)


class ManagerActionTests(unittest.TestCase):
    def test_ai_entrypoint_exists(self):
        self.assertIn('ai = "aiusage.manager:main"', Path("pyproject.toml").read_text())

    def test_dashboard_and_demo_return_to_manager(self):
        for choice, args in (("1", []), ("2", ["--demo"])):
            manager = Manager(config.Config(), input_fn=inputs(choice, "0"), output=io.StringIO())
            with mock.patch.object(manager, "_background_latest"), mock.patch("aiusage.manager.dashboard_main", return_value=0) as launch:
                self.assertEqual(manager.run(), 0)
            launch.assert_called_once_with(args)

    def test_theme_and_timezone_settings_reuse_config(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            manager = Manager(config.Config(), input_fn=inputs("2", "4", "3", "0"), output=io.StringIO())
            with mock.patch.object(config, "config_path", return_value=target):
                manager.settings()
            loaded = config.load(target)
            self.assertEqual(loaded.theme, "green")
            self.assertEqual(loaded.timezone, "UTC+08")

    def test_provider_save_and_cancel(self):
        manager = Manager(config.Config(), input_fn=inputs("3", "0"), output=io.StringIO())
        manager.provider_menu()
        self.assertEqual(manager.cfg.real_providers, ["codex", "grok"])
        with tempfile.TemporaryDirectory() as directory, mock.patch.object(config, "config_path", return_value=Path(directory) / "config.toml"):
            manager.input = inputs("3", "s")
            manager.provider_menu()
            self.assertIn("claude", manager.cfg.real_providers)

    def test_diagnostics_output_does_not_include_secrets(self):
        output = io.StringIO()
        manager = Manager(config.Config(), input_fn=inputs(""), output=output)
        rows = [("Codex", True, "installed"), ("Codex usage", True, "readable")]
        with mock.patch("aiusage.manager.check_latest"), mock.patch("aiusage.manager.collect", return_value=rows):
            manager.diagnostics()
        rendered = output.getvalue().lower()
        for forbidden in ("token", "cookie", "authorization", "password", "account id"):
            self.assertNotIn(forbidden, rendered)

    def test_uninstall_cancel_and_confirm_preserves_config(self):
        manager = Manager(config.Config(), input_fn=inputs("0"), output=io.StringIO())
        with mock.patch("aiusage.manager.subprocess.run") as run:
            self.assertFalse(manager.uninstall_menu())
            run.assert_not_called()
        manager.input = inputs("1", "y")
        with mock.patch("aiusage.manager.subprocess.run") as run:
            self.assertTrue(manager.uninstall_menu())
            run.assert_called_once()

    def test_uninstall_remove_config_only_after_confirmation(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "aiusage" / "config.toml"
            target.parent.mkdir(); target.write_text('language = "zh"\n')
            manager = Manager(config.Config(), input_fn=inputs("2", "yes"), output=io.StringIO())
            with mock.patch.object(config, "config_path", return_value=target), mock.patch("aiusage.manager.subprocess.run"):
                self.assertTrue(manager.uninstall_menu())
            self.assertFalse(target.parent.exists())


if __name__ == "__main__":
    unittest.main()
