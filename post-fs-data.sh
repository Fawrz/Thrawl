#!/system/bin/sh
# Validate / create default system.prop.
MODDIR="${0%/*}"
SP="$MODDIR/system.prop"
if [ ! -s "$SP" ]; then
    cat > "$SP" <<'EOF'
# Default LMKD properties applied by Chimera at boot.
# Do not edit at runtime; tuned by lmkd.sh from config.effective.
EOF
fi
