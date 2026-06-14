#!/system/bin/sh
# Magisk installer. Selects daemon binary for device ABI.
. "$MODPATH/scripts/utils.sh"
. "$MODPATH/scripts/install.sh"
ensure_runtime_dirs || abort "failed to prepare runtime directories"
install_thrawld_binary || abort "failed to install daemon binary"
ui_print "- Thrawl installed."
