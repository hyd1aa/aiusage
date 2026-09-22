use crate::{config, dashboard::Dashboard, providers};
use std::{
    env,
    io::{self, IsTerminal, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const USAGE:&str="usage: aiusage [-h] [--demo] [--menu] [--snapshot] [--size WIDTHxHEIGHT]\n               [--version]";
pub const HELP:&str=concat!("usage: aiusage [-h] [--demo] [--menu] [--snapshot] [--size WIDTHxHEIGHT]\n               [--version]\n\n",
    "Responsive terminal dashboard for verified AI CLI usage limits.\n\noptions:\n",
    "  -h, --help           show this help message and exit\n",
    "  --demo               use deterministic, isolated demo data\n",
    "  --menu               open the AIUsage management menu\n",
    "  --snapshot           print one non-interactive dashboard snapshot\n",
    "  --size WIDTHxHEIGHT  snapshot dimensions (default: 80x24)\n",
    "  --version            show program's version number and exit\n");

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    pub demo: bool,
    pub menu: bool,
    pub snapshot: bool,
    pub size: String,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Parsed {
    Args(Args),
    Help,
    Version,
}
pub fn parse(args: &[String]) -> Result<Parsed, String> {
    // argparse classifies every option before executing help/version actions.
    for token in args.iter().take_while(|s| s.as_str() != "--") {
        let name = token.split('=').next().unwrap_or(token);
        let candidates: Vec<_> = [
            "--help",
            "--demo",
            "--menu",
            "--snapshot",
            "--size",
            "--version",
        ]
        .into_iter()
        .filter(|flag| name.starts_with("--") && flag.starts_with(name))
        .collect();
        if candidates.len() > 1 {
            return Err(format!(
                "ambiguous option: {name} could match {}",
                candidates.join(", ")
            ));
        }
    }
    let mut parsed = Args::default();
    let mut index = 0;
    let mut unknown = vec![];
    while index < args.len() {
        let token = &args[index];
        if token == "--" {
            unknown.extend(args[index..].iter().cloned());
            break;
        }
        if let Some(tail) = token.strip_prefix("-h").filter(|s| !s.is_empty()) {
            if tail.chars().all(|c| c == 'h') {
                return Ok(Parsed::Help);
            }
            return Err(format!(
                "argument -h/--help: ignored explicit argument '{}'",
                tail.strip_prefix('=')
                    .unwrap_or_else(|| tail.trim_start_matches('h'))
            ));
        }
        let (arg, assigned) = token
            .split_once('=')
            .map(|(a, b)| (a, Some(b)))
            .unwrap_or((token.as_str(), None));
        let candidates: Vec<_> = [
            "--help",
            "--demo",
            "--menu",
            "--snapshot",
            "--size",
            "--version",
        ]
        .into_iter()
        .filter(|flag| flag.starts_with(arg) && arg.starts_with("--"))
        .collect();
        if candidates.len() > 1 {
            return Err(format!(
                "ambiguous option: {arg} could match {}",
                candidates.join(", ")
            ));
        }
        let arg = if arg == "-h" {
            "--help"
        } else {
            candidates.first().copied().unwrap_or(arg)
        };
        if assigned.is_some()
            && arg != "--size"
            && matches!(
                arg,
                "--help" | "--demo" | "--menu" | "--snapshot" | "--version"
            )
        {
            return Err(format!(
                "argument {}: ignored explicit argument '{}'",
                if arg == "--help" { "-h/--help" } else { arg },
                assigned.unwrap()
            ));
        }
        match arg {
            "--help" => return Ok(Parsed::Help),
            "--version" => return Ok(Parsed::Version),
            "--demo" => parsed.demo = true,
            "--menu" => parsed.menu = true,
            "--snapshot" => parsed.snapshot = true,
            "--size" => {
                let value = if let Some(value) = assigned {
                    value.to_string()
                } else {
                    index += 1;
                    args.get(index)
                        .filter(|s| {
                            !s.starts_with('-')
                                || s.as_str() == "-"
                                || s.contains(' ')
                                || negative_number(s)
                        })
                        .ok_or("argument --size: expected one argument")?
                        .clone()
                };
                parsed.size = value;
            }
            _ => unknown.push(token.clone()),
        }
        index += 1;
    }
    if !unknown.is_empty() {
        return Err(format!("unrecognized arguments: {}", unknown.join(" ")));
    }
    Ok(Parsed::Args(parsed))
}
// argparse accepts negative decimal arguments, but not arbitrary option-like text.
fn negative_number(value: &str) -> bool {
    let Some(value) = value.strip_prefix('-') else {
        return false;
    };
    if let Some((whole, fraction)) = value.split_once('.') {
        whole
            .chars()
            .all(|c| crate::timezones::decimal_digit(c).is_some())
            && !fraction.is_empty()
            && fraction
                .chars()
                .all(|c| crate::timezones::decimal_digit(c).is_some())
    } else {
        !value.is_empty()
            && value
                .chars()
                .all(|c| crate::timezones::decimal_digit(c).is_some())
    }
}
pub fn dimensions(size: &str) -> Result<(usize, usize), &'static str> {
    if size.is_empty() {
        return Ok((80, 24));
    }
    let lower = size.to_lowercase();
    let parts: Vec<_> = lower.split('x').collect();
    if parts.len() != 2 {
        return Err("aiusage: --size must be WIDTHxHEIGHT");
    }
    let values: Option<Vec<i64>> = parts.iter().map(|s| decimal_integer(s)).collect();
    let values = values.ok_or("aiusage: --size must be WIDTHxHEIGHT")?;
    if values.iter().any(|v| *v < 1) {
        return Err("aiusage: --size dimensions must be positive");
    }
    Ok((values[0] as usize, values[1] as usize))
}
pub fn decimal_integer(value: &str) -> Option<i64> {
    let text = value.trim();
    let (sign, digits) = if let Some(s) = text.strip_prefix('-') {
        ("-", s)
    } else {
        ("", text.strip_prefix('+').unwrap_or(text))
    };
    let mut normalized = sign.to_string();
    let mut digit_before = false;
    for c in digits.chars() {
        if c == '_' && digit_before {
            digit_before = false;
            continue;
        }
        normalized.push(char::from_digit(crate::timezones::decimal_digit(c)?, 10)?);
        digit_before = true;
    }
    if !digit_before {
        return None;
    }
    normalized.parse().ok()
}
pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
pub fn terminal_size() -> (usize, usize) {
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    unsafe {
        libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size);
    }
    let w = env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(if size.ws_col > 0 {
            size.ws_col as usize
        } else {
            80
        });
    let h = env::var("LINES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(if size.ws_row > 0 {
            size.ws_row as usize
        } else {
            24
        });
    (w, h)
}
fn discover(board: &mut Dashboard, monotonic: f64) {
    if !board.demo
        && board.cfg.auto_discover
        && board.apply_discovery(providers::discover_all(), monotonic)
    {
        config::save(&board.cfg, &config::config_path());
    }
}
pub fn refresh(board: &mut Dashboard, with_discovery: bool, monotonic: f64) {
    if with_discovery {
        discover(board, monotonic);
    }
    board.refresh_with(now(), providers::read);
}
pub fn main(args: Vec<String>) -> i32 {
    let parsed = match parse(&args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("{USAGE}\naiusage: error: {error}");
            return 2;
        }
    };
    let args = match parsed {
        Parsed::Help => {
            print!("{HELP}");
            return 0;
        }
        Parsed::Version => {
            println!("AIUsage {}", crate::VERSION);
            return 0;
        }
        Parsed::Args(args) => args,
    };
    if args.menu {
        return crate::manager::main();
    }
    let color = !args.snapshot
        && env::var_os("NO_COLOR").is_none()
        && env::var("TERM").unwrap_or_default() != "dumb";
    let mut board = Dashboard::new(args.demo, config::load(&config::config_path()), color);
    if args.snapshot {
        refresh(&mut board, true, 0.0);
        let (width, height) = match dimensions(&args.size) {
            Ok(size) => size,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        };
        println!("{}", board.frame(width, height, now(), 0.0).join("\n"));
        return 0;
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        eprintln!("aiusage requires an interactive terminal");
        return 2;
    }
    match interactive(board) {
        Ok(()) => 0,
        Err(_) => {
            eprintln!("aiusage: terminal unavailable");
            1
        }
    }
}

