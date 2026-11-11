//! Giới hạn tần suất: xô token. Toàn máy chủ dùng một xô (chỉ có một người dùng, một vài app).
use std::sync::Mutex;
use std::time::Instant;

pub struct TokenBucket {
    capacity: f64,
    per_sec: f64,
    state: Mutex<(f64, Instant)>,
}

impl TokenBucket {
    pub fn new(capacity: u32, per_sec: u32) -> Self {
        Self { capacity: capacity as f64, per_sec: per_sec as f64, state: Mutex::new((capacity as f64, Instant::now())) }
    }

    /// Lấy một token; `false` = hết, phải chờ.
    pub fn try_take(&self) -> bool {
        self.try_take_at(Instant::now())
    }

    pub(crate) fn try_take_at(&self, now: Instant) -> bool {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let dt = now.saturating_duration_since(s.1).as_secs_f64();
        s.0 = (s.0 + dt * self.per_sec).min(self.capacity);
        s.1 = now;
        if s.0 >= 1.0 {
            s.0 -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn burst_then_refill() {
        let b = TokenBucket::new(3, 2);
        let t0 = Instant::now();
        assert!(b.try_take_at(t0));
        assert!(b.try_take_at(t0));
        assert!(b.try_take_at(t0));
        assert!(!b.try_take_at(t0), "hết burst");
        assert!(!b.try_take_at(t0 + Duration::from_millis(400)), "0.8 token chưa đủ");
        assert!(b.try_take_at(t0 + Duration::from_millis(600)), "1.2 token");
        // không tích quá capacity
        let later = t0 + Duration::from_secs(100);
        for _ in 0..3 {
            assert!(b.try_take_at(later));
        }
        assert!(!b.try_take_at(later));
    }
}
