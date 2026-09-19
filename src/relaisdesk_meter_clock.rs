use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct ConnectedClock {
    last: Option<Instant>,
    elapsed: Duration,
}

impl ConnectedClock {
    pub(crate) fn confirm(&mut self, now: Instant) {
        if let Some(last) = self.last {
            if let Some(delta) = now.checked_duration_since(last) {
                if delta <= Duration::from_secs(3) {
                    self.elapsed = self.elapsed.saturating_add(delta);
                }
            }
        }
        self.last = Some(now);
    }

    pub(crate) fn millis(&self) -> u64 {
        self.elapsed.as_millis().min(u64::MAX as u128) as u64
    }
    pub(crate) fn started(&self) -> bool {
        self.last.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_confirmed_time_only() {
        let start = Instant::now();
        let mut c = ConnectedClock::default();
        assert!(!c.started());
        c.confirm(start);
        c.confirm(start + Duration::from_secs(1));
        assert_eq!(c.millis(), 1000);
        c.confirm(start + Duration::from_secs(40));
        assert_eq!(c.millis(), 1000);
        c.confirm(start + Duration::from_secs(41));
        assert_eq!(c.millis(), 2000);
    }
    #[test]
    fn never_rounds_reconnections_up() {
        let mut total = 0;
        for _ in 0..10 {
            let mut c = ConnectedClock::default();
            let t = Instant::now();
            c.confirm(t);
            c.confirm(t + Duration::from_millis(125));
            total += c.millis();
        }
        assert_eq!(total, 1250);
    }
    #[test]
    fn frequent_packets_preserve_submillisecond_time() {
        let mut c = ConnectedClock::default();
        let t = Instant::now();
        for i in 0..=10000 {
            c.confirm(t + Duration::from_micros(i * 100));
        }
        assert_eq!(c.millis(), 1000);
    }
}
