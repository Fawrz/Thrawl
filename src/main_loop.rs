use std::path::Path;
use std::time::Duration;
use std::collections::HashMap;

use chimerad::config::{self, ConfigValue};

fn daemon_reload_flag() -> bool {
    RELOAD_FLAG.swap(false, Ordering::SeqCst)
}

fn daemon_shutdown_flag() -> bool {
    SHUTDOWN_FLAG.load(Ordering::SeqCst)
}

pub fn run_daemon(moddir: &Path, cfg_path: &Path, effective_path: &Path) -> std::io::Result<()> {
    let flags_dir = moddir.join("data/flags");
    let scripts_dir = moddir.join("scripts");

    std::fs::create_dir_all(&flags_dir)?;
    std::fs::create_dir_all(&scripts_dir)?;

    let current = resolve_config(cfg_path, effective_path, flags_dir.as_path())?;
    let psi_avail = chimerad::psi::is_available();
    let backend = if psi_avail { "psi" } else { "legacy" };
    std::fs::write(flags_dir.join("psi_available"), if psi_avail { b"1" } else { b"0" })?;
    std::fs::write(flags_dir.join("swappiness_backend"), backend.as_bytes())?;
    std::fs::write(flags_dir.join("vm_controller"), b"idle")?;

    apply_helpers(&current, &scripts_dir);

    let swappiness_max = chimerad::swappiness::detect_max();
    let cfg_poll = Duration::from_millis(
        current.get("CONFIG_POLL_INTERVAL_MS")
            .and_then(|v| v.as_int())
            .unwrap_or(5000) as u64,
    );

    let mut last_cfg_mtime = get_mtime(cfg_path);
    let mut current_cfg = current;

    loop {
        if daemon_shutdown_flag() {
            break;
        }

        let reload = daemon_reload_flag();
        let mtime = get_mtime(cfg_path);
        let mtime_changed = match (&last_cfg_mtime, &mtime) {
            (Ok(a), Ok(b)) => a != b,
            _ => false,
        };
        if reload || mtime_changed {
            last_cfg_mtime = mtime;
            let next_cfg = resolve_config(cfg_path, effective_path, flags_dir.as_path())
                .unwrap_or_else(|_| current_cfg.clone());
            let new_psi = chimerad::psi::is_available();
            let new_backend = if new_psi { "psi" } else { "legacy" };
            std::fs::write(flags_dir.join("swappiness_backend"), new_backend.as_bytes())?;
            apply_helpers(&next_cfg, &scripts_dir);
            current_cfg = next_cfg;
        }

        let low = current_cfg.get("SWAPPINESS_LOW").and_then(|v| v.as_int()).unwrap_or(40);
        let high = current_cfg.get("SWAPPINESS_HIGH").and_then(|v| v.as_int()).unwrap_or(120);

        if chimerad::psi::is_available() {
            let threshold = current_cfg.get("PSI_THRESHOLD").and_then(|v| v.as_int()).unwrap_or(70) as f64;
            let pressure = chimerad::psi::read_avg60().unwrap_or(0.0);
            let target = if pressure * 100.0 >= threshold { high } else { low };
            let clamped = chimerad::swappiness::clamp_to_kernel(target, swappiness_max);
            let _ = chimerad::swappiness::write_swappiness(clamped);
        } else {
            let threshold = current_cfg.get("LEGACY_PRESSURE_THRESHOLD").and_then(|v| v.as_int()).unwrap_or(65) as f64;
            let hysteresis = current_cfg.get("LEGACY_HYSTERESIS").and_then(|v| v.as_int()).unwrap_or(10) as f64;
            use chimerad::legacy::{read_meminfo, used_percent, decide, HysteresisOp};
            if let Ok(mem) = read_meminfo() {
                let used = used_percent(&mem);
                let current_val = chimerad::swappiness::read_swappiness().unwrap_or(low);
                let op = decide(current_val, used, threshold, hysteresis, low, high);
                let target = match op {
                    HysteresisOp::Raise => high,
                    HysteresisOp::Lower => low,
                    HysteresisOp::Hold => current_val,
                };
                let clamped = chimerad::swappiness::clamp_to_kernel(target, swappiness_max);
                let _ = chimerad::swappiness::write_swappiness(clamped);
            }
        }

        std::thread::sleep(cfg_poll);
    }

    Ok(())
}

fn resolve_config(cfg_path: &Path, effective_path: &Path, _flags_dir: &Path) -> std::io::Result<HashMap<String, ConfigValue>> {
    let mut current = config::defaults();
    if cfg_path.exists() {
        let body = std::fs::read_to_string(cfg_path).unwrap_or_default();
        let parsed = config::parse(&body);
        for (k, v) in parsed {
            current.insert(k, v);
        }
    }
    let psi_avail = chimerad::psi::is_available();
    if let Some(ConfigValue::AutoResolved(ref mut b)) = current.get_mut("PSI_AVAILABLE") {
        *b = psi_avail;
    }
    if let Some(ConfigValue::AutoResolved(ref mut b)) = current.get_mut("LMKD_USE_PSI") {
        *b = psi_avail;
    }
    if let Some(ConfigValue::AutoResolved(ref mut b)) = current.get_mut("LMKD_USE_MINFREE") {
        *b = !psi_avail;
    }
    let _ = config::write_effective(effective_path, &current);
    Ok(current)
}

fn get_mtime(path: &Path) -> std::io::Result<std::time::SystemTime> {
    std::fs::metadata(path)?.modified()
}

fn apply_helpers(cfg: &HashMap<String, ConfigValue>, scripts_dir: &Path) {
    let _ = chimerad::lmkd::apply(scripts_dir);

    let uffd_on = cfg.get("UFFD_GC_ENABLE").and_then(|v| v.as_bool()).unwrap_or(false);
    if uffd_on {
        let _ = chimerad::uffd::apply(scripts_dir);
    } else {
        let _ = chimerad::uffd::clear(scripts_dir);
    }

    let log_on = cfg.get("LOGGING_ENABLE").and_then(|v| v.as_bool()).unwrap_or(true);
    if log_on {
        let _ = chimerad::logging::start(scripts_dir);
    } else {
        let _ = chimerad::logging::stop(scripts_dir);
    }
}
