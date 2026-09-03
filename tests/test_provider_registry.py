import tempfile
import unittest
from pathlib import Path
from unittest import mock

from aiusage import config
from aiusage.demo import demo_usage
from aiusage.models import Availability
from aiusage.providers import REGISTRY


class ProviderRegistryTests(unittest.TestCase):
    def test_registry_ids_are_unique(self):
        self.assertEqual(len(REGISTRY), len(set(REGISTRY)))

    def test_removed_google_providers_are_migrated_out(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "config.toml"
            target.write_text(
                'real_providers = ["gemini", "codex", "antigravity", "grok"]\n'
                'demo_providers = ["gemini", "minimax", "antigravity", "qoder"]\n'
                'disabled_providers = ["antigravity", "grok", "gemini"]\n'
            )
            loaded = config.load(target)
        self.assertEqual(loaded.real_providers, ["codex", "grok"])
        self.assertEqual(loaded.demo_providers, ["minimax", "qoder"])
        self.assertEqual(loaded.disabled_providers, [])

    def test_candidate_executable_detection(self):
        cases = {
            "minimax": {"mmx"},
            "qoder": {"qoder"},
            "qodercn": {"qodercn"},
            "codebuddy": {"codebuddy", "cbc"},
            "traecode": {"traecli"},
        }
        for key, executables in cases.items():
            with self.subTest(key=key), mock.patch("aiusage.providers.shutil.which", side_effect=lambda name: f"/bin/{name}" if name in executables else None):
                result = REGISTRY[key].discover()
                self.assertTrue(result.installed)
                self.assertFalse(result.usage_supported)
                self.assertEqual(result.reason, "unsupported")

    def test_codebuddy_accepts_both_official_aliases(self):
        for executable in ("codebuddy", "cbc"):
            with self.subTest(executable=executable), mock.patch(
                "aiusage.providers.shutil.which",
                side_effect=lambda name, wanted=executable: f"/bin/{name}" if name == wanted else None,
            ):
                self.assertTrue(REGISTRY["codebuddy"].discover().installed)

    def test_zcode_does_not_guess_a_desktop_binary(self):
        with mock.patch("aiusage.providers.shutil.which", return_value="/bin/zcode"):
            result = REGISTRY["zcode"].discover()
        self.assertFalse(result.installed)
        self.assertEqual(result.reason, "not_installed")

    def test_unsupported_candidate_never_generates_real_quota(self):
        with mock.patch("aiusage.providers.shutil.which", return_value="/bin/mmx"):
            state = REGISTRY["minimax"].read()
        self.assertEqual(state.availability, Availability.NOT_SUPPORTED)
        self.assertFalse(state.windows)

    def test_new_six_provider_demo_is_deterministic_and_local(self):
        expected = ["codex", "grok", "minimax", "qoder", "codebuddy", "traecode"]
        self.assertEqual(config.DEMO_DEFAULT, expected)
        first = [demo_usage(key).windows[0].remaining_percent for key in expected]
        second = [demo_usage(key).windows[0].remaining_percent for key in expected]
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
