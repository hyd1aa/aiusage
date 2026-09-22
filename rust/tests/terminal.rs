use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    os::unix::fs::PermissionsExt,
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Pty {
    master: File,
    slave: Option<File>,
}

impl Pty {
    fn open() -> Self {
        let mut master = -1;
        let mut slave = -1;
        let mut size = libc::winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let size_pointer: *mut libc::winsize = &mut size;
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    size_pointer,
                )
            },
            0,
            "{}",
            io::Error::last_os_error()
        );
        Self {
            master: unsafe { File::from_raw_fd(master) },
            slave: Some(unsafe { File::from_raw_fd(slave) }),
        }
    }

    fn spawn(&mut self, command: &mut Command) -> Child {
        let slave = self.slave.take().unwrap();
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY as libc::c_ulong, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        command
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave))
            .spawn()
            .unwrap()
    }

    fn termios(&self) -> libc::termios {
        let mut value = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { libc::tcgetattr(self.master.as_raw_fd(), &mut value) },
            0
        );
        value
    }

    fn read_ready(&mut self, timeout: Duration, output: &mut Vec<u8>) {
        let mut descriptor = libc::pollfd {
            fd: self.master.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe {
            libc::poll(
                &mut descriptor,
                1,
                timeout.as_millis().min(i32::MAX as u128) as i32,
            )
        };
        if ready > 0 && descriptor.revents & libc::POLLIN != 0 {
            let mut buffer = [0; 65536];
            if let Ok(count) = self.master.read(&mut buffer) {
                output.extend_from_slice(&buffer[..count]);
            }
        }
    }
}

fn same_termios(left: &libc::termios, right: &libc::termios) -> bool {
    left.c_iflag == right.c_iflag
        && left.c_oflag == right.c_oflag
        && left.c_cflag == right.c_cflag
        && left.c_lflag == right.c_lflag
        && left.c_cc == right.c_cc
}

enum ExitAction {
    Input(&'static [u8]),
    Signal(i32),
}

#[test]
fn actual_pty_cleanup() {
    for (name, action) in [
        ("q", ExitAction::Input(b"q")),
        ("escape", ExitAction::Input(b"\x1b")),
        ("ctrl-c", ExitAction::Input(b"\x03")),
        ("sigterm", ExitAction::Signal(libc::SIGTERM)),
        ("settings", ExitAction::Input(b"LTPZ\rq")),
    ] {
        let home = tempfile::tempdir().unwrap();
        let mut pty = Pty::open();
        let before = pty.termios();
        let mut command = Command::new(env!("CARGO_BIN_EXE_aiusage"));
        command
            .arg("--demo")
            .env("HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path())
            .env("TERM", "xterm-256color")
            .env_remove("COLUMNS")
            .env_remove("LINES");
        let mut child = pty.spawn(&mut command);
        let mut output = vec![];
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut sent = false;
        let status = loop {
            pty.read_ready(Duration::from_millis(50), &mut output);
            if !sent && output.windows(b"AI USAGE".len()).any(|w| w == b"AI USAGE") {
                match action {
                    ExitAction::Input(bytes) => pty.master.write_all(bytes).unwrap(),
                    ExitAction::Signal(signal) => {
                        assert_eq!(unsafe { libc::kill(child.id() as i32, signal) }, 0)
                    }
                }
                sent = true;
            }
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                unsafe { libc::kill(child.id() as i32, libc::SIGKILL) };
                child.wait().unwrap();
                panic!("PTY child did not exit for {name}");
            }
        };
        for _ in 0..4 {
            pty.read_ready(Duration::from_millis(10), &mut output);
        }
        assert!(sent, "{name}");
        assert!(status.success(), "{name}: {status:?}");
        for marker in [b"\x1b[?25h".as_slice(), b"\x1b[?1049l", b"\x1b[0m"] {
            assert!(
                output.windows(marker.len()).any(|window| window == marker),
                "{name}: missing {marker:?}"
            );
        }
        assert!(same_termios(&pty.termios(), &before), "{name}");
    }
}

#[test]
fn real_mode_refresh_serializes_owned_fixture_readers() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("aiusage")).unwrap();
    fs::write(
        root.path().join("aiusage/config.toml"),
        "auto_discover=false\nreal_providers=[\"codex\"]\ndisabled_providers=[]\n",
    )
    .unwrap();
    let fake = root.path().join("codex");
    fs::write(
        &fake,
        r##"#!/bin/sh
printf '%s\n' start >> "$AIUSAGE_FIXTURE_LOG"
IFS= read -r init
printf '%s\n' '{"id":1,"result":{}}'
IFS= read -r initialized
IFS= read -r limits
/bin/sleep 0.6
printf '%s\n' finish >> "$AIUSAGE_FIXTURE_LOG"
printf '%s\n' '{"id":2,"result":{"rateLimits":{"primary":{"usedPercent":47,"windowDurationMins":300}}}}'
"##,
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
    let log = root.path().join("calls");

    let mut pty = Pty::open();
    let mut command = Command::new(env!("CARGO_BIN_EXE_aiusage"));
    command
        .env("HOME", root.path())
        .env("XDG_CONFIG_HOME", root.path())
        .env("PATH", root.path())
        .env("AIUSAGE_FIXTURE_LOG", &log)
        .env("TERM", "xterm")
        .env_remove("CODEX_API_KEY");
    let mut child = pty.spawn(&mut command);
    let mut output = vec![];
    let mut refreshed = false;
    let mut quit_sent = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        pty.read_ready(Duration::from_millis(30), &mut output);
        let calls = fs::read_to_string(&log).unwrap_or_default();
        if !calls.is_empty()
            && output.windows(b"AI USAGE".len()).any(|w| w == b"AI USAGE")
            && !refreshed
        {
            pty.master.write_all(b"r").unwrap();
            refreshed = true;
        }
        if calls.lines().filter(|line| *line == "finish").count() >= 2 && !quit_sent {
            pty.master.write_all(b"q").unwrap();
            quit_sent = true;
        }
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "refresh PTY child did not exit");
        thread::yield_now();
    };
    assert!(status.success());
    assert!(refreshed && quit_sent);
    assert_eq!(
        fs::read_to_string(log).unwrap().lines().collect::<Vec<_>>(),
        ["start", "finish", "start", "finish"]
    );
}
