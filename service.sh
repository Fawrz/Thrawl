#!/system/bin/sh
# Boot entrypoint. Hands off to thrawld.
main() {
    MODDIR="${0%/*}"
    . "$MODDIR/scripts/utils.sh"
    ensure_runtime_dirs
    if [ ! -f "$RUNTIME_CONFIG" ]; then
        cp "$MODDIR/config.conf" "$RUNTIME_CONFIG"
    fi
    exec "$MODDIR/system/bin/thrawld" "$MODDIR"
}
main "$@"
