#!/system/bin/sh
# Magisk installer. Selects daemon binary for device ABI.
set -e
. "$MODPATH/scripts/utils.sh"
. "$MODPATH/scripts/install.sh"
ensure_runtime_dirs
install_thrawld_binary
ui_print "- Thrawl installed."
