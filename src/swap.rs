use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use crate::command::run_timeout;

pub fn create_swap_file(path: &Path, size_mb: u32) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    file.set_len((size_mb as u64) * 1024 * 1024)
}

pub fn mkswap(path: &Path) -> io::Result<()> {
    let s = path.to_string_lossy();
    run_timeout("mkswap", &[s.as_ref()], Duration::from_secs(10))?;
    Ok(())
}

pub fn swapon(path: &Path) -> io::Result<()> {
    let s = path.to_string_lossy();
    run_timeout("swapon", &[s.as_ref()], Duration::from_secs(10))?;
    Ok(())
}

pub fn swapoff(path: &Path) -> io::Result<()> {
    let s = path.to_string_lossy();
    run_timeout("swapoff", &[s.as_ref()], Duration::from_secs(10))?;
    Ok(())
}

fn flag_name(path: &Path) -> String {
    let safe: String = path
        .to_string_lossy()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    safe
}

pub fn record(flags_dir: &Path, path: &Path) -> io::Result<()> {
    let dir = flags_dir.join("swap.d");
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join(format!("{}.swap", flag_name(path))),
        path.to_string_lossy().as_ref(),
    )
}

pub fn unrecord(flags_dir: &Path, path: &Path) -> io::Result<()> {
    let dir = flags_dir.join("swap.d");
    fs::remove_file(dir.join(format!("{}.swap", flag_name(path))))
}

pub fn record_zram(flags_dir: &Path, idx: u32) -> io::Result<()> {
    let dir = flags_dir.join("swap.d");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(format!("zram{}.zram", idx)), idx.to_string())
}

pub fn unrecord_zram(flags_dir: &Path, idx: u32) -> io::Result<()> {
    let dir = flags_dir.join("swap.d");
    fs::remove_file(dir.join(format!("zram{}.zram", idx)))
}

pub fn read_swap_usage_pct() -> Option<f64> {
    let content = std::fs::read_to_string("/proc/swaps").ok()?;
    let mut total_used: u64 = 0;
    let mut total_size: u64 = 0;
    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let (Ok(s), Ok(u)) = (parts[2].parse::<u64>(), parts[3].parse::<u64>()) {
                total_size += s;
                total_used += u;
            }
        }
    }
    if total_size == 0 {
        Some(0.0)
    } else {
        Some((total_used as f64) * 100.0 / (total_size as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn list(flags_dir: &Path) -> io::Result<Vec<PathBuf>> {
        let dir = flags_dir.join("swap.d");
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut paths = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".swap") {
                let content = fs::read_to_string(entry.path())?;
                paths.push(PathBuf::from(content.trim()));
            }
        }
        Ok(paths)
    }

    #[test]
    fn record_and_list_roundtrip() {
        let tmp = std::env::temp_dir().join("thrawl_swap_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);
        let p = PathBuf::from("/data/adb/thrawl/swap/swapfile0");
        record(&tmp, &p).unwrap();
        let paths = list(&tmp).unwrap();
        assert_eq!(paths, vec![p.clone()]);
        unrecord(&tmp, &p).unwrap();
        let paths2 = list(&tmp).unwrap();
        assert!(paths2.is_empty());
    }

    #[test]
    fn zram_record_roundtrip() {
        let tmp = std::env::temp_dir().join("thrawl_zram_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);
        record_zram(&tmp, 0).unwrap();
        record_zram(&tmp, 3).unwrap();
        let dir = tmp.join("swap.d");
        let entries: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".zram"))
            .map(|e| fs::read_to_string(e.path()).unwrap().trim().to_string())
            .collect();
        assert!(entries.contains(&"0".to_string()));
        assert!(entries.contains(&"3".to_string()));
        unrecord_zram(&tmp, 0).unwrap();
        unrecord_zram(&tmp, 3).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
