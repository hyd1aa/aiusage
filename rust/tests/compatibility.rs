use aiusage::{config, models::remaining_from_used, timezones};
use serde_json::{json, Value};
use std::{io::Write, process::{Command, Stdio}};

fn reference(cases: &[Value], tz: &str) -> Vec<Value> {
    let home=tempfile::tempdir().unwrap();
    let mut child=Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"),"/tests/reference.py"))
        .env("TZ",tz).env("HOME",home.path()).env("XDG_CONFIG_HOME",home.path())
        .env("XDG_CACHE_HOME",home.path()).env_remove("CODEX_API_KEY")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("Python reference required");
    child.stdin.take().unwrap().write_all(serde_json::to_vec(cases).unwrap().as_slice()).unwrap();
    let result=child.wait_with_output().unwrap();
    assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn config_differential() {
    let mut texts=vec![String::new(),include_str!("../../config.example.toml").into(),
        "real_providers=['codex']".into(),"real_providers=['grok']\ndisabled_providers=[]".into(),
        "demo_providers=[\"gemini\",\"qoder\",\"qoder\",\"grok\"]".into()];
    for key in ["language","theme","position","timezone","auto_discover","real_providers","demo_providers","disabled_providers"] {
        for value in ["", "en", "'en'", "\"en\"", "FALSE", "nonsense", "[]", "[\"grok\",'codex',grok]", "\"UTC+05:30\"", "\"x#y\"", "[bogus]"] {
            texts.push(format!("{key} = {value}\n"));
        }
    }
    let cases:Vec<_>=texts.iter().map(|s|json!({"op":"config","text":s})).collect();
    for (text,expected) in texts.iter().zip(reference(&cases,"UTC")) {
        let cfg=config::parse(text);
        assert_eq!(json!({"config":cfg,"saved":config::serialize(&cfg)}),expected,"{text}");
    }
}

#[test]
fn rounding_differential() {
    let values:Vec<_>=(-10..=210).map(|i|i as f64/2.0).collect();
    let cases:Vec<_>=values.iter().map(|v|json!({"op":"remaining","used":v})).collect();
    for (used,expected) in values.iter().zip(reference(&cases,"UTC")) {
        assert_eq!(json!(remaining_from_used(*used).unwrap()),expected,"{used}");
    }
}

#[test]
fn fixed_timezone_differential() {
    let mut settings:Vec<_>=(-720..=840).step_by(15).map(|i|timezones::offset_setting(i).unwrap()).collect();
    settings.extend(["UTC+8","UTC+14:01","UTC-12:01","UTC+05:60","Asia/Shanghai"].map(String::from));
    let mut cases=vec![];
    for epoch in [0.0,-1.0,1788375056.0,1788461400.0,1798761600.0] {
        for setting in &settings {cases.push(json!({"op":"timezone","setting":setting,"epoch":epoch}));}
    }
    for (case,expected) in cases.iter().zip(reference(&cases,"UTC")) {
        let actual=timezones::from_epoch(case["epoch"].as_f64().unwrap(),case["setting"].as_str().unwrap()).ok()
            .map(|v|json!([v.format("%Y-%m-%d %H:%M:%S").to_string(),timezones::label_for(&v)])).unwrap_or(Value::Null);
        assert_eq!(actual,expected,"{case}");
    }
}

#[test]
fn frame_golden_matrix() {
    use aiusage::{demo, render::{self, Frame}};
    let mut cases=vec![];
    for language in ["en","zh"] {
        for theme in ["white","green"] {
            for position in config::POSITIONS {
                for (width,height) in [(80,24),(40,12),(24,8),(1,1),(10,3),(120,40)] {
                    for count in [0,1,2,4,6,8] {
                        let cfg=config::Config{language:language.into(),theme:theme.into(),position:(*position).into(),timezone:"UTC+05:30".into(),
                            demo_providers:aiusage::PROVIDERS.iter().take(count).map(|(key,_)|(*key).into()).collect(),..Default::default()};
                        cases.push(json!({"op":"frame","config":cfg,"now":1788461400.0,"updated":1788461395.0,
                            "width":width,"height":height,"color":true,"demo":true,"notice":null}));
                    }
                }
            }
        }
    }
    for (case,expected) in cases.iter().zip(reference(&cases,"UTC")) {
        let cfg:config::Config=serde_json::from_value(case["config"].clone()).unwrap();
        let providers:Vec<_>=cfg.demo_providers.iter().map(|key|demo::usage(key,1788461400.0).unwrap()).collect();
        let lines=render::dashboard(Frame{width:case["width"].as_u64().unwrap() as usize,height:case["height"].as_u64().unwrap() as usize,
            providers:&providers,config:&cfg,now:1788461400.0,updated:Some(1788461395.0),color:true,demo:true,notice:None});
        assert_eq!(json!(lines),expected,"{case}");
    }
}

#[test]
fn selectors_golden_matrix() {
    use aiusage::render;
    let mut cases=vec![];
    let options:Vec<_>=aiusage::timezones::PRESETS.iter().chain([&"UTC+05:45"]).map(|s|s.to_string()).collect();
    for language in ["en","zh"] {
        for (width,height) in [(80,24),(24,12),(120,40)] {
            for cursor in 0..8 {
                for op in ["selector","zone_selector"] {
                    cases.push(json!({"op":op,"config":config::Config{language:language.into(),..Default::default()},
                        "width":width,"height":height,"cursor":cursor,"color":true,"options":options}));
                }
            }
        }
    }
    for (case,expected) in cases.iter().zip(reference(&cases,"UTC")) {
        let cfg:config::Config=serde_json::from_value(case["config"].clone()).unwrap();
        let (w,h,c)=(case["width"].as_u64().unwrap() as usize,case["height"].as_u64().unwrap() as usize,case["cursor"].as_u64().unwrap() as usize);
        let actual=if case["op"]=="selector" {render::selector(w,h,&cfg.demo_providers,c,&cfg,true,&Default::default())}
            else {render::timezone_selector(w,h,&options,c,&cfg,true)};
        assert_eq!(json!(actual),expected,"{case}");
    }
}
