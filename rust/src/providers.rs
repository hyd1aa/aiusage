use crate::{
    models::{remaining_from_used, Availability, ProviderUsage, RateLimitWindow},
    PROVIDERS,
};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    os::unix::ffi::OsStrExt,
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
            fs::metadata(path).is_ok_and(|m| m.is_file())
                && std::ffi::CString::new(path.as_os_str().as_bytes())
                    .is_ok_and(|value| unsafe { libc::access(value.as_ptr(), libc::X_OK) == 0 })
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
        "gemini_cli" => &["agy", "gemini"],
        _ => &[],
    };
    commands.iter().any(|command| which(command).is_some())
}
fn home() -> PathBuf {
    crate::config::home_dir()
}
pub fn grok_log() -> PathBuf {
    home().join(".grok/logs/unified.jsonl")
}
const DEFAULT_GEMINI_CLIENT_ID_BYTES: &[u8] = &[
    49, 48, 55, 49, 48, 48, 54, 48, 54, 48, 53, 57, 49, 45, 116, 109, 104, 115, 115, 105, 110,
    50, 104, 50, 49, 108, 99, 114, 101, 50, 51, 53, 118, 116, 111, 108, 111, 106, 104, 52, 103,
    52, 48, 51, 101, 112, 46, 97, 112, 112, 115, 46, 103, 111, 111, 103, 108, 101, 117, 115,
    101, 114, 99, 111, 110, 116, 101, 110, 116, 46, 99, 111, 109,
];
const DEFAULT_GEMINI_CLIENT_SECRET_BYTES: &[u8] = &[
    71, 79, 67, 83, 80, 88, 45, 75, 53, 56, 70, 87, 82, 52, 56, 54, 76, 100, 76, 74, 49, 109,
    76, 66, 56, 115, 88, 67, 52, 122, 54, 113, 68, 65, 102,
];

pub fn default_gemini_client_id() -> String {
    String::from_utf8_lossy(DEFAULT_GEMINI_CLIENT_ID_BYTES).into_owned()
}

pub fn default_gemini_client_secret() -> String {
    String::from_utf8_lossy(DEFAULT_GEMINI_CLIENT_SECRET_BYTES).into_owned()
}

pub const GOOGLE_OAUTH_TOKEN_URL: &str =
    "https://oauth2.googleapis.com/token";

pub fn google_oauth_token_url() -> String {
    env::var("GOOGLE_OAUTH_TOKEN_URL").unwrap_or_else(|_| GOOGLE_OAUTH_TOKEN_URL.to_string())
}

