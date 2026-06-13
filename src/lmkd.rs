use std::path::Path;
use std::time::Duration;
use crate::command::run_timeout;

pub fn apply(scripts_dir: &Path) -> std::io::Result<()> {
    let p = scripts_dir.join("lmkd.sh").to_string_lossy().to_string();
    run_timeout("sh", &[&p, "apply"], Duration::from_secs(5)).map(|_| ())
}
pub fn clear(scripts_dir: &Path) -> std::io::Result<()> {
    let p = scripts_dir.join("lmkd.sh").to_string_lossy().to_string();
    run_timeout("sh", &[&p, "clear"], Duration::from_secs(5)).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signature_compiles() { let _ = apply; let _ = clear; }
}
