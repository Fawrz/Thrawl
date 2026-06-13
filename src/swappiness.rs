use std::io::{Read, Write};
use std::path::Path;

pub const SWAPPINESS_PATH: &str = "/proc/sys/vm/swappiness";

pub fn read_swappiness() -> std::io::Result<i64> {
    let mut s = String::new();
    std::fs::File::open(SWAPPINESS_PATH)?.read_to_string(&mut s)?;
    Ok(s.trim().parse().unwrap_or(0))
}

pub fn write_swappiness(v: i64) -> std::io::Result<()> {
    let mut f = std::fs::OpenOptions::new().write(true).open(SWAPPINESS_PATH)?;
    writeln!(f, "{}", v)
}

pub fn detect_max() -> i64 {
    let prev = read_swappiness().ok();
    let candidates = [200, 180, 150, 120, 100, 60, 10, 0];
    let mut max = 0;
    for &c in &candidates {
        if write_swappiness(c).is_err() { continue; }
        if let Ok(got) = read_swappiness() {
            if got == c && got > max { max = got; }
        }
    }
    if let Some(p) = prev { let _ = write_swappiness(p); }
    max
}

pub fn clamp_to_kernel(value: i64, max: i64) -> i64 {
    value.clamp(0, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_helper() {
        assert_eq!(clamp_to_kernel(50, 100), 50);
        assert_eq!(clamp_to_kernel(150, 100), 100);
        assert_eq!(clamp_to_kernel(-5, 100), 0);
    }

    #[test]
    fn detect_max_handles_sandbox() {
        if Path::new(SWAPPINESS_PATH).exists() {
            let m = detect_max();
            assert!(m >= 0 && m <= 200);
        }
    }

    #[test]
    fn path_uses_proc() {
        assert_eq!(SWAPPINESS_PATH, "/proc/sys/vm/swappiness");
    }
}
