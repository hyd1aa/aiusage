//! UI state machine. Effects are returned to the caller, never hidden in keys.
use crate::{
    config::{Config, POSITIONS, THEMES},
    demo,
    i18n::tr,
    models::{retain_stale, Availability, ProviderUsage},
    providers::DiscoveryResult,
    render,
    timezones::{self, Zone, PRESETS},
    PROVIDERS,
};
use std::collections::HashMap;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Effects {
    pub quit: bool,
    pub save: bool,
    pub refresh: bool,
    pub discover: bool,
}
#[derive(Clone)]
pub struct Dashboard {
    pub demo: bool,
    pub cfg: Config,
    pub color: bool,
    pub updated: Option<f64>,
    pub states: HashMap<String, ProviderUsage>,
    pub discovery_states: HashMap<String, DiscoveryResult>,
    pub selecting: bool,
    pub cursor: usize,
    pub draft: Vec<String>,
    pub timezone_selecting: bool,
    pub timezone_cursor: usize,
    pub timezone_options: Vec<String>,
    pub notice: Option<String>,
    pub notice_until: f64,
}
impl Dashboard {
    pub fn new(demo: bool, cfg: Config, color: bool) -> Self {
        Self {
            demo,
            cfg,
            color,
            updated: None,
            states: HashMap::new(),
            discovery_states: HashMap::new(),
            selecting: false,
            cursor: 0,
            draft: vec![],
            timezone_selecting: false,
            timezone_cursor: 0,
            timezone_options: vec![],
            notice: None,
            notice_until: 0.0,
        }
    }
    pub fn enabled(&self) -> &[String] {
        if self.demo {
            &self.cfg.demo_providers
        } else {
            &self.cfg.real_providers
        }
    }
    pub fn enabled_mut(&mut self) -> &mut Vec<String> {
        if self.demo {
            &mut self.cfg.demo_providers
        } else {
            &mut self.cfg.real_providers
        }
    }
    pub fn apply_refresh(&mut self, values: Vec<ProviderUsage>, now: f64) {
        let mut success = false;
        for value in values {
            let key = value.key.clone();
            let state = retain_stale(value, self.states.get(&key));
            success |= state.availability == Availability::Available;
            self.states.insert(key, state);
        }
        if success {
            self.updated = Some(now);
        }
    }
    pub fn refresh_with(&mut self, now: f64, mut reader: impl FnMut(&str) -> ProviderUsage) {
        let values = self
            .enabled()
            .iter()
            .map(|key| {
                if self.demo {
                    demo::usage(key, now).expect("registered demo")
                } else {
                    reader(key)
                }
            })
            .collect();
        self.apply_refresh(values, now);
    }
    pub fn apply_discovery(
        &mut self,
        results: HashMap<String, DiscoveryResult>,
        monotonic: f64,
    ) -> bool {
        if self.demo || !self.cfg.auto_discover {
            return false;
        }
        let mut names = vec![];
        // Stable registry order instead of thread-completion order.
        for &(key, name) in PROVIDERS {
            let Some(mut value) = results.get(key).cloned() else {
                continue;
            };
            if matches!(
                value.reason.as_str(),
                "timeout" | "malformed" | "unavailable"
            ) {
                if let Some(old) = self.discovery_states.get(key) {
                    value = old.clone();
                }
            }
            if value.usable
                && !self.enabled().iter().any(|k| k == key)
                && !self.cfg.disabled_providers.iter().any(|k| k == key)
            {
                self.enabled_mut().push(key.into());
                names.push(name);
            }
            self.discovery_states.insert(key.into(), value);
        }
        if names.is_empty() {
            return false;
        }
        self.notice = Some(render::label_value(
            tr(&self.cfg.language, "discovered"),
            &names.join(", "),
            &self.cfg.language,
        ));
        self.notice_until = monotonic + crate::NOTICE_SECONDS as f64;
        true
    }
    pub fn frame(&self, width: usize, height: usize, now: f64, monotonic: f64) -> Vec<String> {
        if self.selecting {
            let discovery = self
                .discovery_states
                .iter()
                .map(|(k, v)| (k.clone(), v.reason.clone()))
                .collect();
            return render::selector(
                width,
                height,
                &self.draft,
                self.cursor,
                &self.cfg,
                self.color,
                &discovery,
            );
        }
        if self.timezone_selecting {
            return render::timezone_selector(
                width,
                height,
                &self.timezone_options,
                self.timezone_cursor,
                &self.cfg,
                self.color,
            );
        }
        let providers: Vec<_> = self
            .enabled()
            .iter()
            .map(|key| {
                self.states
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| ProviderUsage {
                        key: key.clone(),
                        name: PROVIDERS.iter().find(|(k, _)| k == key).unwrap().1.into(),
                        availability: Availability::Unavailable,
                        windows: vec![],
                        stale: false,
                        error: None,
                    })
            })
            .collect();
        render::dashboard(render::Frame {
            width,
            height,
            providers: &providers,
            config: &self.cfg,
            updated: self.updated,
            now,
            demo: self.demo,
            color: self.color,
            notice: if monotonic < self.notice_until {
                self.notice.as_deref()
            } else {
                None
            },
        })
    }
    pub fn key(&mut self, key: &[u8]) -> Effects {
        let mut effects = Effects::default();
        let down = matches!(key, b"j" | b"J" | b"\x1b[B");
        let up = matches!(key, b"k" | b"K" | b"\x1b[A");
        let enter = matches!(key, b"\r" | b"\n");
        if self.timezone_selecting {
            if key == b"\x1b" {
                self.timezone_selecting = false;
            } else if down {
                self.timezone_cursor =
                    (self.timezone_cursor + 1).min(self.timezone_options.len() - 1);
            } else if up {
                self.timezone_cursor = self.timezone_cursor.saturating_sub(1);
            } else if enter {
                self.cfg.timezone = self.timezone_options[self.timezone_cursor].clone();
                effects.save = true;
                self.timezone_selecting = false;
            } else if self.timezone_cursor == self.timezone_options.len() - 1 {
                let delta = match key {
                    b"h" | b"H" | b"\x1b[D" => -15,
                    b"l" | b"L" | b"\x1b[C" => 15,
                    _ => 0,
                };
                let old = self.timezone_options.last_mut().unwrap();
                let minutes = match timezones::parse(old).unwrap() {
                    Zone::System => 0,
                    Zone::Fixed(n) => n,
                };
                *old = timezones::offset_setting(
                    (minutes + delta).clamp(timezones::MINUTES_MIN, timezones::MINUTES_MAX),
                )
                .unwrap();
            }
            return effects;
        }
        if self.selecting {
            if key == b"\x1b" {
                self.selecting = false;
            } else if enter {
                let previous = self.enabled().to_vec();
                let selected = self.draft.clone();
                self.cfg
                    .disabled_providers
                    .retain(|k| !selected.contains(k));
                for key in previous {
                    if !selected.contains(&key) && !self.cfg.disabled_providers.contains(&key) {
                        self.cfg.disabled_providers.push(key);
                    }
                }
                *self.enabled_mut() = selected;
                self.selecting = false;
                effects.save = true;
                effects.refresh = true;
            } else if down {
                self.cursor = (self.cursor + 1).min(PROVIDERS.len() - 1);
            } else if up {
                self.cursor = self.cursor.saturating_sub(1);
            } else {
                let current = PROVIDERS[self.cursor].0;
                if key == b" " {
                    if let Some(index) = self.draft.iter().position(|k| k == current) {
                        self.draft.remove(index);
                    } else {
                        self.draft.push(current.into());
                    }
                } else if let Some(index) = self.draft.iter().position(|k| k == current) {
                    let target = match key {
                        b"u" | b"U" => index.saturating_sub(1),
                        b"d" | b"D" => (index + 1).min(self.draft.len() - 1),
                        _ => index,
                    };
                    self.draft.swap(index, target);
                }
            }
            return effects;
        }
        match key {
            b"q" | b"Q" | b"\x1b" | b"\x03" => effects.quit = true,
            b"l" | b"L" => {
                self.cfg.language = if self.cfg.language == "en" {
                    "zh"
                } else {
                    "en"
                }
                .into();
                effects.save = true;
            }
            b"t" | b"T" => {
                let index = THEMES.iter().position(|t| *t == self.cfg.theme).unwrap();
                self.cfg.theme = THEMES[(index + 1) % THEMES.len()].into();
                effects.save = true;
            }
            b"p" | b"P" => {
                let index = POSITIONS
                    .iter()
                    .position(|p| *p == self.cfg.position)
                    .unwrap();
                self.cfg.position = POSITIONS[(index + 1) % POSITIONS.len()].into();
                effects.save = true;
            }
            b"s" | b"S" => {
                self.selecting = true;
                self.draft = self.enabled().to_vec();
                self.cursor = 0;
            }
            b"z" | b"Z" => {
                let custom = if PRESETS.contains(&self.cfg.timezone.as_str()) {
                    "UTC+05:30"
                } else {
                    &self.cfg.timezone
                };
                self.timezone_options = PRESETS
                    .iter()
                    .map(|s| s.to_string())
                    .chain([custom.into()])
                    .collect();
                self.timezone_cursor = self
                    .timezone_options
                    .iter()
                    .position(|s| *s == self.cfg.timezone)
                    .unwrap_or(self.timezone_options.len() - 1);
                self.timezone_selecting = true;
            }
            b"r" | b"R" => {
                effects.discover = true;
                effects.refresh = true;
            }
            _ => {}
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn demo_never_calls_reader() {
        let mut board = Dashboard::new(true, Config::default(), false);
        board.refresh_with(1000.0, |_| panic!("real reader in demo"));
        assert_eq!(board.states.len(), 6);
        assert!(!board.apply_discovery(HashMap::new(), 0.0));
    }
    #[test]
    fn keys_save_only_on_commit() {
        let mut board = Dashboard::new(true, Config::default(), false);
        assert!(board.key(b"L").save);
        assert_eq!(board.cfg.language, "en");
        board.key(b"S");
        board.key(b" ");
        board.key(b"\x1b");
        assert_eq!(board.enabled().len(), 6);
        board.key(b"S");
        board.key(b" ");
        assert!(board.key(b"\r").refresh);
        assert_eq!(board.enabled().len(), 5);
        board.key(b"Z");
        board.timezone_cursor = 6;
        board.key(b"\x1b[C");
        board.key(b"\r");
        assert_eq!(board.cfg.timezone, "UTC+05:45");
    }
    #[test]
    fn explicit_disable_and_transient_discovery() {
        let mut board = Dashboard::new(
            false,
            Config {
                real_providers: vec!["codex".into()],
                disabled_providers: vec!["grok".into()],
                ..Default::default()
            },
            false,
        );
        let ready = crate::providers::classify_discovery(true, true, true);
        assert!(!board.apply_discovery(HashMap::from([("grok".into(), ready.clone())]), 0.0));
        board.cfg.disabled_providers.clear();
        assert!(board.apply_discovery(HashMap::from([("grok".into(), ready.clone())]), 0.0));
        let timeout = DiscoveryResult {
            reason: "timeout".into(),
            usable: false,
            ..ready.clone()
        };
        board.apply_discovery(HashMap::from([("grok".into(), timeout)]), 1.0);
        assert_eq!(board.discovery_states["grok"], ready);
    }
}
