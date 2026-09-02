import errno
import fcntl
import os
import pty
import select
import signal
import struct
import sys
import termios
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@unittest.skipUnless(sys.platform.startswith("linux"), "Linux PTY test")
class TerminalCleanupTests(unittest.TestCase):
    def run_pty(self, action, code=None):
        pid, fd = pty.fork()
        if pid == 0:
            env = os.environ.copy()
            env["PYTHONPATH"] = str(ROOT / "src")
            argv = [sys.executable, "-m", "aiusage.cli", "--demo"]
            if code is not None:
                argv = [sys.executable, "-c", code]
            os.execve(sys.executable, argv, env)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        output = b""
        sent = False
        status = None
        exited = False
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            ready, _, _ = select.select([fd], [], [], 0.1)
            if ready:
                try:
                    output += os.read(fd, 65536)
                except OSError as exc:
                    if exc.errno != errno.EIO:
                        raise
            if not sent and b"AI USAGE" in output:
                if isinstance(action, int):
                    os.kill(pid, action)
                else:
                    os.write(fd, action)
                sent = True
            done, child_status = os.waitpid(pid, os.WNOHANG)
            if done:
                status = child_status
                exited = True
                break
        if not exited:
            os.kill(pid, signal.SIGKILL)
            os.waitpid(pid, 0)
            self.fail("PTY child did not exit")
        self.assertIn(b"\x1b[?25h", output)
        self.assertIn(b"\x1b[?1049l", output)
        self.assertIn(b"\x1b[0m", output)
        return os.waitstatus_to_exitcode(status), output

    def test_q_escape_and_ctrl_c_cleanup(self):
        for key in (b"q", b"\x1b", b"\x03"):
            status, _ = self.run_pty(key)
            self.assertEqual(status, 0)

    def test_sigterm_cleanup(self):
        status, _ = self.run_pty(signal.SIGTERM)
        self.assertEqual(status, 0)

    def test_terminal_input_flags_are_restored(self):
        code = (
            "import termios; from aiusage.cli import main; "
            "before=termios.tcgetattr(0); rc=main(['--demo']); "
            "after=termios.tcgetattr(0); "
            "print('TERMIOS_RESTORED', before == after); raise SystemExit(rc)"
        )
        status, output = self.run_pty(b"q", code)
        self.assertEqual(status, 0)
        self.assertIn(b"TERMIOS_RESTORED True", output)

    def test_renderer_exception_cleanup(self):
        code = (
            "from unittest.mock import patch; "
            "from aiusage.cli import Dashboard, main; "
            "p=patch.object(Dashboard,'frame',side_effect=RuntimeError('render')); "
            "p.start(); main(['--demo'])"
        )
        status, output = self.run_pty(b"q", code)
        self.assertNotEqual(status, 0)
        self.assertIn(b"RuntimeError", output)


if __name__ == "__main__":
    unittest.main()
