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

#[test]
fn gemini_refresh_token_and_expiry_check() {
    let nested = json!({
        "token": {
            "access_token": "ya29.nested",
            "refresh_token": "1//nested-refresh",
            "expiry": "2020-01-01T00:00:00+08:00"
        }
    });
    assert_eq!(
        parse_gemini_refresh_token(&nested),
        Some("1//nested-refresh".into())
    );
    assert!(is_gemini_token_expired(&nested));

    let top_level = json!({
        "access_token": "ya29.top",
        "refresh_token": "1//top-refresh",
        "expiry": "2099-01-01T00:00:00Z"
    });
    assert_eq!(
        parse_gemini_refresh_token(&top_level),
        Some("1//top-refresh".into())
    );
    assert!(!is_gemini_token_expired(&top_level));

    // Numeric timestamp in milliseconds
    let numeric_past = json!({
        "token": {
            "expiry_date": 1577836800000.0 // 2020-01-01
        }
    });
    assert!(is_gemini_token_expired(&numeric_past));

    let numeric_future = json!({
        "token": {
            "expiry_date": 4102444800000.0 // 2100-01-01
        }
    });
    assert!(!is_gemini_token_expired(&numeric_future));

    let no_expiry = json!({
        "token": {
            "access_token": "ya29.ok"
        }
    });
    assert_eq!(parse_gemini_token_expiry(&no_expiry), None);
    assert!(!is_gemini_token_expired(&no_expiry));
}

#[test]
fn gemini_token_file_save_and_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let token_path = dir.path().join("antigravity-oauth-token");
    let current_data = json!({
        "token": {
            "access_token": "old-token",
            "token_type": "Bearer",
            "refresh_token": "1//old-refresh",
            "expiry": "2020-01-01T00:00:00Z"
        },
        "auth_method": "consumer"
    });
    let refresh_resp = json!({
        "access_token": "new-ya29-token",
        "expires_in": 3600,
        "token_type": "Bearer"
    });

    save_gemini_token_file(&token_path, &current_data, &refresh_resp).unwrap();

    let saved_content = fs::read_to_string(&token_path).unwrap();
    let saved_json: serde_json::Value = serde_json::from_str(&saved_content).unwrap();

    assert_eq!(
        saved_json["token"]["access_token"],
        "new-ya29-token"
    );
    assert_eq!(
        saved_json["token"]["refresh_token"],
        "1//old-refresh"
    );
    assert_eq!(
        saved_json["auth_method"],
        "consumer"
    );
    assert!(!is_gemini_token_expired(&saved_json));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&token_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn gemini_mock_token_refresh() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_url = format!("http://127.0.0.1:{port}/token");

    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut req = String::new();
        let mut buffer = [0u8; 1024];
        loop {
            let n = stream.read(&mut buffer).unwrap();
            if n == 0 { break; }
            req.push_str(&String::from_utf8_lossy(&buffer[..n]));
            if req.contains("grant_type=refresh_token") {
                break;
            }
        }
        assert!(req.contains("grant_type=refresh_token"));
        assert!(req.contains("refresh_token=test-rt"));

        let body = r#"{"access_token":"refreshed-access-token","expires_in":3600}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    std::env::set_var("GOOGLE_OAUTH_TOKEN_URL", &server_url);
    let result = refresh_gemini_token("test-rt", "test-client-id", "test-client-secret");
    std::env::remove_var("GOOGLE_OAUTH_TOKEN_URL");
    handle.join().unwrap();

    assert!(result.is_ok());
    let json_val = result.unwrap();
    assert_eq!(json_val["access_token"], "refreshed-access-token");
    assert_eq!(json_val["expires_in"], 3600);
}

#[test]
fn gemini_read_expired_without_refresh_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let token_path = dir.path().join("antigravity-oauth-token");
    let expired_no_refresh = json!({
        "token": {
            "access_token": "expired-token",
            "expiry": "2020-01-01T00:00:00Z"
        }
    });
    fs::write(&token_path, expired_no_refresh.to_string()).unwrap();

    std::env::set_var("ANTIGRAVITY_TOKEN_PATH", &token_path);
    std::env::remove_var("GEMINI_API_KEY");

    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result = read_gemini(&cancel);

    std::env::remove_var("ANTIGRAVITY_TOKEN_PATH");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("Credentials expired"));
}


