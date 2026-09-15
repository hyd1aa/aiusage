use aiusage::{config, models::remaining_from_used, timezones};
use serde_json::{json, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn reference(cases: &[Value], tz: &str) -> Vec<Value> {
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reference.py"))
        .env("TZ", tz)
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("XDG_CACHE_HOME", home.path())
        .env_remove("CODEX_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Python reference required");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_vec(cases).unwrap().as_slice())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn argparse_edge_matrix() {
    use aiusage::cli::{self, Parsed};
    let tokens = [
        "--help",
        "-h",
        "-hh",
        "-hX",
        "-hhX",
        "--help=X",
        "--snapshot",
        "--snapshot=false",
        "--size=-١",
        "-x foo",
        "中文",
        "--version",
        "--v",
        "--demo",
        "--demo=yes",
        "--size",
        "--size=",
        "--s",
        "--",
        "",
        "hello",
        "-",
        "-1",
        "-١",
        "--unknown",
        "--menu=x",
    ];
    let mut cases = vec![];
    for a in tokens {
        cases.push(json!({"op":"args", "args":[a]}));
        for b in tokens {
            cases.push(json!({"op":"args", "args":[a,b]}));
        }
    }
    let mut differences = vec![];
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let args: Vec<String> = serde_json::from_value(case["args"].clone()).unwrap();
        let actual = match cli::parse(&args) {
            Ok(Parsed::Args(a)) => {
                json!({"args":{"demo":a.demo,"menu":a.menu,"snapshot":a.snapshot,"size":a.size},"code":null,"stdout":"","stderr":""})
            }
            Ok(Parsed::Help) => json!({"args":null,"code":0,"stdout":cli::HELP,"stderr":""}),
            Ok(Parsed::Version) => {
                json!({"args":null,"code":0,"stdout":format!("AIUsage {}\n", aiusage::VERSION),"stderr":""})
            }
            Err(e) => {
                json!({"args":null,"code":2,"stdout":"","stderr":format!("{}\naiusage: error: {e}\n",cli::USAGE)})
            }
        };
        if actual != expected {
            differences.push(format!("{args:?}\nRust {actual}\nPython {expected}"));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn version_comparison_differential() {
    let cases: Vec<_> = [
        "0.2.2",
        "0.2.3",
        "1.0.0",
        "0.2",
        "0.2.2.0",
        "",
        "v0.2.2",
        "1..3",
        "1.2.rc1",
        "-1.0.0",
        "+1.02.3",
        "１.２.３",
        "1_0.2.3",
        " 1.2.3 ",
    ]
    .iter()
    .map(|v| json!({"op":"versions","value":v}))
    .collect();
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let value = case["value"].as_str().unwrap();
        assert_eq!(
            json!([
                aiusage::updater::version_tuple(value),
                aiusage::updater::is_newer(value, "0.2.2")
            ]),
            expected,
            "{value}"
        );
    }
}

#[test]
fn codex_reply_differential() {
    let mut cases = vec![];
    for used in [
        json!(0),
        json!(100),
        json!(47.5),
        json!(1.5),
        json!(-1),
        json!(101),
        json!(true),
        json!(null),
        json!("47"),
        json!({}),
    ] {
        for duration in [
            json!(300),
            json!(1440),
            json!(10080),
            json!(null),
            json!("300"),
        ] {
            cases.push(json!({"op":"codex","reply":{"result":{"rateLimits":{"primary":{"usedPercent":used,"windowDurationMins":duration,"resetsAt":1789475400.0},"secondary":{"usedPercent":0,"windowDurationMins":10080}}}}}));
        }
    }
    for reply in [
        json!({}),
        json!({"error":{}}),
        json!({"result":{"rateLimits":{}}}),
        json!({"result":{"rateLimits":{"primary":{"resetsAt":123}}}}),
    ] {
        cases.push(json!({"op":"codex","reply":reply}));
    }
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        assert_eq!(
            json!(aiusage::providers::codex_windows(&case["reply"]).ok()),
            expected,
            "{case}"
        );
    }
}

#[test]
fn actual_utf8_menu_bytes_and_narrow_width() {
    let cases: Vec<_> = ["zh", "en"]
        .into_iter()
        .flat_map(|language| {
            [10, 20, 39, 40, 80]
                .into_iter()
                .map(move |width| json!({"op":"menu_utf8","language":language,"width":width}))
        })
        .collect();
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let mut menu = aiusage::manager::Manager::new(
            config::Config {
                language: case["language"].as_str().unwrap().into(),
                ..Default::default()
            },
            std::io::Cursor::new(vec![]),
            vec![],
            aiusage::manager::Production,
        );
        // main_screen is pure: no run/background_latest or provider action.
        menu.width = case["width"].as_u64().unwrap() as usize;
        menu.main_screen().unwrap();
        assert_eq!(
            String::from_utf8(menu.output).unwrap(),
            expected.as_str().unwrap()
        );
    }
}

