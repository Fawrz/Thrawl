#!/system/bin/sh
# Thrawl diagnostics dump.
. "$(dirname "$0")/utils.sh"
ensure_runtime_dirs

OUT="$RUNTIME_LOG_DIR/diagnostics.txt"
{
    echo "=== Thrawl Diagnostics ==="
    echo "Date: $(date)"
    echo "Kernel: $(uname -a)"
    echo
    echo "--- PSI ---"
    [ -f /proc/pressure/memory ] && cat /proc/pressure/memory || echo "(PSI unavailable)"
    echo
    echo "--- Meminfo ---"
    head -n 20 /proc/meminfo
    echo
    echo "--- Swappiness ---"
    cat /proc/sys/vm/swappiness
    echo
    echo "--- ZRAM ---"
    ls /sys/block/ 2>/dev/null | grep '^zram' || echo "(none)"
    echo
    echo "--- Swaps ---"
    cat /proc/swaps
    echo
    echo "--- LMKD props ---"
    getprop | grep -E 'lmk\.' || true
    echo
    echo "--- UFFD ---"
    getprop ro.dalvik.vm.enable_uffd_gc
    cmd device_config get runtime_native_boot enable_uffd_gc_2 2>/dev/null
    echo
    echo "--- Flags ---"
    ls "${MODDIR:-/data/adb/modules/thrawl}/data/flags" 2>/dev/null || true
    echo
} > "$OUT" 2>&1
echo "$OUT"
