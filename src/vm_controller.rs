use std::sync::atomic::{AtomicI64, AtomicU8, Ordering};

pub const STATE_IDLE: u8 = 0u8;
pub const STATE_ACTIVE: u8 = 1u8;

pub struct VmState {
    state: AtomicU8,
    idle_since_ms: AtomicI64,
}

impl VmState {
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_IDLE),
            idle_since_ms: AtomicI64::new(0),
        }
    }

    pub fn is_active(&self) -> bool {
        self.state.load(Ordering::SeqCst) == STATE_ACTIVE
    }

    pub fn activate(&self) {
        self.state.store(STATE_ACTIVE, Ordering::SeqCst);
    }

    pub fn mark_idle(&self, now_ms: i64) {
        self.state.store(STATE_IDLE, Ordering::SeqCst);
        self.idle_since_ms.store(now_ms, Ordering::SeqCst);
    }

    pub fn idle_for_ms(&self, now_ms: i64) -> i64 {
        let since = self.idle_since_ms.load(Ordering::SeqCst);
        if since == 0 {
            return 0;
        }
        now_ms - since
    }
}

pub fn should_activate(swap_used_pct: f64, mem_pressure_pct: f64, high: f64) -> bool {
    swap_used_pct >= high || mem_pressure_pct >= high
}

pub fn should_deactivate(swap_used_pct: f64, mem_pressure_pct: f64, low: f64) -> bool {
    swap_used_pct <= low && mem_pressure_pct <= low
}

pub fn idle_deadline_reached(idle_for_ms: i64, timeout_s: i64) -> bool {
    idle_for_ms >= timeout_s * 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_when_high() {
        assert!(should_activate(85.0, 30.0, 80.0));
        assert!(should_activate(30.0, 90.0, 80.0));
        assert!(!should_activate(30.0, 30.0, 80.0));
    }

    #[test]
    fn deactivate_only_when_both_low() {
        assert!(should_deactivate(20.0, 30.0, 40.0));
        assert!(!should_deactivate(20.0, 50.0, 40.0));
        assert!(!should_deactivate(60.0, 30.0, 40.0));
    }

    #[test]
    fn idle_deadline() {
        assert!(idle_deadline_reached(310_000, 300));
        assert!(!idle_deadline_reached(10_000, 300));
    }

    #[test]
    fn state_transitions() {
        let s = VmState::new();
        assert!(!s.is_active());
        s.activate();
        assert!(s.is_active());
        s.mark_idle(1000);
        assert!(!s.is_active());
        assert_eq!(s.idle_for_ms(2500), 1500);
    }
}