#[test]
fn dimensions_unicode_and_malformed_differential() {
    let cases: Vec<_> = [
        "",
        "80x24",
        " 80 X 24 ",
        "８０x２４",
        "٨٠x٢٤",
        "8_0x2_4",
        "_8x24",
        "8__0x24",
        "8_x24",
        "-1x24",
        "+80x24",
        "80x0",
        "0x0",
        "1.0x24",
        "80×24",
        "x",
        "80x24x1",
        "\u{2003}80x24",
        "NaNx24",
        "inf",
        "８⁰x24",
    ]
    .iter()
    .map(|s| json!({"op":"dimensions","value":s}))
    .collect();
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let actual = match aiusage::cli::dimensions(case["value"].as_str().unwrap()) {
            Ok((w, h)) => json!([w, h]),
            Err(e) => json!(e),
        };
        assert_eq!(actual, expected, "{case}");
    }
}

#[test]
fn timestamp_and_epoch_boundaries() {
    if std::env::var_os("AIUSAGE_TIMESTAMP_CHILD").is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "timestamp_and_epoch_boundaries", "--nocapture"])
            .env("AIUSAGE_TIMESTAMP_CHILD", "1")
            .env("TZ", "UTC")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let values = json!([
        null,
        false,
        true,
        0,
        -1,
        2147483647,
        253402300799i64,
        "",
        "broken",
        "2026-09-15",
        "2026-09-15T12:30",
        "2026-09-15T12",
        "2026-09-15T12:30:45",
        "2026-09-15T12:30:45Z",
        "2026-09-15T12:30:45+05:30",
        "2026-09-15T12:30:45.123456",
        "2026-09-15T12:30:60Z",
        "2026-02-30",
        "0001-01-01T00:00:00+00:00",
        "9999-12-31T23:59:59+00:00",
        "10000-01-01",
        [],
        {}
    ]);
    let mut cases: Vec<_> = values
        .as_array()
        .unwrap()
        .iter()
        .map(|v| json!({"op":"timestamp","value":v}))
        .collect();
    for value in [
        -62135596800.0,
        -62135596801.0,
        253402300799.0,
        253402300800.0,
        -0.0000006,
        0.9999996,
        1e30,
        -1e30,
    ] {
        for zone in ["UTC", "UTC+08", "UTC-12", "system", "bad"] {
            cases.push(json!({"op":"epoch","value":value,"zone":zone}));
        }
    }
    let mut differences = vec![];
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let actual = if case["op"] == "timestamp" {
            json!(aiusage::providers::timestamp(&case["value"]))
        } else {
            json!(timezones::from_epoch(
                case["value"].as_f64().unwrap(),
                case["zone"].as_str().unwrap()
            )
            .ok()
            .map(|v| v.format("%Y-%m-%dT%H:%M:%S%.6f%:z").to_string()))
        };
        if actual != expected {
            differences.push(format!("{case}: Rust={actual}, Python={expected}"));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
#[ignore = "Parity gate OPEN: select Python 3.10 or 3.11+ ISO acceptance contract; run explicitly with --ignored"]
fn version_sensitive_iso_acceptance_gate() {
    let cases: Vec<_> = [
        "2026-09-15T12:30:45.123456789+00:00",
        "20260915T12:30:45+00:00",
        "2026-W38-2T12:30:45+00:00",
        "2026-09-15T12:30:45,123456+00:00",
    ]
    .iter()
    .map(|v| json!({"op":"timestamp","value":v}))
    .collect();
    let mut differences = vec![];
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let actual = json!(aiusage::providers::timestamp(&case["value"]));
        if actual != expected {
            differences.push(format!("{case}: Rust={actual}, Python={expected}"));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn config_differential() {
    let mut texts = vec![
        String::new(),
        include_str!("../../config.example.toml").into(),
        "real_providers=['codex']".into(),
        "real_providers=['grok']\ndisabled_providers=[]".into(),
        "demo_providers=[\"gemini\",\"qoder\",\"qoder\",\"grok\"]".into(),
    ];
    for key in [
        "language",
        "theme",
        "position",
        "timezone",
        "auto_discover",
        "real_providers",
        "demo_providers",
        "disabled_providers",
    ] {
        for value in [
            "",
            "en",
            "'en'",
            "\"en\"",
            "FALSE",
            "nonsense",
            "[]",
            "[\"grok\",'codex',grok]",
            "\"UTC+05:30\"",
            "\"x#y\"",
            "[bogus]",
        ] {
            texts.push(format!("{key} = {value}\n"));
        }
    }
    let cases: Vec<_> = texts
        .iter()
        .map(|s| json!({"op":"config","text":s}))
        .collect();
    for (text, expected) in texts.iter().zip(reference(&cases, "UTC")) {
        let cfg = config::parse(text);
        assert_eq!(
            json!({"config":cfg,"saved":config::serialize(&cfg)}),
            expected,
            "{text}"
        );
    }
}

#[test]
fn rounding_differential() {
    let values: Vec<_> = (-10..=210).map(|i| i as f64 / 2.0).collect();
    let cases: Vec<_> = values
        .iter()
        .map(|v| json!({"op":"remaining","used":v}))
        .collect();
    for (used, expected) in values.iter().zip(reference(&cases, "UTC")) {
        assert_eq!(
            json!(remaining_from_used(*used).unwrap()),
            expected,
            "{used}"
        );
    }
}

#[test]
fn fixed_timezone_differential() {
    let mut settings: Vec<_> = (-720..=840)
        .step_by(15)
        .map(|i| timezones::offset_setting(i).unwrap())
        .collect();
    settings.extend(
        [
            "UTC+8",
            "UTC+14:01",
            "UTC-12:01",
            "UTC+05:60",
            "Asia/Shanghai",
            "UTC+０８",
            "UTC+٠٥:٤٥",
            "UTC-𝟎𝟒",
            "UTC+⁰⁸",
        ]
        .map(String::from),
    );
    let mut cases = vec![];
    for epoch in [0.0, -1.0, 1788375056.0, 1788461400.0, 1798761600.0] {
        for setting in &settings {
            cases.push(json!({"op":"timezone","setting":setting,"epoch":epoch}));
        }
    }
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let actual = timezones::from_epoch(
            case["epoch"].as_f64().unwrap(),
            case["setting"].as_str().unwrap(),
        )
        .ok()
        .map(|v| {
            json!([
                v.format("%Y-%m-%d %H:%M:%S").to_string(),
                timezones::label_for(&v)
            ])
        })
        .unwrap_or(Value::Null);
        assert_eq!(actual, expected, "{case}");
    }
}

#[test]
fn frame_golden_matrix() {
    use aiusage::{
        demo,
        render::{self, Frame},
    };
    let mut cases = vec![];
    for language in ["en", "zh"] {
        for theme in ["white", "green"] {
            for position in config::POSITIONS {
                for (width, height) in [(80, 24), (40, 12), (24, 8), (1, 1), (10, 3), (120, 40)] {
                    for count in [0, 1, 2, 4, 6, 8] {
                        let cfg = config::Config {
                            language: language.into(),
                            theme: theme.into(),
                            position: (*position).into(),
                            timezone: "UTC+05:30".into(),
                            demo_providers: aiusage::PROVIDERS
                                .iter()
                                .take(count)
                                .map(|(key, _)| (*key).into())
                                .collect(),
                            ..Default::default()
                        };
                        cases.push(json!({"op":"frame","config":cfg,"now":1788461400.0,"updated":1788461395.0,
                            "width":width,"height":height,"color":true,"demo":true,"notice":null}));
                    }
                }
            }
        }
    }
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let cfg: config::Config = serde_json::from_value(case["config"].clone()).unwrap();
        let providers: Vec<_> = cfg
            .demo_providers
            .iter()
            .map(|key| demo::usage(key, 1788461400.0).unwrap())
            .collect();
        let lines = render::dashboard(Frame {
            width: case["width"].as_u64().unwrap() as usize,
            height: case["height"].as_u64().unwrap() as usize,
            providers: &providers,
            config: &cfg,
            now: 1788461400.0,
            updated: Some(1788461395.0),
            color: true,
            demo: true,
            notice: None,
        });
        assert_eq!(json!(lines), expected, "{case}");
    }
}

#[test]
fn selectors_golden_matrix() {
    use aiusage::render;
    let mut cases = vec![];
    let options: Vec<_> = aiusage::timezones::PRESETS
        .iter()
        .chain([&"UTC+05:45"])
        .map(|s| s.to_string())
        .collect();
    for language in ["en", "zh"] {
        for (width, height) in [(80, 24), (24, 12), (120, 40)] {
            for cursor in 0..8 {
                for op in ["selector", "zone_selector"] {
                    cases.push(json!({"op":op,"config":config::Config{language:language.into(),..Default::default()},
                        "width":width,"height":height,"cursor":cursor,"color":true,"options":options}));
                }
            }
        }
    }
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let cfg: config::Config = serde_json::from_value(case["config"].clone()).unwrap();
        let (w, h, c) = (
            case["width"].as_u64().unwrap() as usize,
            case["height"].as_u64().unwrap() as usize,
            case["cursor"].as_u64().unwrap() as usize,
        );
        let actual = if case["op"] == "selector" {
            render::selector(
                w,
                h,
                &cfg.demo_providers,
                c,
                &cfg,
                true,
                &Default::default(),
            )
        } else {
            render::timezone_selector(w, h, &options, c, &cfg, true)
        };
        assert_eq!(json!(actual), expected, "{case}");
    }
}

