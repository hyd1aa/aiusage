pub mod cadence;
pub mod cli;
pub mod config;
pub mod dashboard;
pub mod demo;
pub mod diagnostics;
pub mod i18n;
pub mod manager;
pub mod models;
pub mod providers;
pub mod render;
pub mod timezones;
pub mod updater;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REFRESH_SECONDS: u64 = 30;
pub const DISCOVERY_SECONDS: u64 = 300;
pub const NOTICE_SECONDS: u64 = 5;
pub const DISCOVERY_TIMEOUT_SECONDS: u64 = 2;
pub const CODEX_TIMEOUT_SECONDS: u64 = 8;

pub const PROVIDERS: &[(&str, &str)] = &[
    ("codex", "Codex"),
    ("grok", "Grok"),
    ("minimax", "MiniMax"),
    ("qoder", "Qoder"),
    ("qodercn", "Qoder CN"),
    ("codebuddy", "CodeBuddy"),
    ("traecode", "TraeCode"),
    ("zcode", "ZCode"),
];
