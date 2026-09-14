use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability { Available, NotInstalled, Unavailable, NotSupported }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimitWindow {
    pub label: String,
    pub remaining_percent: i32,
    pub reset_at: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub key: String,
    pub name: String,
    pub availability: Availability,
    pub windows: Vec<RateLimitWindow>,
    pub stale: bool,
    pub error: Option<String>,
}

/// Failed reads retain evidence, never fabricate a quota. Uninstallation does
/// not retain old windows, matching the reference's availability distinction.
pub fn retain_stale(fresh: ProviderUsage, old: Option<&ProviderUsage>) -> ProviderUsage {
    if fresh.availability == Availability::Unavailable {
        if let Some(old) = old.filter(|old| !old.windows.is_empty()) {
            return ProviderUsage { stale: true, error: fresh.error, ..old.clone() };
        }
    }
    fresh
}

pub fn remaining_from_used(used: f64) -> Result<i32, &'static str> {
    if !used.is_finite() { return Err("invalid usage percentage"); }
    Ok(100 - used.round_ties_even().clamp(0.0, 100.0) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounding_matches_python_not_rust_round() {
        for (used, expected) in [(0.0,100), (100.0,0), (47.0,53),
            (0.5,100), (1.5,98), (2.5,98), (-1.0,100), (101.0,0)] {
            assert_eq!(remaining_from_used(used), Ok(expected));
        }
        assert!(remaining_from_used(f64::NAN).is_err());
        assert!(remaining_from_used(f64::INFINITY).is_err());
    }
    #[test]
    fn retention_is_not_fake_success() {
        let old = ProviderUsage { key:"grok".into(), name:"Grok".into(), availability:Availability::Available,
            windows:vec![RateLimitWindow{label:"Week".into(),remaining_percent:40,reset_at:Some(123.0)}], stale:false,error:None };
        let failure = ProviderUsage{availability:Availability::Unavailable, windows:vec![],error:Some("timeout".into()),..old.clone()};
        let retained = retain_stale(failure.clone(),Some(&old));
        assert!(retained.stale);
        assert_eq!(retained.windows,old.windows);
        assert_eq!(retain_stale(failure.clone(),None),failure);
        let removed = ProviderUsage{availability:Availability::NotInstalled,..failure};
        assert_eq!(retain_stale(removed.clone(),Some(&old)),removed);
        assert_eq!(retain_stale(old.clone(),Some(&retained)),old);
    }
}