#[test]
fn grok_differential() {
    let mut cases = vec![];
    for used in [
        Value::Null,
        json!(0),
        json!(47),
        json!(100),
        json!(0.5),
        json!(1.5),
        json!("bad"),
        json!(true),
    ] {
        for minutes in [60, 300, 1440, 10080, 20000] {
            let mut cfg = json!({"billingPeriodStart":1788461400.0,"billingPeriodEnd":1788461400.0+minutes as f64*60.0});
            if !used.is_null() {
                cfg["creditUsagePercent"] = used.clone();
            }
            cases.push(json!({"op":"grok","config":cfg}));
        }
    }
    cases.extend([json!({}),json!({"creditUsagePercent":12}),json!({"currentPeriod":{"start":"2026-09-05T15:14:41.162248+00:00","end":"2026-09-12T15:14:41.162248+00:00"}})]
        .into_iter().map(|cfg|json!({"op":"grok","config":cfg})));
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let actual =
            serde_json::to_value(aiusage::providers::grok_window(&case["config"])).unwrap();
        assert_eq!(actual, expected, "{case}");
    }
}

#[test]
fn system_timezone_differential() {
    // Each timezone gets its own process: never mutate TZ concurrently with
    // chrono or Python calls in other tests. Cover winter/summer DST and halves.
    if let Ok(tz) = std::env::var("AIUSAGE_TEST_ZONE") {
        let cases: Vec<_> = [1767268800.0, 1782907200.0, 1788461400.0]
            .into_iter()
            .map(|epoch| json!({"op":"timezone","setting":"system","epoch":epoch}))
            .collect();
        for (case, expected) in cases.iter().zip(reference(&cases, &tz)) {
            let value = timezones::from_epoch(case["epoch"].as_f64().unwrap(), "system").unwrap();
            assert_eq!(
                json!([
                    value.format("%Y-%m-%d %H:%M:%S").to_string(),
                    timezones::label_for(&value)
                ]),
                expected,
                "{tz}: {case}"
            );
        }
    } else {
        for tz in [
            "UTC",
            "Asia/Shanghai",
            "America/New_York",
            "Asia/Kathmandu",
            "Australia/Adelaide",
        ] {
            let result = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "system_timezone_differential", "--nocapture"])
                .env("AIUSAGE_TEST_ZONE", tz)
                .env("TZ", tz)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{tz}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}

