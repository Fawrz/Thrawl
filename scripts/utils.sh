#!/system/bin/sh
# Chimera shell utilities. Sourced by other scripts.

MODDIR="${MODDIR:-/data/adb/modules/chimera}"
RUNTIME_LOG_DIR="/data/adb/chimera/logs"
RUNTIME_CONFIG="/data/adb/chimera/config.conf"

ui_print() { echo "$1"; }
abort() { ui_print "! $1"; exit 1; }

prop_get() { getprop "$1" 2>/dev/null; }
prop_set() { resetprop "$1" "$2" 2>/dev/null; }
prop_del() { resetprop --delete "$1" 2>/dev/null; }

log_info() { echo "[chimera] $1" >> "$RUNTIME_LOG_DIR/chimera.log" 2>/dev/null; }
log_warn() { echo "[chimera][warn] $1" >> "$RUNTIME_LOG_DIR/chimera.log" 2>/dev/null; }
log_err()  { echo "[chimera][err] $1" >> "$RUNTIME_LOG_DIR/chimera.log" 2>/dev/null; }

run_with_timeout() {
    secs="$1"; shift
    ( "$@" ) &
    pid=$!
    ( sleep "$secs"; kill -0 "$pid" 2>/dev/null && kill -9 "$pid" ) >/dev/null 2>&1 &
    watchdog=$!
    wait "$pid"
    rc=$?
    kill "$watchdog" 2>/dev/null
    return $rc
}

ensure_runtime_dirs() {
    mkdir -p "/data/adb/chimera" "$RUNTIME_LOG_DIR" "/data/adb/chimera/swap"
}
