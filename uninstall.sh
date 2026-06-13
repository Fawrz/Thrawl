#!/system/bin/sh
# Cleanup on module removal.
MODDIR="${0%/*}"
. "$MODDIR/scripts/utils.sh"
ensure_runtime_dirs

for f in /data/adb/chimera/swap/*; do
    [ -f "$f" ] || continue
    swapoff "$f" 2>/dev/null
    rm -f "$f"
done

for z in /sys/block/zram*; do
    [ -e "$z" ] || continue
    idx="${z##*/zram}"
    echo "$idx" > /sys/class/zram-control/hot_remove 2>/dev/null
done

sh "$MODDIR/scripts/logging.sh" stop 2>/dev/null

resetprop --delete ro.lmk.use_psi 2>/dev/null
resetprop --delete ro.lmk.use_minfree_levels 2>/dev/null
resetprop --delete ro.dalvik.vm.enable_uffd_gc 2>/dev/null
cmd device_config delete runtime_native_boot enable_uffd_gc_2 2>/dev/null

rm -f "$MODDIR/data/flags/chimerad.pid" "$MODDIR/data/flags/logcat.pid"
log_info "uninstall: cleanup complete"
