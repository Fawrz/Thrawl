use std::process::Command;
use std::time::Duration;

pub fn run_timeout(cmd: &str, args: &[&str], timeout: Duration) -> std::io::Result<i32> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(status.code().unwrap_or(-1)),
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "command timed out",
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_short_command() {
        let r = run_timeout("true", &[], Duration::from_secs(2));
        assert!(r.is_ok());
    }

    #[test]
    fn times_out_slow_command() {
        let r = run_timeout("sleep", &["5"], Duration::from_millis(200));
        assert!(r.is_err());
    }
}
