use std::path::Path;
use std::fs;

#[cfg(unix)]
fn process_exists(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(windows)]
fn process_exists(pid: i32) -> bool {
    extern "system" {
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
        fn CloseHandle(hObject: isize) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32) };
    if handle == 0 {
        return false;
    }
    unsafe { CloseHandle(handle) };
    true
}

pub fn check_and_write_pid(moddir: &Path) -> std::io::Result<()> {
    let dir = moddir.join("data/flags");
    fs::create_dir_all(&dir)?;
    let pid_path = dir.join("thrawld.pid");
    if let Ok(body) = fs::read_to_string(&pid_path) {
        if let Ok(existing) = body.trim().parse::<i32>() {
            if existing > 0 && process_exists(existing) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!("daemon already running (pid={})", existing),
                ));
            }
        }
    }
    let _ = fs::remove_file(&pid_path);
    fs::write(&pid_path, std::process::id().to_string())?;
    Ok(())
}

pub fn remove_pid(moddir: &Path) {
    let _ = fs::remove_file(moddir.join("data/flags/thrawld.pid"));
}
