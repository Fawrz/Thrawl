#!/system/bin/sh
# Thrawl shell utilities. Sourced by other scripts.

MODDIR="${MODDIR:-/data/adb/modules/thrawl}"
RUNTIME_LOG_DIR="/data/adb/thrawl/logs"
RUNTIME_CONFIG="/data/adb/thrawl/config.conf"

if ! command -v ui_print >/dev/null 2>&1; then
    ui_print() { echo "$1"; }
fi

if ! command -v abort >/dev/null 2>&1; then
    abort() { ui_print "! $1"; exit 1; }
fi

prop_set() { resetprop "$1" "$2" 2>/dev/null; }
prop_del() { resetprop --delete "$1" 2>/dev/null; }

log_info() { echo "[thrawl] $1" >> "$RUNTIME_LOG_DIR/thrawl.log" 2>/dev/null; }
log_warn() { echo "[thrawl][warn] $1" >> "$RUNTIME_LOG_DIR/thrawl.log" 2>/dev/null; }
log_err()  { echo "[thrawl][err] $1" >> "$RUNTIME_LOG_DIR/thrawl.log" 2>/dev/null; }

ensure_runtime_dirs() {
    mkdir -p "/data/adb/thrawl" "$RUNTIME_LOG_DIR" "/data/adb/thrawl/swap"
}
