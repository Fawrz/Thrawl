#!/usr/bin/env bash
# Build Thrawl release using cargo-ndk and package the Magisk module.
set -euo pipefail

cd "$(dirname "$0")"

BASE_VERSION="$(sed -n 's/^version=v//p' module.prop | head -n1 | tr -d '\r')"
[ -n "$BASE_VERSION" ] || { echo "Unable to determine base version from module.prop"; exit 1; }

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
    cp "target/$ABI/release/thrawld" "$OUT/system/bin/$STAGE/thrawld"
done

# Stage all scripts / props
cp -r customize.sh post-fs-data.sh service.sh uninstall.sh action.sh system.prop config.conf "$OUT/"
mkdir -p "$OUT/scripts"
cp -r scripts/*.sh "$OUT/scripts/"
chmod +x "$OUT/customize.sh" "$OUT/post-fs-data.sh" "$OUT/service.sh" "$OUT/uninstall.sh" "$OUT/action.sh" "$OUT/scripts/"*.sh

# Release version from git metadata.
SHA=$(git rev-parse --short HEAD)
BUILD=$(git rev-list --count HEAD)
PACKAGE_VERSION="v${BASE_VERSION}-${BUILD}-${SHA}"
ZIP_NAME="thrawl-${PACKAGE_VERSION}-release.zip"

cat > "$OUT/module.prop" <<EOF
id=thrawl
name=Thrawl
version=${PACKAGE_VERSION}
versionCode=${BUILD}
author=GitHub@Fawrz
description=A Rust daemon for adaptive memory management — ZRAM, swap, swappiness, and LMKD tuning. Works on PSI and legacy kernels.
updateJson=https://raw.githubusercontent.com/Fawrz/Thrawl/main/update.json
EOF

cat > "$OUT/update.json" <<EOF
{
    "version": "${PACKAGE_VERSION}",
    "versionCode": ${BUILD},
    "zipUrl": "https://github.com/Fawrz/Thrawl/releases/download/${PACKAGE_VERSION}/${ZIP_NAME}",
    "changelog": "https://github.com/Fawrz/Thrawl/releases/tag/${PACKAGE_VERSION}"
}
EOF

# Package
cd "$OUT"
rm -f "$ZIP_NAME"
zip -r9 "$ZIP_NAME" . -x "*.DS_Store"
cd - >/dev/null

# Create source archives (from repo root, exclude build-out, target, .git)
REPO_ROOT="$(pwd)"
REPO_NAME="$(basename "$REPO_ROOT")"
REPO_PARENT="$(dirname "$REPO_ROOT")"
SOURCE_TAR="$OUT/${PACKAGE_VERSION}-source.tar.gz"
SOURCE_ZIP="$OUT/${PACKAGE_VERSION}-source.zip"

tar --exclude='build-out' --exclude='target' --exclude='.git' -czf "$SOURCE_TAR" -C "$REPO_PARENT" "$REPO_NAME"
zip -r9 "$SOURCE_ZIP" . -x "build-out/*" "target/*" ".git/*" "*.DS_Store"

# Generate SHA256SUMS
cd "$OUT"
{
    sha256sum "$ZIP_NAME" "$SOURCE_TAR" "$SOURCE_ZIP"
} > SHA256SUMS
cd - >/dev/null

echo "Built: $OUT/$ZIP_NAME"
echo "Source tarball: $SOURCE_TAR"
echo "Source zip: $SOURCE_ZIP"
echo "SHA256SUMS: $OUT/SHA256SUMS"
