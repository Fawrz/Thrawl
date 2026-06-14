#!/system/bin/sh
# ABI selection and daemon binary copy.
install_thrawld_binary() {
    ABI="$(getprop ro.product.cpu.abi)"
    ABILIST="$(getprop ro.product.cpu.abilist)"
    DEST="$MODPATH/system/bin/thrawld"
    case "$ABI" in
        aarch64|arm64-v8a) SRC="$MODPATH/system/bin/aarch64/thrawld" ;;
        armeabi-v7a|armv7l) SRC="$MODPATH/system/bin/arm/thrawld" ;;
        *) SRC="";;
    esac
    if [ -z "$SRC" ] || [ ! -f "$SRC" ]; then
        FIRST="$(echo "$ABILIST" | tr ',' '\n' | head -n1)"
        case "$FIRST" in
            aarch64|arm64-v8a) SRC="$MODPATH/system/bin/aarch64/thrawld" ;;
            armeabi-v7a|armv7l) SRC="$MODPATH/system/bin/arm/thrawld" ;;
        esac
    fi
    if [ -z "$SRC" ] || [ ! -f "$SRC" ]; then
        abort "no thrawld binary for ABI: $ABI"
    fi
    rm -f "$DEST"
    cp "$SRC" "$DEST"
    chmod 0755 "$DEST"
}
