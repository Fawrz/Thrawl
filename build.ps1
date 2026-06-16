# Build Thrawl Magisk Module using cargo-ndk
# Run from PowerShell: .\build.ps1

$ErrorActionPreference = "Stop"

function Write-Utf8NoBom {
    param(
        [string]$Path,
        [string]$Content
    )

    $utf8 = New-Object System.Text.UTF8Encoding -ArgumentList $false
    [System.IO.File]::WriteAllText($Path, ($Content -replace "`r`n", "`n"), $utf8)
}

$NDK_HOME = $env:ANDROID_NDK_HOME, $env:ANDROID_NDK_ROOT, "$env:LOCALAPPDATA\Android\Sdk\ndk\28.2.13676358" | Where-Object { $_ } | Select-Object -First 1

if (-not (Test-Path $NDK_HOME)) {
    Write-Error "Android NDK not found at $NDK_HOME. Set ANDROID_NDK_HOME."
    exit 1
}
$env:ANDROID_NDK_HOME = $NDK_HOME

$msvc = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207"
$sdk = "C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0"
$env:LIB = "$msvc\lib\x64;$sdk\ucrt\x64;$sdk\um\x64"
$env:INCLUDE = "$msvc\include;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\ucrt;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared"
$BASE_VERSION_LINE = Get-Content "module.prop" | Where-Object { $_ -match '^version=v' } | Select-Object -First 1
$BASE_VERSION = ($BASE_VERSION_LINE -replace '^version=v', '').Trim()
if (-not $BASE_VERSION) { throw "Unable to determine base version from module.prop" }

$OUT = Join-Path (Get-Location) "build-out"
if (Test-Path $OUT) { Remove-Item -Recurse -Force $OUT }
New-Item -ItemType Directory -Path "$OUT\system\bin\aarch64" -Force | Out-Null
New-Item -ItemType Directory -Path "$OUT\system\bin\arm" -Force | Out-Null

$ABIS = @(
    @{ Target = "aarch64-linux-android"; Stage = "aarch64" },
    @{ Target = "armv7-linux-androideabi"; Stage = "arm" }
)

foreach ($abi in $ABIS) {
    Write-Host "==> Building $($abi.Target)"
    $prevEA = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = cargo ndk --target $abi.Target --platform 30 --manifest-path Cargo.toml build --release 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prevEA
    }
    $outputLines = $output | ForEach-Object { "$_" }
    Write-Host $outputLines
    if ($exitCode -ne 0) { throw "Build failed for $($abi.Target)" }
    Copy-Item "target\$($abi.Target)\release\thrawld" "$OUT\system\bin\$($abi.Stage)\thrawld"
}

# Stage all scripts / props
Copy-Item customize.sh, post-fs-data.sh, service.sh, uninstall.sh, action.sh, system.prop, config.conf $OUT\
New-Item -ItemType Directory -Path "$OUT\scripts" -Force | Out-Null
Copy-Item scripts\*.sh $OUT\scripts\

# Release version from git metadata.
$SHA = (git rev-parse --short HEAD).Trim()
$BUILD = [int](git rev-list --count HEAD).Trim()
$TAG = & { git describe --tags --exact-match HEAD } 2>$null
if ($TAG) {
    $TAG = $TAG.Trim()
    $BASE_TAG = $TAG
} else {
    $BASE_TAG = "v$BASE_VERSION"
}

$DISPLAY_VERSION = "$BASE_TAG ($BUILD-$SHA)"
$ASSET_VERSION = "$BASE_TAG-$BUILD-$SHA"
if ($TAG) {
    $RELEASE_TAG = $TAG
} else {
    $RELEASE_TAG = $ASSET_VERSION
}

$ZIP_NAME = "thrawl-$ASSET_VERSION-release.zip"

Write-Utf8NoBom (Join-Path $OUT "module.prop") @"
id=thrawl
name=Thrawl
version=$DISPLAY_VERSION
versionCode=$BUILD
author=GitHub@Fawrz
description=A Rust daemon for adaptive memory management — ZRAM, swap, swappiness, and LMKD tuning. Works on PSI and legacy kernels.
updateJson=https://raw.githubusercontent.com/Fawrz/Thrawl/main/update.json
"@

