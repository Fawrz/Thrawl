#!/system/bin/sh
# LMKD property helper. Reads config.effective and applies/clears properties.
. "$(dirname "$0")/utils.sh"

EFFECTIVE="${MODDIR:-/data/adb/modules/thrawl}/data/config.effective"

apply() {
    [ -f "$EFFECTIVE" ] || { log_warn "lmkd: no config.effective"; return 0; }
    USE_PSI="$(grep '^LMKD_USE_PSI=' "$EFFECTIVE" | cut -d= -f2)"
    USE_MINFREE="$(grep '^LMKD_USE_MINFREE=' "$EFFECTIVE" | cut -d= -f2)"
    if [ "$USE_PSI" = "1" ]; then
        prop_set "ro.lmk.use_psi" "1"
        prop_del "ro.lmk.use_minfree_levels"
    else
        prop_set "ro.lmk.use_psi" "0"
        if [ "$USE_MINFREE" = "1" ]; then
            prop_set "ro.lmk.use_minfree_levels" "1"
        else
            prop_del "ro.lmk.use_minfree_levels"
        fi
    fi
    resetprop lmkd.reinit 2>/dev/null
    log_info "lmkd: use_psi=$USE_PSI use_minfree=$USE_MINFREE"
}

clear() {
    prop_del "ro.lmk.use_psi"
    prop_del "ro.lmk.use_minfree_levels"
    resetprop lmkd.reinit 2>/dev/null
    log_info "lmkd: cleared"
}

case "${1:-apply}" in
    apply) apply ;;
    clear) clear ;;
    *) log_err "lmkd: unknown subcommand $1"; exit 1 ;;
esac
