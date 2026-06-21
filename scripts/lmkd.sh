#!/system/bin/sh
# LMKD property helper. Reads config.effective and applies/clears properties.
. "$(dirname "$0")/utils.sh"

EFFECTIVE="${MODDIR:-/data/adb/modules/thrawl}/data/config.effective"

apply() {
    [ -f "$EFFECTIVE" ] || { log_warn "lmkd: no config.effective"; return 0; }
    USE_PSI="$(grep '^LMKD_USE_PSI=' "$EFFECTIVE" | cut -d= -f2)"
    USE_MINFREE="$(grep '^LMKD_USE_MINFREE=' "$EFFECTIVE" | cut -d= -f2)"
    KILL_HEAVIEST="$(grep '^LMKD_KILL_HEAVIEST_TASK=' "$EFFECTIVE" | cut -d= -f2)"
    THRASHING_LIMIT="$(grep '^LMKD_THRASHING_LIMIT=' "$EFFECTIVE" | cut -d= -f2)"
    THRASHING_LIMIT_DECAY="$(grep '^LMKD_THRASHING_LIMIT_DECAY=' "$EFFECTIVE" | cut -d= -f2)"

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

    prop_set "ro.lmk.kill_heaviest_task" "${KILL_HEAVIEST:-0}"
    prop_set "ro.lmk.thrashing_limit" "${THRASHING_LIMIT:-30}"
    prop_set "ro.lmk.thrashing_limit_decay" "${THRASHING_LIMIT_DECAY:-80}"

    resetprop lmkd.reinit 2>/dev/null
    log_info "lmkd: use_psi=$USE_PSI kill_heaviest=$KILL_HEAVIEST thrashing=$THRASHING_LIMIT"
}

clear() {
    prop_del "ro.lmk.use_psi"
    prop_del "ro.lmk.use_minfree_levels"
    prop_del "ro.lmk.kill_heaviest_task"
    prop_del "ro.lmk.thrashing_limit"
    prop_del "ro.lmk.thrashing_limit_decay"
    resetprop lmkd.reinit 2>/dev/null
    log_info "lmkd: cleared"
}

case "${1:-apply}" in
    apply) apply ;;
    clear) clear ;;
    *) log_err "lmkd: unknown subcommand $1"; exit 1 ;;
esac
