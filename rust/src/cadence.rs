//! Monotonic deadlines, independent of wall-clock/timezone changes.
#[derive(Debug, Clone, Copy)]
pub struct Cadence {
    pub next_refresh: f64,
    pub next_discovery: f64,
}
impl Default for Cadence {
    fn default() -> Self {
        Self {
            next_refresh: 0.0,
            next_discovery: 0.0,
        }
    }
}
impl Cadence {
    pub fn refresh_due(&self, now: f64) -> bool {
        now >= self.next_refresh
    }
    pub fn discovery_due(&self, now: f64) -> bool {
        now >= self.next_discovery
    }
    pub fn completed(&mut self, now: f64, discovered: bool) {
        self.next_refresh = now + crate::REFRESH_SECONDS as f64;
        if discovered {
            self.next_discovery = now + crate::DISCOVERY_SECONDS as f64;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_then_independent_periods() {
        let mut cadence = Cadence::default();
        assert!(cadence.refresh_due(0.0));
        assert!(cadence.discovery_due(0.0));
        cadence.completed(0.0, true);
        for tick in 1..10 {
            let now = tick as f64 * 30.0;
            assert!(!cadence.refresh_due(now - 0.01));
            assert!(cadence.refresh_due(now));
            assert!(!cadence.discovery_due(now));
            cadence.completed(now, false);
        }
        assert!(cadence.discovery_due(300.0));
        cadence.completed(301.0, true);
        assert_eq!(cadence.next_discovery, 601.0);
        assert_eq!(cadence.next_refresh, 331.0);
    }
}
