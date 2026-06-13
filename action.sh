#!/system/bin/sh
# Quick Settings toggle helper. Restarts chimerad.
MODDIR="${0%/*}"
. "$MODDIR/scripts/utils.sh"
ensure_runtime_dirs
if [ -f "$MODDIR/data/flags/chimerad.pid" ]; then
    pid="$(cat "$MODDIR/data/flags/chimerad.pid")"
    kill -TERM "$pid" 2>/dev/null
    sleep 1
fi
sh "$MODDIR/service.sh" &
echo "Chimera toggled."
