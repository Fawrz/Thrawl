#!/system/bin/sh
# ABI selection and daemon binary copy.
install_chimerad_binary() {
    ABI="$(getprop ro.product.cpu.abi)"
    ABILIST="$(getprop ro.product.cpu.abilist)"
    DEST="$MODPATH/system/bin/chimerad"
    case "$ABI" in
        aarch64|arm64-v8a) SRC="$MODPATH/system/bin/aarch64/chimerad" ;;
        armeabi-v7a|armv7l) SRC="$MODPATH/system/bin/arm/chimerad" ;;
        *) SRC="";;
    esac
    if [ -z "$SRC" ] || [ ! -f "$SRC" ]; then
        FIRST="$(echo "$ABILIST" | tr ',' '\n' | head -n1)"
        case "$FIRST" in
            aarch64|arm64-v8a) SRC="$MODPATH/system/bin/aarch64/chimerad" ;;
            armeabi-v7a|armv7l) SRC="$MODPATH/system/bin/arm/chimerad" ;;
        esac
    fi
    if [ -z "$SRC" ] || [ ! -f "$SRC" ]; then
        abort "no chimerad binary for ABI: $ABI"
    fi
    rm -f "$DEST"
    cp "$SRC" "$DEST"
    chmod 0755 "$DEST"
}
