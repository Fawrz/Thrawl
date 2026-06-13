#!/system/bin/sh
# Magisk installer. Selects daemon binary for device ABI.
set -e
. "$MODPATH/scripts/utils.sh"
. "$MODPATH/scripts/install.sh"
ensure_runtime_dirs
install_chimerad_binary
ui_print "- Chimera installed."
