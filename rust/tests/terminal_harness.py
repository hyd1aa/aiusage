"""Run the reference PTY cleanup contract against the Rust binary.

The temporary home isolates saved preferences. No real adapter runs.
"""
import errno
import fcntl
import os
import pty
import select
import signal
import struct
import sys
import tempfile
import termios
import time

binary = sys.argv[1]
with tempfile.TemporaryDirectory() as directory:
    for action in (b"q", b"\x1b", b"\x03", signal.SIGTERM, b"LTPZ\rq"):
        ready_read, ready_write = os.pipe()
        pid, fd = pty.fork()
        if pid == 0:
            os.close(ready_write)
            os.read(ready_read, 1)
            os.close(ready_read)
            env = {**os.environ, "HOME": directory, "XDG_CONFIG_HOME": directory, "TERM": "xterm-256color"}
            env.pop("COLUMNS", None)
            env.pop("LINES", None)
            os.execve(binary, [binary, "--demo"], env)
        try:
            os.close(ready_read)
            fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
            before = termios.tcgetattr(fd)
            # Capture the original settings before the child can enter cbreak.
            os.write(ready_write, b"1")
            os.close(ready_write)
            output = b""
            sent = False
            finished = False
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                if select.select([fd], [], [], 0.05)[0]:
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
                done, status = os.waitpid(pid, os.WNOHANG)
                if done:
                    finished = True
                    break
            if not finished:
                os.kill(pid, signal.SIGKILL)
                os.waitpid(pid, 0)
                raise AssertionError(f"Rust PTY child did not exit: {action!r}")
            # Drain cleanup bytes written immediately before child exit.
            while select.select([fd], [], [], 0)[0]:
                try:
                    chunk = os.read(fd, 65536)
                    if not chunk:
                        break
                    output += chunk
                except OSError:
                    break
            assert os.waitstatus_to_exitcode(status) == 0, action
            for marker in (b"\x1b[?25h", b"\x1b[?1049l", b"\x1b[0m"):
                assert marker in output, (action, marker)
            assert termios.tcgetattr(fd) == before, action
        finally:
            os.close(fd)
print("5 Rust PTY exit/cleanup scenarios passed")
