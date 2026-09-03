import io
import os
import stat
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from aiusage.updater import ReleaseInfo, install_release


ROOT = Path(__file__).resolve().parents[1]


def mode(path):
    return stat.S_IMODE(path.stat().st_mode)


def metadata(path):
    value = path.stat()
    return stat.S_IMODE(value.st_mode), value.st_uid, value.st_gid


class InstallationPermissionTests(unittest.TestCase):
    def run_install(self, prefix):
        env = {**os.environ, "PREFIX": str(prefix)}
        return subprocess.run(
            [str(ROOT / "install.sh")], cwd=ROOT, env=env,
            text=True, capture_output=True,
        )

    def run_uninstall(self, prefix):
        return subprocess.run(
            [str(prefix / "lib" / "aiusage-uninstall.sh")],
            env={**os.environ, "PREFIX": str(prefix)},
            text=True, capture_output=True,
        )

    def prepared_prefix(self, root, bin_mode=0o775, lib_mode=0o2775):
        prefix = root / "custom-prefix"
        bindir = prefix / "bin"
        libdir = prefix / "lib"
        bindir.mkdir(parents=True)
        libdir.mkdir()
        bindir.chmod(bin_mode)
        libdir.chmod(lib_mode)
        return prefix, bindir, libdir

    def test_existing_bindir_mode_0775_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix, bindir, libdir = self.prepared_prefix(Path(directory))
            result = self.run_install(prefix)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(mode(bindir), 0o775)

    def test_existing_libdir_mode_02775_and_setgid_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix, bindir, libdir = self.prepared_prefix(Path(directory))
            result = self.run_install(prefix)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(mode(libdir), 0o2775)

    def test_restrictive_existing_directory_modes_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix, bindir, libdir = self.prepared_prefix(Path(directory), 0o700, 0o700)
            result = self.run_install(prefix)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((mode(bindir), mode(libdir)), (0o700, 0o700))

    def test_new_bindir_and_libdir_are_created_with_0755(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix = Path(directory) / "new-prefix"
            result = self.run_install(prefix)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(mode(prefix / "bin"), 0o755)
            self.assertEqual(mode(prefix / "lib"), 0o755)

    def test_custom_prefix_existing_modes_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix, bindir, libdir = self.prepared_prefix(Path(directory), 0o770, 0o2775)
            result = self.run_install(prefix)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((mode(bindir), mode(libdir)), (0o770, 0o2775))

    def test_reinstall_preserves_parent_permissions(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix, bindir, libdir = self.prepared_prefix(Path(directory))
            self.assertEqual(self.run_install(prefix).returncode, 0)
            before = (metadata(bindir), metadata(libdir))
            result = self.run_install(prefix)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((metadata(bindir), metadata(libdir)), before)

    def test_update_uses_fixed_installer_and_preserves_parent_permissions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            prefix, bindir, libdir = self.prepared_prefix(root)
            archive_bytes = io.BytesIO()
            with tarfile.open(fileobj=archive_bytes, mode="w:gz") as archive:
                for relative in ("install.sh", "uninstall.sh", "scripts", "src"):
                    archive.add(ROOT / relative, arcname=f"aiusage-0.2.1/{relative}")
            archive_bytes.seek(0)

            class Response(io.BytesIO):
                def __enter__(self): return self
                def __exit__(self, *_args): self.close()

            info = ReleaseInfo("0.2.1", "test", "", "https://api.github.com/repos/hyd1aa/aiusage/tarball/v0.2.1")
            before = (metadata(bindir), metadata(libdir))
            with mock.patch("aiusage.updater.urllib.request.urlopen", return_value=Response(archive_bytes.read())):
                updated, version = install_release(info, "0.1.0", prefix=str(prefix))
            self.assertTrue(updated)
            self.assertEqual(version, "0.2.1")
            self.assertEqual((metadata(bindir), metadata(libdir)), before)

    def test_uninstall_preserves_parents_and_third_party_ai(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix, bindir, libdir = self.prepared_prefix(Path(directory))
            foreign = bindir / "ai"
            foreign.write_text("#!/bin/sh\necho third-party\n")
            foreign.chmod(0o755)
            contents = foreign.read_bytes()
            self.assertEqual(self.run_install(prefix).returncode, 0)
            before = (metadata(bindir), metadata(libdir))
            result = self.run_uninstall(prefix)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((metadata(bindir), metadata(libdir)), before)
            self.assertEqual(foreign.read_bytes(), contents)
            self.assertFalse((bindir / "aiusage").exists())
            self.assertFalse((libdir / "aiusage").exists())

    def test_path_existing_as_regular_file_fails_safely(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix = Path(directory) / "prefix"
            prefix.mkdir()
            blocker = prefix / "bin"
            blocker.write_text("third-party data")
            result = self.run_install(prefix)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("exists but is not a directory", result.stderr)
            self.assertEqual(blocker.read_text(), "third-party data")
            self.assertFalse((prefix / "lib").exists())

    def test_third_party_reserved_package_or_uninstaller_is_not_overwritten(self):
        for relative in (Path("lib/aiusage"), Path("lib/aiusage-uninstall.sh")):
            with self.subTest(path=relative), tempfile.TemporaryDirectory() as directory:
                prefix = Path(directory) / "prefix"
                target = prefix / relative
                if target.suffix == ".sh":
                    target.parent.mkdir(parents=True)
                    target.write_text("third-party uninstaller")
                else:
                    target.mkdir(parents=True)
                    (target / "data").write_text("third-party package")
                result = self.run_install(prefix)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("not managed by AIUsage", result.stderr)
                self.assertTrue(target.exists())


if __name__ == "__main__":
    unittest.main()
