# Build Chimera Magisk Module using cargo-ndk
# Run from PowerShell: .\build.ps1

$ErrorActionPreference = "Stop"

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
    cargo ndk --target $abi.Target --platform 30 --manifest-path Cargo.toml build --release 2>&1
    if (-not $?) { throw "Build failed for $($abi.Target)" }
    Copy-Item "target\$($abi.Target)\release\chimerad" "$OUT\system\bin\$($abi.Stage)\chimerad"
}

# Stage all scripts / props
Copy-Item customize.sh, post-fs-data.sh, service.sh, uninstall.sh, action.sh, module.prop, system.prop, config.conf $OUT\
New-Item -ItemType Directory -Path "$OUT\scripts" -Force | Out-Null
Copy-Item scripts\*.sh $OUT\scripts\

# Package using PowerShell Compress-Archive
$ZIP_NAME = "chimera-v1.0.0.zip"
$ZIP_PATH = Join-Path $OUT $ZIP_NAME
Remove-Item $ZIP_PATH -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
Compress-Archive -Path "$OUT\*" -DestinationPath $ZIP_PATH -CompressionLevel Optimal

Write-Host "Built: $ZIP_PATH"
