//! Pure fixtures: no filesystem, credentials, executable lookup or network.
use crate::{
    models::{Availability, ProviderUsage, RateLimitWindow},
    PROVIDERS,
};

pub fn usage(key: &str, now: f64) -> Option<ProviderUsage> {
    let name = PROVIDERS.iter().find(|(k, _)| *k == key)?.1;
    let values: &[(&str, i32, f64)] = match key {
        "codex" => &[("5h", 83, 2.2), ("Week", 61, 74.0)],
        "grok" => &[("Cycle", 72, 51.0)],
        "minimax" => &[("5h", 64, 4.0), ("Week", 42, 82.0)],
        "qoder" => &[("Cycle", 91, 96.0)],
        "qodercn" => &[("Cycle", 76, 72.0)],
        "codebuddy" => &[("Credits", 66, 120.0)],
        "traecode" => &[("Cycle", 37, 240.0)],
        "zcode" => &[("Cycle", 55, 36.0)],
        _ => return None,
    };
    Some(ProviderUsage {
        key: key.into(),
        name: name.into(),
        availability: Availability::Available,
        windows: values
            .iter()
            .map(|(label, remaining, hours)| RateLimitWindow {
                label: (*label).into(),
                remaining_percent: *remaining,
                reset_at: Some(now + hours * 3600.0),
            })
            .collect(),
        stale: false,
        error: None,
    })
}
