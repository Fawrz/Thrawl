#!/system/bin/sh
# UFFD GC helper. Never enables v1 and v2 simultaneously.
. "$(dirname "$0")/utils.sh"

SDK="$(getprop ro.build.version.sdk)"
EFFECTIVE="${MODDIR:-/data/adb/modules/chimera}/data/config.effective"
UFFD_GC_ENABLE="$(grep '^UFFD_GC_ENABLE=' "$EFFECTIVE" 2>/dev/null | cut -d= -f2)"
UFFD_GC_ENABLE="${UFFD_GC_ENABLE:-0}"

apply() {
    if [ "$UFFD_GC_ENABLE" != "1" ]; then
        clear
        return 0
    fi
    if [ "$SDK" -ge 33 ]; then
        cmd device_config put runtime_native_boot enable_uffd_gc_2 true 2>/dev/null
        prop_del "ro.dalvik.vm.enable_uffd_gc"
        log_info "uffd: v2 enabled (sdk=$SDK)"
    elif [ "$SDK" -ge 31 ] && [ "$SDK" -le 32 ]; then
        prop_set "ro.dalvik.vm.enable_uffd_gc" "true"
        cmd device_config delete runtime_native_boot enable_uffd_gc_2 2>/dev/null
        log_info "uffd: v1 enabled (sdk=$SDK)"
    else
        log_warn "uffd: unsupported sdk=$SDK"
    fi
}

clear() {
    prop_del "ro.dalvik.vm.enable_uffd_gc"
    cmd device_config delete runtime_native_boot enable_uffd_gc_2 2>/dev/null
    log_info "uffd: cleared"
}

case "${1:-apply}" in
    apply) apply ;;
    clear) clear ;;
    *) log_err "uffd: unknown subcommand $1"; exit 1 ;;
esac
