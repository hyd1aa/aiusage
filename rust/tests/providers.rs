use aiusage::providers::*;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};

#[test]
fn discovery_classifies_installation_support_and_login_separately() {
    for (installed, supported, ready, reason) in [
        (false, true, true, "not_installed"),
        (true, false, true, "unsupported"),
        (true, true, false, "needs_login"),
        (true, true, true, "ready"),
    ] {
        let state = classify_discovery(installed, supported, ready);
        assert_eq!(state.reason, reason);
        assert_eq!(state.usable, reason == "ready");
    }
}
#[test]
fn codex_missing_malformed_and_explicit_zero() {
    assert!(codex_windows(&json!({})).is_err());
    assert!(codex_windows(&json!({"error":{}})).is_err());
    for (used, remaining) in [(0, 100), (47, 53), (100, 0)] {
        let parsed=codex_windows(&json!({"result":{"rateLimits":{"primary":{"usedPercent":used,"windowDurationMins":300,"resetsAt":1788461400}}}})).unwrap();
        assert_eq!(parsed[0].remaining_percent, remaining);
        assert_eq!(parsed[0].label, "5h");
    }
    assert!(
        codex_windows(&json!({"result":{"rateLimits":{"primary":{"usedPercent":"bad"}}}})).is_err()
    );
}
#[test]
fn grok_rollover_and_bounded_tail() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("usage.jsonl");
    let old = json!({"ctx":{"config":{"billingPeriodEnd":10000,"creditUsagePercent":100}}});
    let new = json!({"ctx":{"config":{"billingPeriodEnd":20000}}});
    fs::write(&path, format!("{old}\n{new}\nnot json\n")).unwrap();
    let result = read_grok(&path).unwrap();
    assert_eq!(result[0].remaining_percent, 100);
    assert_eq!(result[0].reset_at, Some(20000.0));
    fs::write(&path, format!("{old}\n{}\n", "x".repeat(4 * 1024 * 1024))).unwrap();
    assert!(read_grok(&path).is_err());
    fs::write(&path, format!("{}\n{new}\n", "x".repeat(4 * 1024 * 1024))).unwrap();
    assert_eq!(read_grok(&path).unwrap()[0].remaining_percent, 100);
}
#[test]
fn codex_protocol_and_timeout_use_owned_fake_process() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("fake-codex");
    fs::write(&path,concat!("#!/bin/sh\n",
        "read init\nprintf '%s\\n' '{\"id\":1,\"result\":{}}'\n",
        "read initialized\nread limits\n",
        "printf '%s\\n' '{\"id\":2,\"result\":{\"rateLimits\":{\"primary\":{\"usedPercent\":47,\"windowDurationMins\":300}}}}'\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        read_codex(&path, Duration::from_secs(1)).unwrap()[0].remaining_percent,
        53
    );
    fs::write(&path, "#!/bin/sh\nread init\nread never\n").unwrap();
    let start = Instant::now();
    assert!(read_codex(&path, Duration::from_millis(50)).is_err());
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn cancellation_reaps_slow_owned_codex_without_waiting_eight_seconds() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("slow-codex");
    fs::write(&path, "#!/bin/sh\nread init\nread never\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let trigger = cancel.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        trigger.store(true, Ordering::Relaxed);
    });
    let start = Instant::now();
    assert!(read_codex_cancellable(&path, Duration::from_secs(8), &cancel).is_err());
    assert!(start.elapsed() < Duration::from_secs(2));
    thread.join().unwrap();
}