#[test]
fn manager_screen_differential() {
    let mut cases = vec![];
    for language in ["zh", "en"] {
        for theme in ["white", "green"] {
            for width in [24, 80, 120] {
                for unicode in [false, true] {
                    for latest in [
                        Value::Null,
                        json!({"version":"9.0.0","title":"fixture","notes":"","tarball_url":"https://github.com/hyd1aa/aiusage/archive/v9.0.0.tar.gz"}),
                    ] {
                        cases.push(json!({"op":"manager","config":config::Config{language:language.into(),theme:theme.into(),..Default::default()},"width":width,"color":true,"unicode":unicode,"latest":latest}));
                    }
                }
            }
        }
    }
    for (case, expected) in cases.iter().zip(reference(&cases, "UTC")) {
        let cfg = serde_json::from_value(case["config"].clone()).unwrap();
        let mut menu = aiusage::manager::Manager::new(
            cfg,
            std::io::Cursor::new(vec![]),
            vec![],
            aiusage::manager::Production,
        );
        menu.width = case["width"].as_u64().unwrap() as usize;
        menu.color = true;
        menu.unicode = case["unicode"].as_bool().unwrap();
        menu.latest = serde_json::from_value(case["latest"].clone()).unwrap();
        menu.main_screen().unwrap();
        assert_eq!(
            json!(String::from_utf8(menu.output).unwrap()),
            expected,
            "{case}"
        );
    }
}