pub fn is_debug() -> bool {
    env::var("AIUSAGE_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn debug_log(msg: impl std::fmt::Display) {
    if is_debug() {
        eprintln!("[aiusage:debug] {msg}");
    }
}

pub fn gemini_oauth_token() -> PathBuf {
    env::var("ANTIGRAVITY_TOKEN_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".gemini/antigravity-cli/antigravity-oauth-token"))
}
pub fn discover(key: &str) -> DiscoveryResult {
    let supported = matches!(key, "codex" | "grok" | "gemini_cli");
    let present = installed(key);
    let ready = present
        && supported
        && match key {
            "codex" => {
                env::var_os("CODEX_API_KEY").is_some_and(|s| !s.is_empty())
                    || home().join(".codex/auth.json").is_file()
            }
            "grok" => grok_log().is_file() && File::open(grok_log()).is_ok(),
            "gemini_cli" => {
                env::var_os("GEMINI_API_KEY").is_some_and(|s| !s.is_empty())
                    || (gemini_oauth_token().is_file() && File::open(gemini_oauth_token()).is_ok())
            }
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
                usage_supported: matches!(key, "codex" | "grok" | "gemini_cli"),
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
    let mut text = value.as_str()?.replace('Z', "+00:00");
    // The compatibility contract accepts fractional seconds only at
    // millisecond or microsecond precision.
    for (index, _) in text.match_indices('.') {
        let digits = text[index + 1..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if !matches!(digits, 3 | 6) {
            return None;
        }
    }
    // fromisoformat accepts a time specified to the hour or minute.
    if text.len() >= 13 && text.is_char_boundary(10) && text.is_char_boundary(11) {
        let end = text[11..]
            .find(['+', '-'])
            .map(|i| i + 11)
            .unwrap_or(text.len());
        if end == 13 {
            text.insert_str(end, ":00:00");
        } else if end == 16 {
            text.insert_str(end, ":00");
        }
    }
    let epoch = |value: DateTime<chrono::FixedOffset>| {
        // Quota timestamps reject leap seconds.
        (value.nanosecond() < 1_000_000_000)
            .then(|| value.timestamp() as f64 + value.timestamp_subsec_micros() as f64 / 1e6)
    };
    if let Ok(value) = DateTime::parse_from_rfc3339(&text) {
        return epoch(value);
    }
    for format in ["%Y-%m-%d %H:%M:%S%.f%:z", "%Y-%m-%dT%H:%M:%S%.f%:z"] {
        if let Ok(value) = DateTime::parse_from_str(&text, format) {
            return epoch(value);
        }
    }
    let naive = NaiveDateTime::parse_from_str(&text, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(&text, "%Y-%m-%d %H:%M:%S%.f"))
        .or_else(|_| {
            NaiveDate::parse_from_str(&text, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap())
        })
        .ok()?;
    let value = Local.from_local_datetime(&naive).earliest()?;
    epoch(value.fixed_offset())
}
pub fn grok_window(config: &Value) -> Option<RateLimitWindow> {
    config.as_object()?;
    // Epoch zero falls back to the billing-period field by contract.
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
    read_codex_cancellable(
        executable,
        timeout,
        &std::sync::atomic::AtomicBool::new(false),
    )
}
pub fn read_codex_cancellable(
    executable: &Path,
    timeout: Duration,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Vec<RateLimitWindow>, String> {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("usage read cancelled".into());
    }
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
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err("usage read cancelled".into());
                }
                if Instant::now() >= deadline {
                    return Err("Codex usage timeout".into());
                }
                let item = match rx.recv_timeout(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(50)),
                ) {
                    Ok(item) => item,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => return Err("Codex usage timeout".into()),
                };
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
    // This PID is exclusively owned by this invocation. Use graceful
    // termination, a bounded wait, then force termination as a last resort.
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
pub fn parse_gemini_oauth_token(data: &Value) -> Option<String> {
    data.get("token")
        .and_then(|t| {
            t.get("access_token")
                .and_then(|v| v.as_str())
                .or_else(|| t.as_str())
        })
        .or_else(|| data.get("access_token").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

pub fn parse_gemini_refresh_token(data: &Value) -> Option<String> {
    data.get("token")
        .and_then(|t| t.get("refresh_token").and_then(|v| v.as_str()))
        .or_else(|| data.get("refresh_token").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .or_else(|| env::var("ANTIGRAVITY_REFRESH_TOKEN").ok())
        .or_else(|| env::var("GEMINI_REFRESH_TOKEN").ok())
}

pub fn parse_gemini_client_id(data: &Value) -> String {
    env::var("ANTIGRAVITY_CLIENT_ID")
        .or_else(|_| env::var("GEMINI_CLIENT_ID"))
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            data.get("token")
                .and_then(|t| t.get("client_id"))
                .or_else(|| data.get("client_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(default_gemini_client_id)
}

pub fn parse_gemini_client_secret(data: &Value) -> String {
    env::var("ANTIGRAVITY_CLIENT_SECRET")
        .or_else(|_| env::var("GEMINI_CLIENT_SECRET"))
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            data.get("token")
                .and_then(|t| t.get("client_secret"))
                .or_else(|| data.get("client_secret"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(default_gemini_client_secret)
}

pub fn parse_gemini_token_expiry(data: &Value) -> Option<DateTime<Utc>> {
    let raw = data
        .get("token")
        .and_then(|t| t.get("expiry"))
        .or_else(|| data.get("expiry"))
        .or_else(|| data.get("token").and_then(|t| t.get("expires_at")))
        .or_else(|| data.get("expires_at"));

    if let Some(val) = raw {
        if let Some(s) = val.as_str() {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(dt.with_timezone(&Utc));
            }
            if let Ok(sec) = s.parse::<f64>() {
                let secs = if sec > 100_000_000_000.0 { sec / 1000.0 } else { sec };
                return DateTime::from_timestamp(secs as i64, 0);
            }
        }
        if let Some(num) = val.as_f64() {
            let secs = if num > 100_000_000_000.0 { num / 1000.0 } else { num };
            return DateTime::from_timestamp(secs as i64, 0);
        }
    }

    let date_raw = data
        .get("token")
        .and_then(|t| t.get("expiry_date"))
        .or_else(|| data.get("expiry_date"));
    if let Some(val) = date_raw {
        if let Some(num) = val.as_f64() {
            let secs = if num > 100_000_000_000.0 { num / 1000.0 } else { num };
            return DateTime::from_timestamp(secs as i64, 0);
        }
    }

    None
}

pub fn is_gemini_token_expired(data: &Value) -> bool {
    if let Some(exp) = parse_gemini_token_expiry(data) {
        let now = Utc::now();
        let threshold = now + chrono::Duration::seconds(60);
        return exp <= threshold;
    }
    false
}

pub fn refresh_gemini_token(
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<Value, String> {
    let token_url = google_oauth_token_url();
    debug_log(format!("Refreshing OAuth credentials via {token_url}"));

    let body = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("client_id", client_id)
        .append_pair("client_secret", client_secret)
        .append_pair("refresh_token", refresh_token)
        .append_pair("grant_type", "refresh_token")
        .finish();

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(crate::DISCOVERY_TIMEOUT_SECONDS))
        .build();

    let resp = agent
        .post(&token_url)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body);

    match resp {
        Ok(response) => {
            let text = response
                .into_string()
                .map_err(|e| format!("Failed to read OAuth refresh response: {e}"))?;
            let json_val = serde_json::from_str::<Value>(&text)
                .map_err(|e| format!("Failed to parse OAuth refresh JSON: {e}"))?;
            if json_val.get("access_token").is_some() {
                debug_log("OAuth credentials refresh successful");
                Ok(json_val)
            } else {
                Err("OAuth refresh response missing credential".to_string())
            }
        }
        Err(ureq::Error::Status(code, response)) => {
            let body_text = response.into_string().unwrap_or_default();
            let err_desc = serde_json::from_str::<Value>(&body_text)
                .ok()
                .and_then(|v| {
                    v.get("error_description")
                        .and_then(|d| d.as_str())
                        .or_else(|| v.get("error").and_then(|e| e.as_str()))
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| body_text.trim().to_string());
            Err(format!("OAuth refresh failed (HTTP {code}): {err_desc}"))
        }
        Err(ureq::Error::Transport(transport_err)) => {
            Err(format!("OAuth refresh network error: {transport_err}"))
        }
    }
}

pub fn save_gemini_token_file(
    token_path: &Path,
    current_data: &Value,
    refresh_resp: &Value,
) -> Result<(), String> {
    let new_access_token = refresh_resp
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing access credential in refresh response".to_string())?;

    let expires_in = refresh_resp
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(3600);

    let new_expiry = (Utc::now() + chrono::Duration::seconds(expires_in)).to_rfc3339();

    let mut updated = current_data.clone();
    if let Some(t_obj) = updated.get_mut("token").and_then(|v| v.as_object_mut()) {
        t_obj.insert("access_token".into(), Value::String(new_access_token.into()));
        t_obj.insert("expiry".into(), Value::String(new_expiry));
        if let Some(rt) = refresh_resp.get("refresh_token").and_then(|v| v.as_str()) {
            t_obj.insert("refresh_token".into(), Value::String(rt.into()));
        }
    } else {
        if !updated.is_object() {
            updated = json!({
                "auth_method": "consumer",
                "token": {}
            });
            let t_obj = updated.get_mut("token").unwrap().as_object_mut().unwrap();
            t_obj.insert("access_token".into(), Value::String(new_access_token.into()));
            t_obj.insert("expiry".into(), Value::String(new_expiry));
        } else {
            updated["access_token"] = Value::String(new_access_token.into());
            updated["expiry"] = Value::String(new_expiry);
        }
        if let Some(rt) = refresh_resp.get("refresh_token").and_then(|v| v.as_str()) {
            updated["refresh_token"] = Value::String(rt.into());
        }
    }

    if let Some(id) = refresh_resp.get("id_token").and_then(|v| v.as_str()) {
        updated["id_token"] = Value::String(id.into());
    }

    let parent = token_path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        let _ = fs::create_dir_all(parent);
    }

    let serialized = serde_json::to_string_pretty(&updated)
        .map_err(|e| format!("Failed to serialize credentials: {e}"))?;

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Failed to create temp credentials file: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o600));
    }

    temp.write_all(serialized.as_bytes())
        .map_err(|e| format!("Failed to write temp credentials file: {e}"))?;
    temp.flush()
        .map_err(|e| format!("Failed to flush temp credentials file: {e}"))?;

    temp.persist(token_path)
        .map_err(|e| format!("Failed to persist credentials file {}: {e}", token_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(token_path, fs::Permissions::from_mode(0o600));
    }

    debug_log(format!("Successfully wrote refreshed credentials to {}", token_path.display()));
    Ok(())
}

pub fn ensure_valid_gemini_token(
    token_path: &Path,
    token_data: &Value,
    force_refresh: bool,
) -> Result<String, String> {
    let is_expired = is_gemini_token_expired(token_data);
    let current_token = parse_gemini_oauth_token(token_data);
    let refresh_token = parse_gemini_refresh_token(token_data);

    if force_refresh || is_expired || current_token.is_none() {
        if let Some(rt) = refresh_token {
            let client_id = parse_gemini_client_id(token_data);
            let client_secret = parse_gemini_client_secret(token_data);
            debug_log(format!(
                "Refreshing credentials (force={}, expired={}, has_curr={})",
                force_refresh, is_expired, current_token.is_some()
            ));
            match refresh_gemini_token(&rt, &client_id, &client_secret) {
                Ok(resp) => {
                    let new_token = resp
                        .get("access_token")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Refresh response missing access credential".to_string())?
                        .to_string();

                    if let Err(e) = save_gemini_token_file(token_path, token_data, &resp) {
                        debug_log(format!("Warning: failed to save credentials file: {e}"));
                    }
                    return Ok(new_token);
                }
                Err(err) => {
                    debug_log(format!("Credentials refresh failed: {err}"));
                    if force_refresh {
                        return Err(err);
                    }
                    if is_expired {
                        return Err(format!("Credentials expired and refresh failed: {err}"));
                    }
                }
            }
        } else if is_expired {
            return Err("Credentials expired and no refresh credential found".to_string());
        }
    }

    current_token.ok_or_else(|| "No valid Gemini credentials found".to_string())
}

fn classify_gemini_window(bucket: &Value) -> Option<String> {
    if let Some(w) = bucket.get("window").and_then(|v| v.as_str()) {
        let w_lower = w.to_ascii_lowercase();
        if w_lower == "5h" || w_lower == "18000s" || w_lower == "18000" || w_lower == "300m" {
            return Some("5h".into());
        }
        if w_lower == "weekly"
            || w_lower == "week"
            || w_lower == "7d"
            || w_lower == "604800s"
            || w_lower == "604800"
            || w_lower == "10080m"
        {
            return Some("Week".into());
        }
    }
    if let Some(sec) = bucket.get("windowDuration").and_then(|v| {
        v.as_f64().or_else(|| {
            v.as_str().and_then(|s| s.trim_end_matches('s').parse::<f64>().ok())
        })
    }) {
        if (sec - 18000.0).abs() < 1.0 {
            return Some("5h".into());
        }
        if (sec - 604800.0).abs() < 1.0 {
            return Some("Week".into());
        }
        return Some(window_label(Some(sec / 60.0), "Cycle"));
    }
    if let Some(mins) = bucket.get("windowDurationMins").and_then(number) {
        if (mins - 300.0).abs() < 1.0 {
            return Some("5h".into());
        }
        if (mins - 10080.0).abs() < 1.0 {
            return Some("Week".into());
        }
        return Some(window_label(Some(mins), "Cycle"));
    }
    if let Some(id) = bucket.get("bucketId").and_then(|v| v.as_str()) {
        let id_lower = id.to_ascii_lowercase();
        if id_lower.contains("5h") {
            return Some("5h".into());
        }
        if id_lower.contains("weekly") || id_lower.contains("week") || id_lower.contains("7d") {
            return Some("Week".into());
        }
    }
    if let Some(name) = bucket.get("displayName").and_then(|v| v.as_str()) {
        let name_lower = name.to_ascii_lowercase();
        if name_lower.contains("5 hour")
            || name_lower.contains("5-hour")
            || name_lower.contains("five hour")
            || name_lower.contains("5h")
        {
            return Some("5h".into());
        }
        if name_lower.contains("week")
            || name_lower.contains("7 day")
            || name_lower.contains("7-day")
            || name_lower.contains("seven day")
        {
            return Some("Week".into());
        }
    }
    bucket
        .get("displayName")
        .and_then(|s| s.as_str())
        .or_else(|| bucket.get("bucketId").and_then(|s| s.as_str()))
        .map(|s| s.to_string())
}

fn extract_remaining_percent(bucket: &Value) -> Option<i32> {
    if let Some(fraction) = bucket.get("remainingFraction").and_then(number) {
        return Some((fraction * 100.0).round_ties_even().clamp(0.0, 100.0) as i32);
    }
    if let Some(remaining) = bucket.get("remaining").and_then(number) {
        return remaining_from_used(100.0 - remaining).ok();
    }
    if let Some(used) = bucket.get("usedPercent").and_then(number) {
        return remaining_from_used(used).ok();
    }
    if let Some(used_frac) = bucket.get("usedFraction").and_then(number) {
        return Some(((1.0 - used_frac) * 100.0).round_ties_even().clamp(0.0, 100.0) as i32);
    }
    if let (Some(rem), Some(total)) = (
        bucket.get("remainingAmount").and_then(number),
        bucket
            .get("totalAmount")
            .or_else(|| bucket.get("quotaAmount"))
            .and_then(number),
    ) {
        if total > 0.0 {
            return Some((rem / total * 100.0).round_ties_even().clamp(0.0, 100.0) as i32);
        }
    }
    None
}

fn extract_reset_at(bucket: &Value) -> Option<f64> {
    bucket
        .get("resetTime")
        .or_else(|| bucket.get("resetsAt"))
        .or_else(|| bucket.get("reset_at"))
        .and_then(timestamp)
}

pub fn gemini_quota_windows(reply: &Value) -> Result<Vec<RateLimitWindow>, String> {
    if reply.get("error").is_some() {
        return Err("Gemini quota unavailable".into());
    }
    let reply = reply.get("response").unwrap_or(reply);
    let mut raw_windows = vec![];

    if let Some(groups) = reply.get("groups").and_then(|g| g.as_array()) {
        let gemini_group = groups.iter().find(|g| {
            g.get("displayName")
                .and_then(|s| s.as_str())
                .is_some_and(|name| name.to_ascii_lowercase().contains("gemini"))
                || g.get("buckets")
                    .and_then(|b| b.as_array())
                    .is_some_and(|buckets| {
                        buckets.iter().any(|b| {
                            b.get("bucketId")
                                .and_then(|id| id.as_str())
                                .is_some_and(|id| id.to_ascii_lowercase().contains("gemini"))
                        })
                    })
        });

        let target_buckets: Vec<&Value> = if let Some(g) = gemini_group {
            g.get("buckets")
                .and_then(|b| b.as_array())
                .map(|arr| arr.iter().collect())
                .unwrap_or_default()
        } else {
            groups
                .iter()
                .filter_map(|g| g.get("buckets").and_then(|b| b.as_array()))
                .flatten()
                .collect()
        };

        for b in target_buckets {
            if let (Some(label), Some(pct)) = (classify_gemini_window(b), extract_remaining_percent(b)) {
                let reset = extract_reset_at(b);
                raw_windows.push(RateLimitWindow {
                    label,
                    remaining_percent: pct,
                    reset_at: reset,
                });
            }
        }
    }

    if raw_windows.is_empty() {
        if let Some(buckets) = reply.get("buckets").and_then(|b| b.as_array()) {
            for b in buckets {
                if let (Some(label), Some(pct)) = (classify_gemini_window(b), extract_remaining_percent(b)) {
                    let reset = extract_reset_at(b);
                    raw_windows.push(RateLimitWindow {
                        label,
                        remaining_percent: pct,
                        reset_at: reset,
                    });
                }
            }
        }
    }

    if raw_windows.is_empty() {
        if let Some(quota_info) = reply.get("quotaInfo") {
            if let Some(arr) = quota_info.as_array() {
                for b in arr {
                    if let (Some(label), Some(pct)) = (classify_gemini_window(b), extract_remaining_percent(b)) {
                        let reset = extract_reset_at(b);
                        raw_windows.push(RateLimitWindow {
                            label,
                            remaining_percent: pct,
                            reset_at: reset,
                        });
                    }
                }
            } else if let Some(obj) = quota_info.as_object() {
                for (k, v) in obj {
                    if v.is_object() {
                        let label = match k.to_ascii_lowercase().as_str() {
                            "5h" | "18000s" => "5h".to_string(),
                            "7d" | "week" | "weekly" | "604800s" => "Week".to_string(),
                            _ => classify_gemini_window(v).unwrap_or_else(|| k.clone()),
                        };
                        if let Some(pct) = extract_remaining_percent(v) {
                            let reset = extract_reset_at(v);
                            raw_windows.push(RateLimitWindow {
                                label,
                                remaining_percent: pct,
                                reset_at: reset,
                            });
                        }
                    }
                }
                if raw_windows.is_empty() {
                    if let Some(pct) = extract_remaining_percent(quota_info) {
                        let label = classify_gemini_window(quota_info).unwrap_or_else(|| "Quota".into());
                        let reset = extract_reset_at(quota_info);
                        raw_windows.push(RateLimitWindow {
                            label,
                            remaining_percent: pct,
                            reset_at: reset,
                        });
                    }
                }
            }
        }
    }

    let mut five_h = None;
    let mut week = None;
    let mut others = vec![];
    for w in raw_windows {
        if w.label == "5h" && five_h.is_none() {
            five_h = Some(w);
        } else if w.label == "Week" && week.is_none() {
            week = Some(w);
        } else if w.label != "5h" && w.label != "Week" {
            others.push(w);
        }
    }

    let mut windows = vec![];
    if let Some(h) = five_h {
        windows.push(h);
    }
    if let Some(w) = week {
        windows.push(w);
    }
    windows.extend(others);

    Ok(windows)
}

pub fn read_gemini(cancel: &std::sync::atomic::AtomicBool) -> Result<Vec<RateLimitWindow>, String> {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("usage read cancelled".into());
    }
    let token_path = gemini_oauth_token();
    let has_token_file = token_path.is_file();
    let api_key = env::var("GEMINI_API_KEY").ok().filter(|s| !s.is_empty());

    if !has_token_file && api_key.is_none() {
        return Err("Gemini credentials not found".into());
    }

    if has_token_file {
        let token_content = fs::read_to_string(&token_path)
            .map_err(|e| format!("Failed to read Gemini credentials: {e}"))?;
        let token_data = serde_json::from_str::<Value>(&token_content)
            .map_err(|e| format!("Failed to parse Gemini credentials: {e}"))?;

        let mut access_token = match ensure_valid_gemini_token(&token_path, &token_data, false) {
            Ok(tok) => tok,
            Err(err) => {
                debug_log(format!("ensure_valid_gemini_token failed: {err}"));
                return Err(err);
            }
        };

        let project_id = env::var("ANTIGRAVITY_PROJECT_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                fs::read_to_string(home().join(".gemini/antigravity-cli/cache/default_project_id.txt"))
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "default-cli-project".to_string());
        let body =
            serde_json::to_string(&serde_json::json!({"project": project_id}))
                .unwrap_or_default();
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(crate::DISCOVERY_TIMEOUT_SECONDS))
            .build();

        let urls = [
            "https://daily-cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
            "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
        ];

        let mut last_error = None;
        let mut did_refresh_on_401 = false;

        'url_loop: for url in urls {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return Err("usage read cancelled".into());
            }

            debug_log(format!("Requesting quota from {url}..."));
            let mut resp = agent
                .post(url)
                .set("Authorization", &format!("Bearer {access_token}"))
                .set("Content-Type", "application/json")
                .set("User-Agent", "antigravity/1.3.3")
                .send_string(&body);

            // Reactive refresh: if 401 Unauthorized, refresh and retry once
            if let Err(ureq::Error::Status(401, _)) = &resp {
                if !did_refresh_on_401 {
                    did_refresh_on_401 = true;
                    debug_log("Quota request returned 401 Unauthorized. Attempting credential refresh...");
                    let current_file_content = fs::read_to_string(&token_path).unwrap_or_default();
                    let current_file_data = serde_json::from_str::<Value>(&current_file_content)
                        .unwrap_or_else(|_| token_data.clone());
                    match ensure_valid_gemini_token(&token_path, &current_file_data, true) {
                        Ok(new_tok) => {
                            debug_log("Successfully refreshed credentials on 401. Retrying request...");
                            access_token = new_tok;
                            resp = agent
                                .post(url)
                                .set("Authorization", &format!("Bearer {access_token}"))
                                .set("Content-Type", "application/json")
                                .set("User-Agent", "antigravity/1.3.3")
                                .send_string(&body);
                        }
                        Err(refresh_err) => {
                            debug_log(format!("Credential refresh on 401 failed: {refresh_err}"));
                            last_error = Some(format!("Credentials expired (HTTP 401): {refresh_err}"));
                            continue 'url_loop;
                        }
                    }
                }
            }

            match resp {
                Ok(resp) => {
                    if let Ok(text) = resp.into_string() {
                        if let Ok(quota_json) = serde_json::from_str::<Value>(&text) {
                            if let Ok(windows) = gemini_quota_windows(&quota_json) {
                                if !windows.is_empty() {
                                    debug_log(format!("Found {} quota windows from {url}", windows.len()));
                                    return Ok(windows);
                                } else {
                                    debug_log(format!("Valid response from {url} but 0 windows (unlimited tier)"));
                                    return Ok(vec![]);
                                }
                            } else {
                                last_error = Some(format!("Failed to parse quota windows from {url}"));
                            }
                        } else {
                            last_error = Some(format!("Failed to parse JSON response from {url}"));
                        }
                    } else {
                        last_error = Some(format!("Failed to read response body from {url}"));
                    }
                }
                Err(ureq::Error::Status(code, response)) => {
                    let err_body = response.into_string().unwrap_or_default();
                    debug_log(format!("Quota request to {url} failed with HTTP {code}: {err_body}"));
                    last_error = Some(format!("Quota API error (HTTP {code})"));
                }
                Err(ureq::Error::Transport(transport_err)) => {
                    debug_log(format!("Quota request to {url} failed with transport error: {transport_err}"));
                    last_error = Some(format!("Network error: {transport_err}"));
                }
            }
        }

        if let Some(err) = last_error {
            return Err(err);
        }
    }

    if api_key.is_some() {
        return Ok(vec![]);
    }

    Err("Gemini credentials unavailable".into())
}
pub fn read(key: &str) -> ProviderUsage {
    read_cancellable(key, &std::sync::atomic::AtomicBool::new(false))
}
pub fn read_cancellable(key: &str, cancel: &std::sync::atomic::AtomicBool) -> ProviderUsage {
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
            .and_then(|exe| {
                read_codex_cancellable(
                    &exe,
                    Duration::from_secs(crate::CODEX_TIMEOUT_SECONDS),
                    cancel,
                )
            }),
        "grok" => read_grok(&grok_log()),
        "gemini_cli" => read_gemini(cancel),
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
