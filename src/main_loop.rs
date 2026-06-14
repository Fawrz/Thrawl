use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use thrawld::config::{self, ConfigValue};

fn daemon_reload_flag() -> bool {
    RELOAD_FLAG.swap(false, Ordering::SeqCst)
}

fn daemon_shutdown_flag() -> bool {
    SHUTDOWN_FLAG.load(Ordering::SeqCst)
}

fn write_backend_flags(flags_dir: &Path, psi_available: bool) -> std::io::Result<()> {
    std::fs::write(
        flags_dir.join("psi_available"),
        if psi_available { "1" } else { "0" },
    )?;
    std::fs::write(
        flags_dir.join("swappiness_backend"),
        if psi_available { "psi" } else { "legacy" },
    )?;
    Ok(())
}

fn poll_interval(cfg: &HashMap<String, ConfigValue>, key: &str, default_ms: u64) -> Duration {
    Duration::from_millis(
        cfg.get(key)
            .and_then(|v| v.as_int())
            .unwrap_or(default_ms as i64) as u64,
    )
}

pub fn run_daemon(moddir: &Path, cfg_path: &Path, effective_path: &Path) -> std::io::Result<()> {
    let flags_dir = moddir.join("data/flags");
    let scripts_dir = moddir.join("scripts");

    std::fs::create_dir_all(&flags_dir)?;
    std::fs::create_dir_all(&scripts_dir)?;

    let mut current_cfg = resolve_config(cfg_path, effective_path, flags_dir.as_path())?;
    let mut backend_is_psi = thrawld::psi::is_available();
    let mut psi_handle = if backend_is_psi {
        thrawld::psi::open_psi().ok()
    } else {
        None
    };
    write_backend_flags(&flags_dir, backend_is_psi)?;
    std::fs::write(flags_dir.join("vm_controller"), b"idle")?;

    apply_helpers(&current_cfg, &scripts_dir);

    let swappiness_max = thrawld::swappiness::detect_max();
    let mut last_applied_swappiness = thrawld::swappiness::read_swappiness().ok();
    let mut config_poll = poll_interval(&current_cfg, "CONFIG_POLL_INTERVAL_MS", 5000);
    let mut psi_poll = poll_interval(&current_cfg, "PSI_POLL_TIMEOUT_MS", 5000);
    let mut legacy_poll = poll_interval(&current_cfg, "LEGACY_POLL_INTERVAL_MS", 5000);

    let mut last_cfg_mtime = get_mtime(cfg_path);

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
            apply_helpers(&next_cfg, &scripts_dir);
            current_cfg = next_cfg;
            config_poll = poll_interval(&current_cfg, "CONFIG_POLL_INTERVAL_MS", 5000);
            psi_poll = poll_interval(&current_cfg, "PSI_POLL_TIMEOUT_MS", 5000);
            legacy_poll = poll_interval(&current_cfg, "LEGACY_POLL_INTERVAL_MS", 5000);
        }

        let psi_available = thrawld::psi::is_available();
        if psi_available != backend_is_psi {
            backend_is_psi = psi_available;
            write_backend_flags(&flags_dir, backend_is_psi)?;
            if backend_is_psi {
                psi_handle = thrawld::psi::open_psi().ok();
            } else {
                psi_handle = None;
            }
        } else if backend_is_psi && psi_handle.is_none() {
            psi_handle = thrawld::psi::open_psi().ok();
        }

        let low = current_cfg
            .get("SWAPPINESS_LOW")
            .and_then(|v| v.as_int())
            .unwrap_or(40);
        let high = current_cfg
            .get("SWAPPINESS_HIGH")
            .and_then(|v| v.as_int())
            .unwrap_or(120);

        if backend_is_psi {
            let threshold = current_cfg
                .get("PSI_THRESHOLD")
                .and_then(|v| v.as_int())
                .unwrap_or(70) as f64;
            let pressure = thrawld::psi::read_avg60().unwrap_or(0.0);
            let target = if pressure * 100.0 >= threshold {
                high
            } else {
                low
            };
            let clamped = thrawld::swappiness::clamp_to_kernel(target, swappiness_max);
            if last_applied_swappiness != Some(clamped) {
                let _ = thrawld::swappiness::write_swappiness(clamped);
                last_applied_swappiness = Some(clamped);
            }

            let wait = std::cmp::min(config_poll, psi_poll);
            if let Some(ref handle) = psi_handle {
                #[cfg(unix)]
                {
                    use std::os::unix::io::AsRawFd;
                    let _ = thrawld::psi::wait_event(handle.as_raw_fd(), wait);
                }
                #[cfg(not(unix))]
                {
                    std::thread::sleep(wait);
                }
            } else {
                std::thread::sleep(wait);
            }
        } else {
            let threshold = current_cfg
                .get("LEGACY_PRESSURE_THRESHOLD")
                .and_then(|v| v.as_int())
                .unwrap_or(65) as f64;
            let hysteresis = current_cfg
                .get("LEGACY_HYSTERESIS")
                .and_then(|v| v.as_int())
                .unwrap_or(10) as f64;
            use thrawld::legacy::{decide, read_meminfo, used_percent, HysteresisOp};
            if let Ok(mem) = read_meminfo() {
                let used = used_percent(&mem);
                let current_val = thrawld::swappiness::read_swappiness().unwrap_or(low);
                let op = decide(current_val, used, threshold, hysteresis, low, high);
                let target = match op {
                    HysteresisOp::Raise => high,
                    HysteresisOp::Lower => low,
                    HysteresisOp::Hold => current_val,
                };
                let clamped = thrawld::swappiness::clamp_to_kernel(target, swappiness_max);
                if last_applied_swappiness != Some(clamped) {
                    let _ = thrawld::swappiness::write_swappiness(clamped);
                    last_applied_swappiness = Some(clamped);
                }
            }

            std::thread::sleep(std::cmp::min(config_poll, legacy_poll));
        }
    }

    Ok(())
}

fn resolve_config(
    cfg_path: &Path,
    effective_path: &Path,
    _flags_dir: &Path,
) -> std::io::Result<HashMap<String, ConfigValue>> {
    let mut current = config::defaults();
    if cfg_path.exists() {
        let body = std::fs::read_to_string(cfg_path).unwrap_or_default();
        let parsed = config::parse(&body);
        for (k, v) in parsed {
            current.insert(k, v);
        }
    }
    let psi_avail = thrawld::psi::is_available();
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
    let _ = thrawld::lmkd::apply(scripts_dir);

    let uffd_on = cfg
        .get("UFFD_GC_ENABLE")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if uffd_on {
        let _ = thrawld::uffd::apply(scripts_dir);
    } else {
        let _ = thrawld::uffd::clear(scripts_dir);
    }

    let log_on = cfg
        .get("LOGGING_ENABLE")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if log_on {
        let _ = thrawld::logging::start(scripts_dir);
    } else {
        let _ = thrawld::logging::stop(scripts_dir);
    }
}
