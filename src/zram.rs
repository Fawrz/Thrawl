use std::fs;
use std::io;

pub fn sys_root() -> &'static str {
    "/sys/block"
}

#[cfg(target_os = "linux")]
pub fn list_devices() -> Vec<String> {
    let root = std::path::Path::new(sys_root());
    let mut devices = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with("zram") {
                    devices.push(name);
                }
            }
        }
    }
    devices
}

#[cfg(not(target_os = "linux"))]
pub fn list_devices() -> Vec<String> {
    Vec::new()
}

fn zram_path(idx: u32) -> String {
    format!("{}/zram{}", sys_root(), idx)
}

#[cfg(target_os = "linux")]
pub fn hot_add() -> io::Result<()> {
    fs::write("/sys/class/zram-control/hot_add", b"1")?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn hot_add() -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "ZRAM not supported"))
}

#[cfg(target_os = "linux")]
pub fn hot_remove(idx: u32) -> io::Result<()> {
    fs::write("/sys/class/zram-control/hot_remove", idx.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn hot_remove(_idx: u32) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "ZRAM not supported"))
}

#[cfg(target_os = "linux")]
pub fn set_disksize(idx: u32, bytes: u64) -> io::Result<()> {
    let path = format!("{}/disksize", zram_path(idx));
    fs::write(&path, bytes.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn set_disksize(_idx: u32, _bytes: u64) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "ZRAM not supported"))
}

#[cfg(target_os = "linux")]
pub fn set_comp_algo(idx: u32, algo: &str) -> io::Result<()> {
    let recomp_path = format!("{}/recomp_algorithm", zram_path(idx));
    let content = fs::read_to_string(&recomp_path)?;
    let first = content.split_whitespace().next().unwrap_or("");
    if first != algo {
        let comp_path = format!("{}/comp_algorithm", zram_path(idx));
        fs::write(&comp_path, algo)?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn set_comp_algo(_idx: u32, _algo: &str) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "ZRAM not supported"))
}

#[cfg(target_os = "linux")]
pub fn reset(idx: u32) -> io::Result<()> {
    let path = format!("{}/reset", zram_path(idx));
    fs::write(&path, b"1")?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn reset(_idx: u32) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "ZRAM not supported"))
}

pub fn auto_size_bytes(total_ram_kb: u64) -> u64 {
    let bytes = total_ram_kb * 1024 / 4;
    bytes.clamp(512 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_safe_on_host() {
        let _ = list_devices();
    }

    #[test]
    fn auto_size_within_bounds() {
        let s = auto_size_bytes(2 * 1024 * 1024);
        assert!(s >= 512 * 1024 * 1024);
        assert!(s <= 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn sys_root_path() {
        assert_eq!(sys_root(), "/sys/block");
    }
}
