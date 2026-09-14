use std::{collections::HashMap, env, fs, io, path::{Path, PathBuf}};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use serde::{Deserialize, Serialize};
use crate::{PROVIDERS, timezones};

pub const POSITIONS: &[&str] = &["top-left","top-center","top-right","center","bottom-left","bottom-center","bottom-right"];
pub const THEMES: &[&str] = &["white","green"];
pub const DEMO_DEFAULT: &[&str] = &["codex","grok","minimax","qoder","codebuddy","traecode"];

fn strings(values: &[&str]) -> Vec<String> { values.iter().map(|s|(*s).into()).collect() }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub language: String,
    pub theme: String,
    pub position: String,
    pub timezone: String,
    pub auto_discover: bool,
    pub real_providers: Vec<String>,
    pub demo_providers: Vec<String>,
    pub disabled_providers: Vec<String>,
}

impl Default for Config {
    fn default()->Self {
        Self{language:"zh".into(),theme:"white".into(),position:"center".into(),timezone:"system".into(),
            auto_discover:true,real_providers:strings(&["codex","grok"]),demo_providers:strings(DEMO_DEFAULT),disabled_providers:vec![]}
    }
}

pub fn config_path()->PathBuf {
    env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .unwrap_or_else(|| env::var_os("HOME").map(PathBuf::from).unwrap_or_else(||PathBuf::from("~")).join(".config"))
        .join("aiusage/config.toml")
}

pub fn valid(values:&[String])->Vec<String> {
    let mut result=vec![];
    for value in values {
        if PROVIDERS.iter().any(|(key,_)|key==value) && !result.contains(value) { result.push(value.clone()); }
    }
    result
}

pub fn parse(text:&str)->Config {
    let mut cfg=Config::default();
    let mut values=HashMap::new();
    for raw in text.lines() {
        let line=raw.split('#').next().unwrap_or("").trim();
        if let Some((key,value))=line.split_once('=') {values.insert(key.trim(),value.trim());}
    }
    for (key,dest) in [("language",&mut cfg.language),("theme",&mut cfg.theme),("position",&mut cfg.position),("timezone",&mut cfg.timezone)] {
        if let Some(value)=values.get(key) { *dest=value.trim_matches('"').into(); }
    }
    cfg.auto_discover=values.get("auto_discover").map(|v|!v.eq_ignore_ascii_case("false")).unwrap_or(true);
    for (key,dest) in [("real_providers",&mut cfg.real_providers),("demo_providers",&mut cfg.demo_providers),("disabled_providers",&mut cfg.disabled_providers)] {
        if let Some(value)=values.get(key) {
            *dest=if value.starts_with('[') && value.ends_with(']') {
                value[1..value.len()-1].split(',').filter(|v|!v.trim().is_empty())
                    .map(|v|v.trim().trim_matches('"').trim_matches('\'').to_string()).collect()
            } else {vec![]};
        }
        *dest=valid(dest);
    }
    if !["en","zh"].contains(&cfg.language.as_str()) {cfg.language="zh".into();}
    if !THEMES.contains(&cfg.theme.as_str()) {cfg.theme="white".into();}
    if !POSITIONS.contains(&cfg.position.as_str()) {cfg.position="center".into();}
    if timezones::parse(&cfg.timezone).is_err() {cfg.timezone="system".into();}
    if cfg.real_providers.is_empty() {cfg.real_providers=strings(&["codex","grok"]);}
    if cfg.demo_providers.is_empty() {cfg.demo_providers=strings(DEMO_DEFAULT);}
    cfg.disabled_providers.retain(|k|!cfg.real_providers.contains(k));
    if values.contains_key("real_providers") && !values.contains_key("disabled_providers") {
        for key in ["codex","grok"] {
            if !cfg.real_providers.iter().any(|s|s==key) {cfg.disabled_providers.push(key.into());}
        }
    }
    cfg
}

pub fn load(path:&Path)->Config {
    fs::read_to_string(path).map(|text|parse(&text)).unwrap_or_default()
}

pub fn serialize(cfg:&Config)->String {
    let mut text=format!("language = \"{}\"\ntheme = \"{}\"\nposition = \"{}\"\ntimezone = \"{}\"\nauto_discover = {}\n",
        cfg.language,cfg.theme,cfg.position,cfg.timezone,cfg.auto_discover);
    for (key,list) in [("real_providers",&cfg.real_providers),("demo_providers",&cfg.demo_providers),("disabled_providers",&cfg.disabled_providers)] {
        let items=valid(list).iter().map(|v|format!("\"{v}\"")).collect::<Vec<_>>().join(", ");
        text.push_str(&format!("{key} = [{items}]\n"));
    }
    text
}

pub fn save(cfg:&Config,path:&Path)->bool {
    let temporary=path.with_extension("tmp");
    let result=(||->io::Result<()> {
        let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        fs::DirBuilder::new().recursive(true).mode(0o700).create(parent)?;
        fs::write(&temporary,serialize(cfg))?;
        fs::set_permissions(&temporary,fs::Permissions::from_mode(0o600))?;
        fs::rename(&temporary,path)
    })();
    if result.is_err() {let _=fs::remove_file(temporary);}
    result.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn legacy_and_explicit_disable() {
        let cfg=parse("real_providers = [\"codex\"]\n");
        assert_eq!(cfg.disabled_providers,vec!["grok"]);
        let cfg=parse("real_providers = [\"codex\"]\ndisabled_providers = []\n");
        assert!(cfg.disabled_providers.is_empty());
    }
    #[test] fn permissive_not_standard_toml() {
        let cfg=parse("language = en\nreal_providers=['grok', 'bogus', 'grok']\nauto_discover = FALSE\n");
        assert_eq!(cfg.language,"en");assert_eq!(cfg.real_providers,vec!["grok"]);assert!(!cfg.auto_discover);
        assert_eq!(parse("language='en'").language,"zh");
        assert_eq!(parse("language=en\nlanguage=zh").language,"zh");
        assert_eq!(parse("timezone=\"UTC+15\"").timezone,"system");
    }
    #[test] fn private_atomic_roundtrip() {
        let temp=tempfile::tempdir().unwrap();
        let path=temp.path().join("new/config.toml");
        let cfg=Config{theme:"green".into(),..Config::default()};
        assert!(save(&cfg,&path)); assert_eq!(load(&path),cfg);
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode()&0o777,0o600);
        assert!(!path.with_extension("tmp").exists());
        assert_eq!(load(&temp.path().join("missing")),Config::default());
        fs::write(&path,[255,254]).unwrap();
        assert_eq!(load(&path),Config::default());
    }
}
