use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub enum ValueKind {
    Bool,
    Auto,
    Int(i64, i64),
    String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Bool(bool),
    AutoResolved(bool),
    Int(i64),
    String(String),
}

impl ConfigValue {
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            ConfigValue::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match *self {
            ConfigValue::Int(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_auto(&self) -> Option<bool> {
        match *self {
            ConfigValue::AutoResolved(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(ref s) => Some(s.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigKey {
    pub name: &'static str,
    pub kind: ValueKind,
    pub default: ConfigValue,
}

pub static KEYS: LazyLock<Vec<ConfigKey>> = LazyLock::new(|| vec![
    ConfigKey { name: "PSI_AVAILABLE", kind: ValueKind::Auto, default: ConfigValue::AutoResolved(false) },
    ConfigKey { name: "PSI_THRESHOLD", kind: ValueKind::Int(0, 100), default: ConfigValue::Int(70) },
    ConfigKey { name: "PSI_POLL_TIMEOUT_MS", kind: ValueKind::Int(100, 60000), default: ConfigValue::Int(5000) },
    ConfigKey { name: "SWAPPINESS_LOW", kind: ValueKind::Int(0, 200), default: ConfigValue::Int(40) },
    ConfigKey { name: "SWAPPINESS_HIGH", kind: ValueKind::Int(0, 200), default: ConfigValue::Int(120) },
    ConfigKey { name: "LEGACY_PRESSURE_THRESHOLD", kind: ValueKind::Int(0, 100), default: ConfigValue::Int(65) },
    ConfigKey { name: "LEGACY_HYSTERESIS", kind: ValueKind::Int(0, 100), default: ConfigValue::Int(10) },
    ConfigKey { name: "LEGACY_POLL_INTERVAL_MS", kind: ValueKind::Int(100, 60000), default: ConfigValue::Int(5000) },
    ConfigKey { name: "ZRAM_ENABLE", kind: ValueKind::Bool, default: ConfigValue::Bool(true) },
    ConfigKey { name: "ZRAM_COUNT", kind: ValueKind::Int(0, 32), default: ConfigValue::Int(4) },
    ConfigKey { name: "ZRAM_SIZE_MB", kind: ValueKind::Int(0, 65536), default: ConfigValue::Int(0) },
    ConfigKey { name: "ZRAM_COMP_ALGO", kind: ValueKind::String, default: ConfigValue::String("zstd".to_string()) },
    ConfigKey { name: "SWAP_ENABLE", kind: ValueKind::Bool, default: ConfigValue::Bool(true) },
    ConfigKey { name: "SWAP_SIZE_MB", kind: ValueKind::Int(0, 65536), default: ConfigValue::Int(0) },
    ConfigKey { name: "SWAP_PATH", kind: ValueKind::String, default: ConfigValue::String("/data/adb/chimera/swap".to_string()) },
    ConfigKey { name: "VM_POLL_INTERVAL_MS", kind: ValueKind::Int(100, 60000), default: ConfigValue::Int(5000) },
    ConfigKey { name: "VM_SWAP_USAGE_LOW", kind: ValueKind::Int(0, 100), default: ConfigValue::Int(40) },
    ConfigKey { name: "VM_SWAP_USAGE_HIGH", kind: ValueKind::Int(0, 100), default: ConfigValue::Int(80) },
    ConfigKey { name: "VM_IDLE_TIMEOUT_S", kind: ValueKind::Int(0, 86400), default: ConfigValue::Int(300) },
    ConfigKey { name: "LMKD_USE_PSI", kind: ValueKind::Auto, default: ConfigValue::AutoResolved(false) },
    ConfigKey { name: "LMKD_USE_MINFREE", kind: ValueKind::Auto, default: ConfigValue::AutoResolved(false) },
    ConfigKey { name: "UFFD_GC_ENABLE", kind: ValueKind::Bool, default: ConfigValue::Bool(false) },
    ConfigKey { name: "LOGGING_ENABLE", kind: ValueKind::Bool, default: ConfigValue::Bool(true) },
    ConfigKey { name: "LOG_LEVEL", kind: ValueKind::String, default: ConfigValue::String("info".to_string()) },
    ConfigKey { name: "LOG_MAX_SIZE_KB", kind: ValueKind::Int(64, 1048576), default: ConfigValue::Int(1024) },
    ConfigKey { name: "LOG_RETAIN_COUNT", kind: ValueKind::Int(0, 100), default: ConfigValue::Int(3) },
    ConfigKey { name: "CONFIG_POLL_INTERVAL_MS", kind: ValueKind::Int(100, 60000), default: ConfigValue::Int(5000) },
]);

pub fn keys() -> &'static [ConfigKey] {
    &KEYS
}

pub fn defaults() -> HashMap<String, ConfigValue> {
    KEYS.iter().map(|k| (k.name.to_string(), k.default.clone())).collect()
}

fn parse_line<'a>(line: &'a str) -> Option<(&'a str, &'a str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let eq_pos = line.find('=')?;
    let key = line[..eq_pos].trim();
    let raw_val = line[eq_pos + 1..].trim();
    if key.is_empty() {
        return None;
    }
    let val = if let Some(hash_pos) = raw_val.find('#') {
        let trimmed = raw_val[..hash_pos].trim();
        if trimmed.is_empty() { return None; }
        trimmed
    } else {
        raw_val
    };
    Some((key, val))
}

fn parse_value(key: &ConfigKey, value_str: &str) -> ConfigValue {
    match key.kind {
        ValueKind::Bool => match value_str.to_lowercase().as_str() {
            "true" | "1" | "yes" => ConfigValue::Bool(true),
            "false" | "0" | "no" => ConfigValue::Bool(false),
            _ => {
                eprintln!("WARN: invalid bool value '{}' for '{}', using default", value_str, key.name);
                key.default.clone()
            }
        },
        ValueKind::Auto => match value_str.to_lowercase().as_str() {
            "auto" | "true" | "1" => ConfigValue::AutoResolved(true),
            "false" | "0" | "no" => ConfigValue::AutoResolved(false),
            _ => {
                eprintln!("WARN: invalid auto value '{}' for '{}', using default", value_str, key.name);
                key.default.clone()
            }
        },
        ValueKind::Int(min, max) => match value_str.parse::<i64>() {
            Ok(n) => ConfigValue::Int(n.clamp(min, max)),
            Err(_) => {
                eprintln!("WARN: invalid int value '{}' for '{}', using default", value_str, key.name);
                key.default.clone()
            }
        },
        ValueKind::String => ConfigValue::String(value_str.to_string()),
    }
}

fn format_config_value(val: &ConfigValue) -> String {
    match val {
        ConfigValue::Bool(b) => (if *b { "true" } else { "false" }).to_string(),
        ConfigValue::AutoResolved(b) => (if *b { "true" } else { "false" }).to_string(),
        ConfigValue::Int(i) => i.to_string(),
        ConfigValue::String(s) => s.clone(),
    }
}

pub fn parse(text: &str) -> HashMap<String, ConfigValue> {
    let mut map = defaults();
    let key_index: HashMap<&str, &ConfigKey> = KEYS.iter().map(|k| (k.name, k)).collect();

    for line in text.lines() {
        if let Some((raw_key, raw_val)) = parse_line(line) {
            match key_index.get(raw_key) {
                Some(ck) => {
                    map.insert(ck.name.to_string(), parse_value(ck, raw_val));
                }
                None => {
                    eprintln!("DEBUG: unknown config key '{}' ignored", raw_key);
                }
            }
        }
    }
    map
}

pub fn write_effective(path: &Path, map: &HashMap<String, ConfigValue>) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    let mut content = String::new();
    for ck in KEYS.iter() {
        if let Some(val) = map.get(ck.name) {
            content.push_str(&format!("{}={}\n", ck.name, format_config_value(val)));
        }
    }
    std::fs::write(&tmp_path, &content)?;
    let _ = std::fs::remove_file(path);
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_comments_and_blanks() {
        let s = "# top\n\nPSI_THRESHOLD=55\n";
        let m = parse(s);
        assert_eq!(m.get("PSI_THRESHOLD").and_then(|v| v.as_int()), Some(55));
    }

    #[test]
    fn last_value_wins_on_duplicates() {
        let s = "PSI_THRESHOLD=10\nPSI_THRESHOLD=80\n";
        let m = parse(s);
        assert_eq!(m.get("PSI_THRESHOLD").and_then(|v| v.as_int()), Some(80));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let s = "PSI_THRESHOLD=50\nFOO=bar\n";
        let m = parse(s);
        assert!(!m.contains_key("FOO"));
    }

    #[test]
    fn bool_invalid_falls_back_to_default() {
        let s = "ZRAM_ENABLE=yes\n";
        let m = parse(s);
        assert_eq!(m.get("ZRAM_ENABLE"), Some(&ConfigValue::Bool(true)));
    }

    #[test]
    fn auto_invalid_falls_back_to_default() {
        let s = "PSI_AVAILABLE=maybe\n";
        let m = parse(s);
        assert_eq!(m.get("PSI_AVAILABLE"), Some(&ConfigValue::AutoResolved(false)));
    }

    #[test]
    fn int_is_clamped() {
        let s = "PSI_THRESHOLD=500\n";
        let m = parse(s);
        assert_eq!(m.get("PSI_THRESHOLD").and_then(|v| v.as_int()), Some(100));
    }

    #[test]
    fn int_invalid_falls_back_to_default() {
        let s = "PSI_THRESHOLD=hot\n";
        let m = parse(s);
        assert_eq!(m.get("PSI_THRESHOLD").and_then(|v| v.as_int()), Some(70));
    }

    #[test]
    fn effective_writer_is_atomic() {
        let dir = std::env::temp_dir().join("chimera_test_eff");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("config.effective");
        let mut m = defaults();
        m.insert("PSI_THRESHOLD".into(), ConfigValue::Int(33));
        write_effective(&target, &m).unwrap();
        assert!(target.exists());
        let body = std::fs::read_to_string(&target).unwrap();
        assert!(body.contains("PSI_THRESHOLD=33"));
    }
}
