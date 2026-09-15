use crate::{
    models::{remaining_from_used, Availability, ProviderUsage, RateLimitWindow},
    PROVIDERS,
};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub installed: bool,
    pub ready: bool,
    pub usage_supported: bool,
    pub usable: bool,
    pub reason: String,
}
pub fn classify_discovery(installed: bool, supported: bool, ready: bool) -> DiscoveryResult {
    let ready = installed && supported && ready;
    DiscoveryResult {
        installed,
        ready,
        usage_supported: supported,
        usable: ready,
        reason: if !installed {
            "not_installed"
        } else if !supported {
            "unsupported"
        } else if !ready {
            "needs_login"
        } else {
            "ready"
        }
        .into(),
    }
}
pub fn which(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .map(|p| p.join(name))
        .find(|path| {
            fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
}
pub fn installed(key: &str) -> bool {
    let commands: &[&str] = match key {
        "codex" => &["codex"],
        "grok" => &["grok"],
        "minimax" => &["mmx"],
        "qoder" => &["qoder"],
        "qodercn" => &["qodercn"],
        "codebuddy" => &["codebuddy", "cbc"],
        "traecode" => &["traecli"],
        _ => &[],
    };
    commands.iter().any(|command| which(command).is_some())
}
fn home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
}
pub fn grok_log() -> PathBuf {
    home().join(".grok/logs/unified.jsonl")
}
pub fn discover(key: &str) -> DiscoveryResult {
    let supported = matches!(key, "codex" | "grok");
    let present = installed(key);
    let ready = present
        && supported
        && match key {
            "codex" => {
                env::var_os("CODEX_API_KEY").is_some_and(|s| !s.is_empty())
                    || home().join(".codex/auth.json").is_file()
            }
            "grok" => grok_log().is_file() && File::open(grok_log()).is_ok(),
            _ => false,
        };
    classify_discovery(present, supported, ready)
}
pub fn discover_all() -> std::collections::HashMap<String, DiscoveryResult> {
    let (tx, rx) = mpsc::channel();
    let deadline = Instant::now() + Duration::from_secs(crate::DISCOVERY_TIMEOUT_SECONDS);
    for &(key, _) in PROVIDERS {
        let tx = tx.clone();
        thread::spawn(move || {
            let _ = tx.send((key.to_string(), discover(key)));
        });
    }
    drop(tx);
    let mut results = std::collections::HashMap::new();
    while let Ok((key, value)) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
    {
        results.insert(key, value);
    }
    for &(key, _) in PROVIDERS {
        results
            .entry(key.into())
            .or_insert_with(|| DiscoveryResult {
                installed: false,
                ready: false,
                usage_supported: matches!(key, "codex" | "grok"),
                usable: false,
                reason: "timeout".into(),
            });
    }
    results
}
pub fn window_label(minutes: Option<f64>, fallback: &str) -> String {
    match minutes {
        Some(1440.0) => "Daily".into(),
        Some(10080.0) => "Week".into(),
        Some(m) if m % 60.0 == 0.0 && m < 1440.0 => format!("{}h", (m / 60.0) as i64),
        _ => fallback.into(),
    }
}
fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_bool().map(|v| if v { 1.0 } else { 0.0 }))
}
pub fn timestamp(value: &Value) -> Option<f64> {
    if let Some(n) = number(value) {
        return Some(n);
    }
    let text = value.as_str()?.replace('Z', "+00:00");
    if let Ok(value) = DateTime::parse_from_rfc3339(&text) {
        return Some(value.timestamp() as f64 + value.timestamp_subsec_nanos() as f64 / 1e9);
    }
    for format in ["%Y-%m-%d %H:%M:%S%.f%:z", "%Y-%m-%dT%H:%M:%S%.f%:z"] {
        if let Ok(value) = DateTime::parse_from_str(&text, format) {
            return Some(value.timestamp() as f64 + value.timestamp_subsec_nanos() as f64 / 1e9);
        }
    }
    let naive = NaiveDateTime::parse_from_str(&text, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(&text, "%Y-%m-%d %H:%M:%S%.f"))
        .or_else(|_| {
            NaiveDate::parse_from_str(&text, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap())
        })
        .ok()?;
    let value = Local.from_local_datetime(&naive).earliest()?;
    Some(value.timestamp() as f64 + value.timestamp_subsec_nanos() as f64 / 1e9)
}
pub fn grok_window(config: &Value) -> Option<RateLimitWindow> {
    config.as_object()?;
    // Python uses `or`: epoch zero falls back to the billing-period field.
    let reset = timestamp(&config["currentPeriod"]["end"])
        .filter(|v| *v != 0.0)
        .or_else(|| timestamp(&config["billingPeriodEnd"]))?;
    let start = timestamp(&config["currentPeriod"]["start"])
        .filter(|v| *v != 0.0)
        .or_else(|| timestamp(&config["billingPeriodStart"]));
    let used = match config.get("creditUsagePercent") {
        Some(value) => number(value)?,
        None => 0.0,
    };
    let duration = start
        .map(|start| (reset - start) / 60.0)
        .filter(|m| *m != 0.0)
        .map(f64::round_ties_even);
    Some(RateLimitWindow {
        label: window_label(duration, "Cycle"),
        remaining_percent: remaining_from_used(used).ok()?,
        reset_at: Some(reset),
    })
}
pub fn read_grok(path: &Path) -> Result<Vec<RateLimitWindow>, String> {
    let mut file = File::open(path).map_err(|_| "Grok usage log unavailable")?;
    let size = file
        .seek(SeekFrom::End(0))
        .map_err(|_| "Grok usage log unavailable")?;
    const LIMIT: u64 = 4 * 1024 * 1024;
    file.seek(SeekFrom::Start(size.saturating_sub(LIMIT)))
        .map_err(|_| "Grok usage log unavailable")?;
    let mut reader = BufReader::new(file);
    let mut line = vec![];
    if size > LIMIT {
        reader
            .read_until(b'\n', &mut line)
            .map_err(|_| "Grok usage log unavailable")?;
    }
    let mut latest = None;
    loop {
        line.clear();
        if reader
            .read_until(b'\n', &mut line)
            .map_err(|_| "Grok usage log unavailable")?
            == 0
        {
            break;
        }
        if !line.windows(16).any(|w| w == b"billingPeriodEnd")
            && !line.windows(18).any(|w| w == b"creditUsagePercent")
        {
            continue;
        }
        if let Ok(value) = serde_json::from_slice::<Value>(&line) {
            if let Some(window) = grok_window(&value["ctx"]["config"]) {
                latest = Some(window);
            }
        }
    }
    latest
        .map(|w| vec![w])
        .ok_or("No reliable Grok billing snapshot".into())
}
pub fn codex_windows(reply: &Value) -> Result<Vec<RateLimitWindow>, String> {
    if reply.get("error").is_some() {
        return Err("Codex usage unavailable".into());
    }
    let snapshot = &reply["result"]["rateLimits"];
    let mut windows = vec![];
    for (key, fallback) in [("primary", "Primary"), ("secondary", "Secondary")] {
        let window = &snapshot[key];
        if !window.is_object() || window.get("usedPercent").is_none() {
            continue;
        }
        let used = number(&window["usedPercent"]).ok_or("Malformed rate-limit response")?;
        windows.push(RateLimitWindow {
            label: window_label(number(&window["windowDurationMins"]), fallback),
            remaining_percent: remaining_from_used(used).map_err(String::from)?,
            reset_at: number(&window["resetsAt"]),
        });
    }
    if windows.is_empty() {
        Err("No Codex rate-limit windows".into())
    } else {
        Ok(windows)
    }
}
pub fn read_codex(executable: &Path, timeout: Duration) -> Result<Vec<RateLimitWindow>, String> {
    let mut child = Command::new(executable)
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Codex usage unavailable")?;
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                if tx.send(value).is_err() {
                    break;
                }
            }
        }
    });
    let result = (|| -> Result<_, String> {
        let deadline = Instant::now() + timeout;
        let mut send = |value: Value| -> Result<(), String> {
            let input = child.stdin.as_mut().ok_or("Codex usage unavailable")?;
            writeln!(input, "{value}")
                .and_then(|_| input.flush())
                .map_err(|_| "Codex usage unavailable".into())
        };
        let response = |id: i32| -> Result<Value, String> {
            loop {
                let item = rx
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .map_err(|_| "Codex usage timeout")?;
                if item["id"] == id {
                    return Ok(item);
                }
                if Instant::now() >= deadline {
                    return Err("Codex usage timeout".into());
                }
            }
        };
        send(
            json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"aiusage","version":"2"}}}),
        )?;
        if response(1)?.get("error").is_some() {
            return Err("Codex initialization failed".into());
        }
        send(json!({"method":"initialized","params":{}}))?;
        send(json!({"id":2,"method":"account/rateLimits/read","params":{}}))?;
        codex_windows(&response(2)?)
    })();
    // Always reap our own helper, including malformed response and timeout.
    // This PID is exclusively owned by this invocation. Match Python's
    // graceful termination, bounded wait, then force termination as last resort.
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }
    let stop_deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Err(_) => break,
            _ => {}
        }
        if Instant::now() >= stop_deadline {
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    result
}
pub fn read(key: &str) -> ProviderUsage {
    let name = PROVIDERS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, name)| *name)
        .unwrap_or(key);
    let mut usage = ProviderUsage {
        key: key.into(),
        name: name.into(),
        availability: Availability::Unavailable,
        windows: vec![],
        stale: false,
        error: None,
    };
    if !installed(key) {
        usage.availability = Availability::NotInstalled;
        return usage;
    }
    let result = match key {
        "codex" => which("codex")
            .ok_or("Codex is not installed".into())
            .and_then(|exe| read_codex(&exe, Duration::from_secs(crate::CODEX_TIMEOUT_SECONDS))),
        "grok" => read_grok(&grok_log()),
        _ => {
            usage.availability = Availability::NotSupported;
            return usage;
        }
    };
    match result {
        Ok(windows) => {
            usage.availability = Availability::Available;
            usage.windows = windows;
        }
        Err(error) => usage.error = Some(error),
    };
    usage
}
