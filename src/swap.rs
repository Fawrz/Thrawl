use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::command::run_timeout;

pub const SWAP_FLAG_DIR: &str = "flags/swap.d";

pub fn ensure_flag_dir(flags_dir: &Path) -> io::Result<()> {
    let dir = flags_dir.join("swap.d");
    fs::create_dir_all(&dir)
}

fn path_to_flag_name(path: &Path) -> String {
    let s = path.to_string_lossy();
    let safe: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    safe + ".swap"
}

pub fn create_swap_file(path: &Path, size_mb: u32) -> io::Result<()> {
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

pub fn record(flags_dir: &Path, path: &Path) -> io::Result<()> {
    let dir = flags_dir.join("swap.d");
    let flag_path = dir.join(path_to_flag_name(path));
    fs::write(&flag_path, path.to_string_lossy().as_ref())
}

pub fn list(flags_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let dir = flags_dir.join("swap.d");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let content = fs::read_to_string(entry.path())?;
        paths.push(PathBuf::from(content.trim()));
    }
    Ok(paths)
}

pub fn unrecord(flags_dir: &Path, path: &Path) -> io::Result<()> {
    let dir = flags_dir.join("swap.d");
    let flag_path = dir.join(path_to_flag_name(path));
    fs::remove_file(&flag_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_list_roundtrip() {
        let tmp = std::env::temp_dir().join("thrawl_swap_test");
        let _ = std::fs::remove_dir_all(&tmp);
        ensure_flag_dir(&tmp).unwrap();
        let p = PathBuf::from("/data/adb/thrawl/swap/swapfile0");
        record(&tmp, &p).unwrap();
        let paths = list(&tmp).unwrap();
        assert_eq!(paths, vec![p.clone()]);
        unrecord(&tmp, &p).unwrap();
        let paths2 = list(&tmp).unwrap();
        assert!(paths2.is_empty());
    }
}
