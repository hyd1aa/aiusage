pub mod config;
pub mod models;
pub mod timezones;
pub mod render;
pub mod i18n;
pub mod demo;

pub const VERSION: &str = env!("AIUSAGE_VERSION");
pub const REFRESH_SECONDS: u64 = 30;
pub const DISCOVERY_SECONDS: u64 = 300;
pub const NOTICE_SECONDS: u64 = 5;
pub const DISCOVERY_TIMEOUT_SECONDS: u64 = 2;
pub const CODEX_TIMEOUT_SECONDS: u64 = 8;

pub const PROVIDERS: &[(&str, &str)] = &[
    ("codex", "Codex"), ("grok", "Grok"), ("minimax", "MiniMax"),
    ("qoder", "Qoder"), ("qodercn", "Qoder CN"), ("codebuddy", "CodeBuddy"),
    ("traecode", "TraeCode"), ("zcode", "ZCode"),
];
