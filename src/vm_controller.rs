use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use crate::config::ConfigValue;
use crate::{legacy, swap, zram};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VmState {
    Idle,
    Active,
    ActiveNoPressure,
}

pub struct VmController {
    pub state: VmState,
    owned_zram: Vec<u32>,
    swap_path: Option<std::path::PathBuf>,
    swap_active: bool,
    idle_since: Option<Instant>,
}

impl Default for VmController {
    fn default() -> Self {
        Self::new()
    }
}

impl VmController {
    pub fn new() -> Self {
        Self {
            state: VmState::Idle,
            owned_zram: Vec::new(),
            swap_path: None,
            swap_active: false,
            idle_since: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        pressure_pct: f64,
        swap_used_pct: f64,
        high: f64,
        low: f64,
        idle_timeout_s: i64,
        cfg: &HashMap<String, ConfigValue>,
        flags_dir: &Path,
    ) -> VmState {
        let zram_enable = cfg
            .get("ZRAM_ENABLE")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let zram_count = cfg.get("ZRAM_COUNT").and_then(|v| v.as_int()).unwrap_or(1) as u32;
        let swap_enable = cfg
            .get("SWAP_ENABLE")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let swap_size = cfg
            .get("SWAP_SIZE_MB")
            .and_then(|v| v.as_int())
            .unwrap_or(0) as u32;

        if should_activate(swap_used_pct, pressure_pct, high) {
            self.state = VmState::Active;
            self.idle_since = None;

            if zram_enable && (self.owned_zram.len() as u32) < zram_count {
                let _ = self.activate_next_zram(cfg, flags_dir);
            }

            if swap_enable && swap_size > 0 && !self.swap_active {
                let _ = self.activate_swap(cfg, flags_dir);
            }
        } else if should_deactivate(swap_used_pct, pressure_pct, low) {
            match self.state {
                VmState::Active => {
                    self.state = VmState::ActiveNoPressure;
                    self.idle_since = Some(Instant::now());
                }
                VmState::ActiveNoPressure => {
                    if let Some(since) = self.idle_since {
                        let elapsed = since.elapsed().as_secs();
                        if elapsed >= idle_timeout_s as u64 {
                            let _ = self.deactivate_swap(flags_dir);
                            let _ = self.deactivate_oldest_zram(flags_dir);
                            self.state = VmState::Idle;
                            self.idle_since = None;
                        }
                    }
                }
                VmState::Idle => {}
            }
        } else {
            self.idle_since = None;
            if self.state == VmState::ActiveNoPressure {
                self.state = VmState::Active;
            }
        }

        self.state
    }

    fn activate_next_zram(
        &mut self,
        cfg: &HashMap<String, ConfigValue>,
        flags_dir: &Path,
    ) -> std::io::Result<()> {
        let idx = zram::hot_add()?;
        let algo = cfg
            .get("ZRAM_COMP_ALGO")
            .and_then(|v| v.as_str())
            .unwrap_or("zstd");
        let size_mb = cfg
            .get("ZRAM_SIZE_MB")
            .and_then(|v| v.as_int())
            .unwrap_or(0);
        let bytes = if size_mb > 0 {
            (size_mb as u64) * 1024 * 1024
        } else {
            let total_kb = legacy::read_meminfo()
                .map(|m| m.total_kb)
                .unwrap_or(2 * 1024 * 1024);
            zram::auto_size_bytes(total_kb)
        };

        zram::set_disksize(idx, bytes)?;
        zram::set_comp_algo(idx, algo)?;
        swap::record_zram(flags_dir, idx)?;

        self.owned_zram.push(idx);
        Ok(())
    }

    fn deactivate_oldest_zram(&mut self, flags_dir: &Path) -> std::io::Result<()> {
        if let Some(idx) = self.owned_zram.first().copied() {
            let _ = swap::swapoff(&std::path::PathBuf::from(format!("/dev/block/zram{}", idx)));
            zram::hot_remove(idx)?;
            swap::unrecord_zram(flags_dir, idx)?;
            self.owned_zram.retain(|&i| i != idx);
        }
        Ok(())
    }

    fn activate_swap(
        &mut self,
        cfg: &HashMap<String, ConfigValue>,
        flags_dir: &Path,
    ) -> std::io::Result<()> {
        let size_mb = cfg
            .get("SWAP_SIZE_MB")
            .and_then(|v| v.as_int())
            .unwrap_or(0) as u32;
        let path_str = cfg
            .get("SWAP_PATH")
            .and_then(|v| v.as_str())
            .unwrap_or("/data/adb/thrawl/swap");
        let path = std::path::PathBuf::from(format!("{}/swapfile0", path_str));

        swap::create_swap_file(&path, size_mb)?;
        swap::mkswap(&path)?;
        swap::swapon(&path)?;
        swap::record(flags_dir, &path)?;

        self.swap_path = Some(path);
        self.swap_active = true;
        Ok(())
    }

    fn deactivate_swap(&mut self, flags_dir: &Path) -> std::io::Result<()> {
        if let Some(ref path) = self.swap_path {
            let _ = swap::swapoff(path);
            swap::unrecord(flags_dir, path)?;
            let _ = std::fs::remove_file(path);
        }
        self.swap_path = None;
        self.swap_active = false;
        Ok(())
    }

    pub fn reconcile(
        &mut self,
        old_cfg: &HashMap<String, ConfigValue>,
        new_cfg: &HashMap<String, ConfigValue>,
        flags_dir: &Path,
    ) {
        let zram_old = old_cfg
            .get("ZRAM_ENABLE")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let zram_new = new_cfg
            .get("ZRAM_ENABLE")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if zram_old && !zram_new {
            while !self.owned_zram.is_empty() {
                let _ = self.deactivate_oldest_zram(flags_dir);
            }
        }

        let count_new = new_cfg
            .get("ZRAM_COUNT")
            .and_then(|v| v.as_int())
            .unwrap_or(1) as u32;
        while (self.owned_zram.len() as u32) > count_new {
            let _ = self.deactivate_oldest_zram(flags_dir);
        }

        let swap_new = new_cfg
            .get("SWAP_ENABLE")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let size_new = new_cfg
            .get("SWAP_SIZE_MB")
            .and_then(|v| v.as_int())
            .unwrap_or(0);
        if self.swap_active && (!swap_new || size_new == 0) {
            let _ = self.deactivate_swap(flags_dir);
        }
    }
}

pub fn should_activate(swap_used_pct: f64, mem_pressure_pct: f64, high: f64) -> bool {
    swap_used_pct >= high || mem_pressure_pct >= high
}

pub fn should_deactivate(swap_used_pct: f64, mem_pressure_pct: f64, low: f64) -> bool {
    swap_used_pct <= low && mem_pressure_pct <= low
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle_deadline_reached(idle_for_ms: i64, timeout_s: i64) -> bool {
        idle_for_ms >= timeout_s * 1000
    }

    #[test]
    fn activate_when_high() {
        assert!(should_activate(85.0, 30.0, 80.0));
        assert!(should_activate(30.0, 90.0, 80.0));
        assert!(!should_activate(30.0, 30.0, 80.0));
    }

    #[test]
    fn deactivate_only_when_both_low() {
        assert!(should_deactivate(20.0, 30.0, 40.0));
        assert!(!should_deactivate(20.0, 50.0, 40.0));
        assert!(!should_deactivate(60.0, 30.0, 40.0));
    }

    #[test]
    fn idle_deadline() {
        assert!(idle_deadline_reached(310_000, 300));
        assert!(!idle_deadline_reached(10_000, 300));
    }

    #[test]
    fn vm_state_initial_is_idle() {
        let vm = VmController::new();
        assert_eq!(vm.state, VmState::Idle);
    }
}
