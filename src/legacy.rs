use std::fs;

pub struct MemInfo {
    pub total_kb: u64,
    pub available_kb: u64,
}

pub fn read_meminfo() -> std::io::Result<MemInfo> {
    let s = fs::read_to_string("/proc/meminfo")?;
    let mut total = 0u64;
    let mut avail = 0u64;
    for line in s.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total = v
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail = v
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        }
    }
    if total == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "MemTotal not found",
        ));
    }
    Ok(MemInfo {
        total_kb: total,
        available_kb: avail,
    })
}

pub fn used_percent(info: &MemInfo) -> f64 {
    if info.total_kb == 0 {
        return 0.0;
    }
    let used = info.total_kb.saturating_sub(info.available_kb);
    (used as f64) * 100.0 / (info.total_kb as f64)
}

#[derive(Debug, PartialEq)]
pub enum HysteresisOp {
    Raise,
    Lower,
    Hold,
}

pub fn decide(
    current: i64,
    used: f64,
    threshold: f64,
    hysteresis: f64,
    low: i64,
    high: i64,
) -> HysteresisOp {
    if used >= threshold {
        return HysteresisOp::Raise;
    }
    if used <= threshold - hysteresis {
        return HysteresisOp::Lower;
    }
    let _ = (current, low, high);
    HysteresisOp::Hold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn used_percent_basic() {
        let m = MemInfo {
            total_kb: 1000,
            available_kb: 250,
        };
        assert!((used_percent(&m) - 75.0).abs() < 1e-6);
    }

    #[test]
    fn hysteresis_raises() {
        let op = decide(40, 90.0, 65.0, 10.0, 40, 120);
        assert_eq!(op, HysteresisOp::Raise);
    }

    #[test]
    fn hysteresis_lowers() {
        let op = decide(120, 30.0, 65.0, 10.0, 40, 120);
        assert_eq!(op, HysteresisOp::Lower);
    }

    #[test]
    fn hysteresis_holds() {
        let op = decide(80, 60.0, 65.0, 10.0, 40, 120);
        assert_eq!(op, HysteresisOp::Hold);
    }
}
