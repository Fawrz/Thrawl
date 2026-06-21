#!/system/bin/sh
# Cleanup on module removal.
MODDIR="${0%/*}"
. "$MODDIR/scripts/utils.sh"
ensure_runtime_dirs

# Deactivate Thrawl-owned swap files
for f in /data/adb/thrawl/swap/*; do
    [ -f "$f" ] || continue
    swapoff "$f" 2>/dev/null
    rm -f "$f"
done

# Deactivate Thrawl-owned ZRAM (tracked in flags/swap.d/*.zram)
for f in "$MODDIR/data/flags/swap.d/"*.zram; do
    [ -f "$f" ] || continue
    idx="$(cat "$f")"
    swapoff "/dev/block/zram${idx}" 2>/dev/null
    echo "$idx" > /sys/class/zram-control/hot_remove 2>/dev/null
    rm -f "$f"
done

# Remove Thrawl-owned swap flags
rm -f "$MODDIR/data/flags/swap.d/"*.swap 2>/dev/null

sh "$MODDIR/scripts/logging.sh" stop 2>/dev/null

resetprop --delete ro.lmk.use_psi 2>/dev/null
resetprop --delete ro.lmk.use_minfree_levels 2>/dev/null
resetprop --delete ro.lmk.kill_heaviest_task 2>/dev/null
resetprop --delete ro.lmk.thrashing_limit 2>/dev/null
resetprop --delete ro.lmk.thrashing_limit_decay 2>/dev/null
resetprop --delete ro.dalvik.vm.enable_uffd_gc 2>/dev/null
cmd device_config delete runtime_native_boot enable_uffd_gc_2 2>/dev/null

rm -f "$MODDIR/data/flags/thrawld.pid" "$MODDIR/data/flags/logcat.pid"
log_info "uninstall: cleanup complete"