Write-Utf8NoBom (Join-Path $OUT "update.json") @"
{
    "version": "$DISPLAY_VERSION",
    "versionCode": $BUILD,
    "zipUrl": "https://github.com/Fawrz/Thrawl/releases/download/$RELEASE_TAG/$ZIP_NAME",
    "changelog": "https://github.com/Fawrz/Thrawl/releases/tag/$RELEASE_TAG"
}
"@

# Package using .NET ZipArchive with Unix forward-slash paths
$ZIP_PATH = Join-Path $OUT $ZIP_NAME
Remove-Item $ZIP_PATH -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

Add-Type -AssemblyName System.IO.Compression
$zipStream = [System.IO.File]::Create($ZIP_PATH)
$zip = [System.IO.Compression.ZipArchive]::new($zipStream, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    $files = Get-ChildItem -Recurse -File $OUT | Where-Object { $_.FullName -ne $ZIP_PATH }
    foreach ($f in $files) {
        $rel = $f.FullName.Substring($OUT.Length + 1)
        $rel = $rel.Replace('\', '/')
        $entry = $zip.CreateEntry($rel, [System.IO.Compression.CompressionLevel]::Optimal)
        $entryWriter = $entry.Open()
        $fileBytes = [System.IO.File]::ReadAllBytes($f.FullName)
        $entryWriter.Write($fileBytes, 0, $fileBytes.Length)
        $entryWriter.Dispose()
    }
} finally {
    $zip.Dispose()
    $zipStream.Dispose()
}

# Create source archives (from repo root, exclude build-out, target, .git)
$REPO_ROOT = (Get-Location).Path
$REPO_NAME = Split-Path $REPO_ROOT -Leaf
$REPO_PARENT = Split-Path $REPO_ROOT -Parent
$SOURCE_NAME = "$ASSET_VERSION-source"
$SOURCE_TAR_PATH = Join-Path $OUT "$SOURCE_NAME.tar.gz"
$SOURCE_ZIP_PATH = Join-Path $OUT "$SOURCE_NAME.zip"

# Create source tarball using tar (Windows 10+ ships with bsdtar)
Push-Location $REPO_PARENT
try {
    tar --exclude='build-out' --exclude='target' --exclude='.git' -czf $SOURCE_TAR_PATH $REPO_NAME
} finally {
    Pop-Location
}

# Create source zip using .NET ZipArchive
$sourceZipStream = [System.IO.File]::Create($SOURCE_ZIP_PATH)
$sourceZip = [System.IO.Compression.ZipArchive]::new($sourceZipStream, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    $sourceFiles = Get-ChildItem -Recurse -File $REPO_ROOT | Where-Object {
        $full = $_.FullName
        -not ($full.Contains([IO.Path]::DirectorySeparatorChar + 'build-out' + [IO.Path]::DirectorySeparatorChar)) -and
        -not ($full.Contains([IO.Path]::DirectorySeparatorChar + 'target' + [IO.Path]::DirectorySeparatorChar)) -and
        -not ($full.Contains([IO.Path]::DirectorySeparatorChar + '.git' + [IO.Path]::DirectorySeparatorChar))
    }
    foreach ($f in $sourceFiles) {
        $rel = $f.FullName.Substring($REPO_ROOT.Length + 1)
        $rel = $rel.Replace('\', '/')
        $entry = $sourceZip.CreateEntry($rel, [System.IO.Compression.CompressionLevel]::Optimal)
        $entryWriter = $entry.Open()
        $fileBytes = [System.IO.File]::ReadAllBytes($f.FullName)
        $entryWriter.Write($fileBytes, 0, $fileBytes.Length)
        $entryWriter.Dispose()
    }
} finally {
    $sourceZip.Dispose()
    $sourceZipStream.Dispose()
}

# Generate SHA256SUMS
$files = @(
    (Join-Path $OUT $ZIP_NAME),
    $SOURCE_TAR_PATH,
    $SOURCE_ZIP_PATH
)

$sumContent = ($files | ForEach-Object {
    $hash = Get-FileHash -Path $_ -Algorithm SHA256
    $name = Split-Path $hash.Path -Leaf
    "$($hash.Hash.ToUpper())  $name"
}) -join "`n"

Write-Utf8NoBom (Join-Path $OUT "SHA256SUMS") $sumContent

Write-Host "Built: $ZIP_PATH"
Write-Host "Source tarball: $SOURCE_TAR_PATH"
Write-Host "Source zip: $SOURCE_ZIP_PATH"
Write-Host "SHA256SUMS: $(Join-Path $OUT 'SHA256SUMS')"
