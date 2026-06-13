use std::path::Path;
use std::time::Duration;
use crate::command::run_timeout;

pub fn start(scripts_dir: &Path) -> std::io::Result<()> {
    let p = scripts_dir.join("logging.sh").to_string_lossy().to_string();
    run_timeout("sh", &[&p, "start"], Duration::from_secs(5)).map(|_| ())
}
pub fn stop(scripts_dir: &Path) -> std::io::Result<()> {
    let p = scripts_dir.join("logging.sh").to_string_lossy().to_string();
    run_timeout("sh", &[&p, "stop"], Duration::from_secs(5)).map(|_| ())
}
pub fn restart(scripts_dir: &Path) -> std::io::Result<()> {
    let p = scripts_dir.join("logging.sh").to_string_lossy().to_string();
    run_timeout("sh", &[&p, "restart"], Duration::from_secs(5)).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signature_compiles() { let _ = start; let _ = stop; let _ = restart; }
}
