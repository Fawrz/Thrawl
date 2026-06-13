#!/system/bin/sh
# Persistent logcat service with rotation.
. "$(dirname "$0")/utils.sh"

LOGCAT_FLAG="${MODDIR:-/data/adb/modules/chimera}/data/flags/logcat.pid"
LOG_DIR="/data/adb/chimera/logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/logcat.log"

is_running() {
    [ -f "$LOGCAT_FLAG" ] || return 1
    pid="$(cat "$LOGCAT_FLAG" 2>/dev/null)"
    [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null
}

start() {
    is_running && { log_info "logcat: already running"; return 0; }
    : > "$LOG_FILE"
    nohup logcat -v threadtime > "$LOG_FILE" 2>&1 &
    echo $! > "$LOGCAT_FLAG"
    log_info "logcat: started pid=$!"
}

stop() {
    if [ -f "$LOGCAT_FLAG" ]; then
        pid="$(cat "$LOGCAT_FLAG" 2>/dev/null)"
        [ -n "$pid" ] && kill "$pid" 2>/dev/null
        rm -f "$LOGCAT_FLAG"
    fi
    log_info "logcat: stopped"
}

restart() { stop; start; }

rotate_if_needed() {
    max_kb="${LOG_MAX_SIZE_KB:-1024}"
    retain="${LOG_RETAIN_COUNT:-3}"
    if [ -f "$LOG_FILE" ]; then
        size_kb=$(( $(stat -c %s "$LOG_FILE" 2>/dev/null || echo 0) / 1024 ))
        if [ "$size_kb" -ge "$max_kb" ]; then
            i="$retain"
            while [ "$i" -ge 1 ]; do
                prev=$((i - 1))
                if [ "$prev" -ge 1 ]; then
                    [ -f "$LOG_FILE.$prev" ] && mv "$LOG_FILE.$prev" "$LOG_FILE.$i"
                fi
                i=$prev
            done
            mv "$LOG_FILE" "$LOG_FILE.1"
        fi
    fi
}

case "${1:-status}" in
    start) rotate_if_needed; start ;;
    stop) stop ;;
    restart) stop; start ;;
    status) is_running && echo "running" || echo "stopped" ;;
    *) log_err "logging: unknown subcommand $1"; exit 1 ;;
esac
