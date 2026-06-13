use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

pub const PSI_MEMORY: &str = "/proc/pressure/memory";

pub fn is_available() -> bool {
    Path::new(PSI_MEMORY).exists()
}

pub fn read_avg60() -> std::io::Result<f64> {
    let mut s = String::new();
    File::open(PSI_MEMORY)?.read_to_string(&mut s)?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("some ") {
            for part in rest.split_whitespace() {
                if let Some(v) = part.strip_prefix("avg60=") {
                    return v
                        .parse::<f64>()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e));
                }
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "no avg60 found in PSI output",
    ))
}

#[cfg(unix)]
pub fn wait_event(fd: i32, timeout: Duration) -> std::io::Result<bool> {
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLPRI,
        revents: 0,
    };
    let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
    let n = unsafe { libc::poll(&mut pfd, 1, ms) };
    if n < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(n > 0 && (pfd.revents & libc::POLLPRI) != 0)
}

#[cfg(not(unix))]
pub fn wait_event(_fd: i32, _timeout: Duration) -> std::io::Result<bool> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "PSI wait_event is not supported on this platform",
    ))
}

pub fn open_psi() -> std::io::Result<File> {
    File::open(PSI_MEMORY)
}

pub fn rewind(file: &mut File) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_check_does_not_panic() {
        let _ = is_available();
    }

    #[test]
    fn read_avg60_when_present() {
        if !Path::new(PSI_MEMORY).exists() {
            return;
        }
        let v = read_avg60();
        assert!(v.is_ok());
    }

    #[test]
    fn path_constant() {
        assert_eq!(PSI_MEMORY, "/proc/pressure/memory");
    }
}
