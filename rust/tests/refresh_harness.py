"""Offline PTY integration: manual refreshes share the provider worker."""
import errno
import os
import pathlib
import pty
import select
import signal
import sys
import tempfile
import time

with tempfile.TemporaryDirectory() as directory:
    root = pathlib.Path(directory)
    (root / "aiusage").mkdir()
    (root / "aiusage/config.toml").write_text(
        'auto_discover=false\nreal_providers=["codex"]\ndisabled_providers=[]\n'
    )
    fake = root / "codex"
    fake.write_text(f"#!{sys.executable}\n" + '''import json, os, sys, time
def log(value):
    with open(os.environ["AIUSAGE_FIXTURE_LOG"], "a") as output:
        output.write(value + "\\n")
log("start")
sys.stdin.readline()
print('{"id":1,"result":{}}', flush=True)
sys.stdin.readline()
sys.stdin.readline()
time.sleep(0.6)
log("finish")
print(json.dumps({"id":2,"result":{"rateLimits":{"primary":{"usedPercent":47,"windowDurationMins":300}}}}), flush=True)
''')
    fake.chmod(0o700)
    log = root / "calls"
    pid, fd = pty.fork()
    if pid == 0:
        environment = {**os.environ, "HOME": directory, "XDG_CONFIG_HOME": directory,
                       "PATH": directory, "AIUSAGE_FIXTURE_LOG": str(log), "TERM": "xterm"}
        environment.pop("CODEX_API_KEY", None)
        os.execve(sys.argv[1], [sys.argv[1]], environment)
    done = False
    try:
        output = b""
        refreshed = False
        quit_sent = False
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if select.select([fd], [], [], 0.03)[0]:
                try:
                    output += os.read(fd, 65536)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise
            calls = log.read_text().splitlines() if log.exists() else []
            if calls and b"AI USAGE" in output and not refreshed:
                os.write(fd, b"r")
                refreshed = True
            if calls.count("finish") >= 2 and not quit_sent:
                os.write(fd, b"q")
                quit_sent = True
            child, status = os.waitpid(pid, os.WNOHANG)
            if child:
                done = True
                assert os.waitstatus_to_exitcode(status) == 0
                break
        assert done and refreshed and quit_sent
        assert log.read_text().splitlines() == ["start", "finish", "start", "finish"]
    finally:
        if not done:
            os.kill(pid, signal.SIGKILL)
            os.waitpid(pid, 0)
        os.close(fd)
