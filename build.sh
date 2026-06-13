#!/usr/bin/env bash
# Build Chimera release using cargo-ndk and package the Magisk module.
set -euo pipefail

cd "$(dirname "$0")"

NDK_HOME="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-$HOME/Android/Sdk/ndk/28.2.13676358}}"
[ -d "$NDK_HOME" ] || { echo "ANDROID_NDK_HOME not set and default not found"; exit 1; }

command -v cargo-ndk >/dev/null || { echo "cargo-ndk not installed (cargo install cargo-ndk)"; exit 1; }

OUT="$(pwd)/build-out"
rm -rf "$OUT"
mkdir -p "$OUT/system/bin/aarch64" "$OUT/system/bin/arm"

ABIS=("aarch64-linux-android" "armv7-linux-androideabi")
STAGE_DIRS=("aarch64" "arm")

for i in "${!ABIS[@]}"; do
    ABI="${ABIS[$i]}"
    STAGE="${STAGE_DIRS[$i]}"
    echo "==> Building $ABI"
    cargo ndk \
        --target "$ABI" \
        --platform 30 \
        --manifest-path Cargo.toml \
        build --release
    cp "target/$ABI/release/chimerad" "$OUT/system/bin/$STAGE/chimerad"
done

# Stage all scripts / props
cp -r customize.sh post-fs-data.sh service.sh uninstall.sh action.sh module.prop system.prop config.conf "$OUT/"
mkdir -p "$OUT/scripts"
cp -r scripts/*.sh "$OUT/scripts/"
chmod +x "$OUT/customize.sh" "$OUT/post-fs-data.sh" "$OUT/service.sh" "$OUT/uninstall.sh" "$OUT/action.sh" "$OUT/scripts/"*.sh

# Package
cd "$OUT"
ZIP_NAME="chimera-v1.0.0.zip"
rm -f "$ZIP_NAME"
zip -r9 "$ZIP_NAME" . -x "*.DS_Store"
cd - >/dev/null
echo "Built: $OUT/$ZIP_NAME"
