use crate::{config::Config, providers, timezones};
pub type Row = (String, bool, String);
pub fn collect(cfg: &Config, github_ok: Option<bool>) -> Vec<Row> {
    let encoding = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_CTYPE"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let utf8 = encoding.to_lowercase().replace('-', "").contains("utf8");
    let mut rows = vec![
        ("AIUsage".into(), true, format!("v{}", crate::VERSION)),
        ("Rust".into(), true, "native binary".into()),
        (
            "Terminal".into(),
            utf8,
            if utf8 { "UTF-8" } else { "unknown" }.into(),
        ),
        ("Config".into(), true, "readable".into()),
    ];
    let discovery = providers::discover_all();
    for &(key, name) in crate::PROVIDERS {
        let state = &discovery[key];
        let disabled = cfg.disabled_providers.iter().any(|k| k == key);
        rows.push((
            name.into(),
            state.usable && !disabled,
            if disabled {
                "disabled_by_user".into()
            } else {
                state.reason.clone()
            },
        ));
    }
    for (key, name) in [("codex", "Codex usage"), ("grok", "Grok usage")] {
        let readable = discovery[key].usable && !providers::read(key).windows.is_empty();
        rows.push((
            name.into(),
            readable,
            if readable { "readable" } else { "unavailable" }.into(),
        ));
    }
    let now = crate::cli::now();
    for (name, setting) in [
        ("System timezone", "system"),
        ("Display timezone", cfg.timezone.as_str()),
    ] {
        let value = timezones::from_epoch(now, setting).ok();
        rows.push((
            name.into(),
            value.is_some(),
            value
                .map(|v| timezones::label_for(&v))
                .unwrap_or("unknown".into()),
        ));
    }
    rows.push((
        "GitHub".into(),
        github_ok == Some(true),
        match github_ok {
            Some(true) => "available",
            Some(false) => "unavailable",
            None => "unknown",
        }
        .into(),
    ));
    rows
}