static QUITTING: AtomicBool = AtomicBool::new(false);
extern "C" fn exit_signal(_: libc::c_int) {
    QUITTING.store(true, Ordering::Relaxed);
}
struct Terminal {
    original: libc::termios,
    signals: Vec<(libc::c_int, libc::sighandler_t)>,
}
impl Terminal {
    fn enter() -> io::Result<Self> {
        let mut original = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(0, &mut original) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut terminal = Self {
            original,
            signals: vec![],
        };
        QUITTING.store(false, Ordering::Relaxed);
        for sig in [libc::SIGINT, libc::SIGTERM] {
            let old = unsafe { libc::signal(sig, exit_signal as libc::sighandler_t) };
            terminal.signals.push((sig, old));
        }
        let mut raw = original;
        raw.c_lflag &= !(libc::ECHO | libc::ICANON);
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(0, libc::TCSADRAIN, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        print!("\x1b[?1049h\x1b[?25l\x1b[?7l");
        io::stdout().flush()?;
        Ok(terminal)
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(0, libc::TCSADRAIN, &self.original);
        }
        let _ = io::stdout().write_all(b"\x1b[?7h\x1b[?25h\x1b[?1049l\x1b[0m");
        let _ = io::stdout().flush();
        for &(sig, old) in &self.signals {
            unsafe {
                libc::signal(sig, old);
            }
        }
    }
}
fn readable(milliseconds: i32) -> bool {
    let mut fd = libc::pollfd {
        fd: 0,
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe { libc::poll(&mut fd, 1, milliseconds) > 0 }
}
fn read_terminal(buffer: &mut [u8]) -> io::Result<usize> {
    // Do not use buffered Stdin with poll: it may prefetch later keystrokes,
    // leaving the OS descriptor unreadable while Rust still holds input.
    let count = unsafe { libc::read(0, buffer.as_mut_ptr().cast(), buffer.len()) };
    if count < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(count as usize)
    }
}
pub fn paint(lines: &[String], previous: &[String], height: usize) -> String {
    let mut output = String::new();
    for index in 0..height.min(lines.len().max(previous.len())) {
        let line = lines.get(index).map(String::as_str).unwrap_or("");
        if previous.get(index).map(String::as_str) != Some(line) {
            output.push_str(&format!("\x1b[{};1H\x1b[2K{line}", index + 1));
        }
    }
    output
}
pub fn interactive(board: Dashboard) -> io::Result<()> {
    let _terminal = Terminal::enter()?;
    let board = Arc::new(Mutex::new(board));
    let stop = Arc::new(AtomicBool::new(false));
    let origin = Instant::now();
    let worker_board = board.clone();
    let worker_stop = stop.clone();
    // All usage reads share one worker. Key-triggered refreshes cannot race a
    // periodic read and overwrite a newer quota with an older response.
    let (requests, receiver) = std::sync::mpsc::channel::<Option<bool>>();
    let worker = thread::spawn(move || {
        let mut cadence = crate::cadence::Cadence::default();
        let mut manual = None;
        loop {
            if worker_stop.load(Ordering::Relaxed) {
                return;
            }
            let mut local = worker_board.lock().unwrap().clone();
            let discover_now =
                manual.unwrap_or_else(|| cadence.discovery_due(origin.elapsed().as_secs_f64()));
            let discovered = discover_now && !local.demo && local.cfg.auto_discover;
            let mut discovery = None;
            if discovered {
                let results = providers::discover_all();
                local.apply_discovery(results.clone(), origin.elapsed().as_secs_f64());
                discovery = Some(results);
            }
            local.refresh_with(now(), |key| providers::read_cancellable(key, &worker_stop));
            if worker_stop.load(Ordering::Relaxed) {
                return;
            }
            {
                let mut board = worker_board.lock().unwrap();
                // Never replace concurrent user edits with the worker's old config.
                let enabled = local.enabled().to_vec();
                if let Some(results) = discovery {
                    if board.apply_discovery(results, origin.elapsed().as_secs_f64()) {
                        config::save(&board.cfg, &config::config_path());
                    }
                }
                board.apply_refresh(
                    local
                        .states
                        .into_values()
                        .filter(|s| enabled.contains(&s.key))
                        .collect(),
                    now(),
                );
            }
            // Manual R does not reset the independent periodic deadlines.
            if manual.is_none() {
                cadence.completed(origin.elapsed().as_secs_f64(), discover_now);
            }
            let remaining = (cadence.next_refresh - origin.elapsed().as_secs_f64()).max(0.0);
            match receiver.recv_timeout(Duration::from_secs_f64(remaining)) {
                Ok(Some(discover)) => manual = Some(discover),
                Ok(None) => return,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => manual = None,
                Err(_) => return,
            }
        }
    });
    let result = (|| -> io::Result<()> {
        let mut previous = vec![];
        while !QUITTING.load(Ordering::Relaxed) {
            let (width, height) = terminal_size();
            let lines =
                board
                    .lock()
                    .unwrap()
                    .frame(width, height, now(), origin.elapsed().as_secs_f64());
            io::stdout().write_all(paint(&lines, &previous, height).as_bytes())?;
            io::stdout().flush()?;
            previous = lines;
            if readable(200) {
                let mut byte = [0];
                if read_terminal(&mut byte)? == 0 {
                    break;
                }
                let mut key = byte.to_vec();
                let mut board = board.lock().unwrap();
                if byte[0] == 27 && (board.selecting || board.timezone_selecting) && readable(20) {
                    let mut extra = [0; 2];
                    let count = read_terminal(&mut extra)?;
                    key.extend_from_slice(&extra[..count]);
                }
                let effects = board.key(&key);
                if effects.save {
                    config::save(&board.cfg, &config::config_path());
                }
                if effects.quit {
                    break;
                }
                if effects.refresh {
                    let _ = requests.send(Some(effects.discover));
                }
            }
        }
        Ok(())
    })();
    stop.store(true, Ordering::Relaxed);
    let _ = requests.send(None);
    let _ = worker.join();
    result
}
