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

#[test]
fn gemini_token_parse_and_quota_windows() {
    let token_json = json!({
        "token": {
            "access_token": "ya29.fake-token",
            "token_type": "Bearer"
        },
        "auth_method": "consumer"
    });
    assert_eq!(
        parse_gemini_oauth_token(&token_json),
        Some("ya29.fake-token".into())
    );
    assert_eq!(parse_gemini_oauth_token(&json!({})), None);

    let quota_response = json!({
        "buckets": [
            {
                "bucketId": "gemini-flash",
                "displayName": "Flash",
                "remaining": 80.0,
                "resetTime": "2026-10-15T12:00:00Z"
            }
        ]
    });
    let windows = gemini_quota_windows(&quota_response).unwrap();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].label, "Flash");
    assert_eq!(windows[0].remaining_percent, 80);

    let empty_windows = gemini_quota_windows(&json!({})).unwrap();
    assert!(empty_windows.is_empty());

    // Google Code Assist retrieveUserQuotaSummary format with groups
    let quota_groups_response = json!({
        "groups": [
            {
                "displayName": "Gemini Models",
                "buckets": [
                    {
                        "bucketId": "gemini-weekly",
                        "displayName": "Weekly Limit Remaining",
                        "window": "weekly",
                        "resetTime": "2026-10-17T14:49:57Z",
                        "remainingFraction": 0.9431
                    },
                    {
                        "bucketId": "gemini-5h",
                        "displayName": "Five Hour Limit Remaining",
                        "window": "5h",
                        "resetTime": "2026-10-11T06:49:57Z",
                        "remainingFraction": 0.6592
                    }
                ]
            },
            {
                "displayName": "Claude and GPT models",
                "buckets": [
                    {
                        "bucketId": "3p-5h",
                        "window": "5h",
                        "remainingFraction": 1.0,
                        "resetTime": "2026-10-11T10:20:20Z"
                    }
                ]
            }
        ]
    });
    let windows_groups = gemini_quota_windows(&quota_groups_response).unwrap();
    assert_eq!(windows_groups.len(), 2);
    assert_eq!(windows_groups[0].label, "5h");
    assert_eq!(windows_groups[0].remaining_percent, 66);
    assert!(windows_groups[0].reset_at.is_some());
    assert_eq!(windows_groups[1].label, "Week");
    assert_eq!(windows_groups[1].remaining_percent, 94);
    assert!(windows_groups[1].reset_at.is_some());

    // Window duration in seconds format (18000s / 604800s)
    let quota_durations_response = json!({
        "buckets": [
            {
                "bucketId": "b-week",
                "windowDuration": "604800s",
                "remainingFraction": 0.85,
                "resetTime": "2026-10-17T12:00:00Z"
            },
            {
                "bucketId": "b-5h",
                "windowDuration": "18000s",
                "remainingFraction": 0.45,
                "resetTime": "2026-10-11T15:00:00Z"
            }
        ]
    });
    let windows_durations = gemini_quota_windows(&quota_durations_response).unwrap();
    assert_eq!(windows_durations.len(), 2);
    assert_eq!(windows_durations[0].label, "5h");
    assert_eq!(windows_durations[0].remaining_percent, 45);
    assert_eq!(windows_durations[1].label, "Week");
    assert_eq!(windows_durations[1].remaining_percent, 85);

    // quotaInfo with 5h and 7d objects
    let quota_info_response = json!({
        "quotaInfo": {
            "5h": {
                "remainingFraction": 0.5,
                "resetTime": "2026-10-11T14:49:00Z"
            },
            "7d": {
                "remainingFraction": 0.9,
                "resetTime": "2026-10-17T14:49:00Z"
            }
        }
    });
    let windows_qi = gemini_quota_windows(&quota_info_response).unwrap();
    assert_eq!(windows_qi.len(), 2);
    assert_eq!(windows_qi[0].label, "5h");
    assert_eq!(windows_qi[0].remaining_percent, 50);
    assert_eq!(windows_qi[1].label, "Week");
    assert_eq!(windows_qi[1].remaining_percent, 90);
}

