use std::io;

fn read_from_any(paths: &[&str]) -> io::Result<String> {
    for p in paths {
        if let Ok(s) = std::fs::read_to_string(p) {
            return Ok(s);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no zram-control found",
    ))
}

pub fn hot_add() -> io::Result<u32> {
    let content = read_from_any(&[
        "/sys/class/zram-control/hot_add",
        "/sys/devices/virtual/misc/zram-control/hot_add",
    ])?;
    let id: u32 = content
        .trim()
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("bad zram id: {}", e)))?;
    Ok(id)
}

pub fn hot_remove(idx: u32) -> io::Result<()> {
    let content = idx.to_string();
    let paths = [
        "/sys/class/zram-control/hot_remove",
        "/sys/devices/virtual/misc/zram-control/hot_remove",
    ];
    for p in &paths {
        if std::fs::write(p, &content).is_ok() {
            return Ok(());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no zram-control found",
    ))
}

fn zram_path(idx: u32) -> String {
    format!("/sys/block/zram{}", idx)
}

pub fn set_disksize(idx: u32, bytes: u64) -> io::Result<()> {
    let path = format!("{}/disksize", zram_path(idx));
    std::fs::write(&path, bytes.to_string())
}

pub fn set_comp_algo(idx: u32, algo: &str) -> io::Result<()> {
    let recomp_path = format!("{}/recomp_algorithm", zram_path(idx));
    if let Ok(content) = std::fs::read_to_string(&recomp_path) {
        let first = content.split_whitespace().next().unwrap_or("");
        if first == algo {
            return Ok(());
        }
    }
    let comp_path = format!("{}/comp_algorithm", zram_path(idx));
    std::fs::write(&comp_path, algo)
}

pub fn auto_size_bytes(total_ram_kb: u64) -> u64 {
    let bytes = total_ram_kb * 1024 / 4;
    bytes.clamp(512 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_size_within_bounds() {
        let s = auto_size_bytes(2 * 1024 * 1024);
        assert!(s >= 512 * 1024 * 1024);
        assert!(s <= 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn auto_size_small_ram() {
        let s = auto_size_bytes(512 * 1024);
        assert_eq!(s, 512 * 1024 * 1024);
    }

    #[test]
    fn auto_size_large_ram() {
        let s = auto_size_bytes(16 * 1024 * 1024);
        assert_eq!(s, 4 * 1024 * 1024 * 1024);
    }
}
