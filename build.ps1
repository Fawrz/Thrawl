# Build Thrawl Magisk Module using cargo-ndk
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
    $output = & { cargo ndk --target $abi.Target --platform 30 --manifest-path Cargo.toml build --release } 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { throw "Build failed for $($abi.Target)" }
    Write-Host $output
    Copy-Item "target\$($abi.Target)\release\thrawld" "$OUT\system\bin\$($abi.Stage)\thrawld"
}

# Stage all scripts / props
Copy-Item customize.sh, post-fs-data.sh, service.sh, uninstall.sh, action.sh, module.prop, system.prop, config.conf $OUT\
New-Item -ItemType Directory -Path "$OUT\scripts" -Force | Out-Null
Copy-Item scripts\*.sh $OUT\scripts\

# Dynamic version from git
$SHA = (git rev-parse --short HEAD).Trim()
$BUILD = (git rev-list --count HEAD).Trim()
$VERSION = "v1.0.0-$BUILD-$SHA"
$ZIP_NAME = "thrawl-release-$VERSION.zip"

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

Write-Host "Built: $ZIP_PATH"
