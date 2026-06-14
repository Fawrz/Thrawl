# Thrawl

> Adaptive LMKD / swappiness / ZRAM / swap optimizer for rooted Android (Magisk module).

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-v1.0.0-blue.svg)](https://github.com/Fawrz/Thrawl/releases)
[![Magisk](https://img.shields.io/badge/Magisk-20.4%2B-green.svg)](https://github.com/topjohnwu/Magisk)
[![Android](https://img.shields.io/badge/Android-8%E2%80%9313-brightgreen.svg)](#requirements)
[![Arch](https://img.shields.io/badge/arch-arm64%20%7C%20armv7-orange.svg)](#requirements)
[![Rust](https://img.shields.io/badge/rust-1.74%2B-orange.svg)](https://www.rust-lang.org)

Thrawl is a Magisk module that tunes the Android memory subsystem in real time using a single, statically-linked native daemon written in Rust. It monitors memory pressure, scales swap and ZRAM on demand, and reconfigures LMKD and userfaultfd garbage collection without requiring shell glue to keep running.

---

## Table of Contents

- [Description](#description)
- [Features](#features)
- [How It Works](#how-it-works)
- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Verifying It Works](#verifying-it-works)
- [Troubleshooting](#troubleshooting)
- [Uninstall](#uninstall)
- [Building From Source](#building-from-source)
- [Project Structure](#project-structure)
- [Testing](#testing)
- [Security and Risks](#security-and-risks)
- [Contributing](#contributing)
- [License](#license)
- [Credits](#credits)

---

## Description

Thrawl is a drop-in memory optimizer for rooted Android devices. Most "RAM booster" modules are scattered shell scripts that race on PID files and lose configuration on every boot. Thrawl replaces that surface with a single `thrawld` binary that:

- Reads `/proc/pressure/memory` (PSI) on modern kernels, falling back to `/proc/meminfo` pressure heuristics on older ones.
- Drives `/proc/sys/vm/swappiness` between two user-defined values based on the current pressure.
- Activates and deactivates a backing swap file and ZRAM devices on demand.
- Applies LMKD (`ro.lmk.*`) and userfaultfd GC (`ro.dalvik.vm.enable_uffd_gc`, `enable_uffd_gc_2`) properties for the current Android SDK.
- Persists its configuration across module updates by writing an effective config snapshot to `/data/adb/thrawl/config.effective`.
- Hot-reloads on `SIGHUP` and exits cleanly on `SIGTERM`.

The daemon is intentionally small, with no JSON or YAML parsers embedded. All configuration is read from a flat `KEY=VALUE` file with built-in type validation and clamping.

---

## Features

### Pressure backends
- **PSI backend** (Kernel 4.20+): reads `some avg60=` from `/proc/pressure/memory` and writes swappiness once per poll cycle.
- **Legacy backend** (Kernel 3.18 - 4.19, or kernels without `CONFIG_PSI`): reads `MemTotal` and `MemAvailable` and applies a hysteresis state machine so swappiness does not oscillate at the threshold.

### Swappiness
- Two-state auto-tuning between `SWAPPINESS_LOW` and `SWAPPINESS_HIGH`.
- Detects the kernel's effective swappiness maximum (some kernels clamp values above 100) and clamps the target before writing.
- Skips writes when the value has not changed.

### ZRAM
- Hot-adds and hot-removes `/sys/devices/virtual/misc/zram-control` devices up to `ZRAM_COUNT`.
- Sets `disksize` and `comp_algorithm` (`zstd` by default).
- Auto-sizing: defaults to `MemTotal / 4`, clamped between 512 MB and 4 GB.

### Swap
- Backing swap file under `SWAP_PATH` (default `/data/adb/thrawl/swap`).
- `mkswap` / `swapon` / `swapoff` lifecycle with timeout-protected invocations.
- Active swap devices are recorded in `data/flags/swap.d/*.swap` so the daemon can clean up after itself.

### LMKD
- Resolves `LMKD_USE_PSI` and `LMKD_USE_MINFREE` to concrete values when set to `auto` (PSI on if available, otherwise minfree).
- Writes `ro.lmk.use_psi`, `ro.lmk.use_minfree_levels`, and triggers `lmkd.reinit`.

### Userfaultfd GC
- Android 13+ (SDK 33): enables `enable_uffd_gc_2` via `cmd device_config`.
- Android 12 (SDK 31-32): enables `ro.dalvik.vm.enable_uffd_gc`.
- Never enables v1 and v2 simultaneously.

### Logging
- Persistent `logcat -v threadtime` capture into `/data/adb/thrawl/logs/logcat.log` with size-based rotation (`LOG_MAX_SIZE_KB`, `LOG_RETAIN_COUNT`).
- Daemon activity log at `/data/adb/thrawl/logs/thrawl.log`.

### Lifecycle
- PID lock file in `data/flags/thrawld.pid` with stale-PID recovery.
- Hot-reload on `SIGHUP` (re-reads config, reapplies helpers).
- Clean shutdown on `SIGTERM`, releases PID file.
- Magisk action button (`action.sh`) restarts the daemon on demand.

---

## How It Works

### Boot sequence

```
Android boot
  -> Magisk loads module
    -> post-fs-data.sh  : validates / creates system.prop defaults
      -> service.sh     : copies config.conf to /data/adb/thrawl/ (first boot)
        -> exec thrawld <MODDIR>
          -> detects PSI availability, writes data/flags/psi_available
          -> writes data/flags/swappiness_backend = "psi" | "legacy"
          -> applies LMKD + UFFD + logging helpers
          -> main loop: read pressure -> pick swappiness -> write to kernel
          -> polls CONFIG_POLL_INTERVAL_MS, watches config.conf mtime
```

### Daemon structure

```
thrawld (single process, multi-thread capable)
  |
  +-- config.rs        : typed KEYS table, parser, effective config writer
  +-- psi.rs           : /proc/pressure/memory reader, poll() wait
  +-- legacy.rs        : /proc/meminfo pressure + hysteresis state machine
  +-- swappiness.rs    : read/write, kernel-max detection, clamp helper
  +-- zram.rs          : hot_add, hot_remove, disksize, comp_algorithm
  +-- swap.rs          : create/mkswap/swapon/swapoff, flag tracking
  +-- vm_controller.rs : idle/active state for VM subsystem (scaffold)
  +-- lmkd.rs / uffd.rs / logging.rs : spawn helper scripts with timeout
  +-- flags.rs         : PID lock with stale-PID recovery
  +-- main_loop.rs     : the only place that orchestrates everything
```

### Signal protocol

| Signal | Effect |
|--------|--------|
| `SIGHUP` | Re-read `config.conf`, write `config.effective`, re-apply LMKD / UFFD / logging helpers, continue loop. |
| `SIGTERM` | Set shutdown flag, exit main loop, remove PID file. |
| `SIGPIPE` | Ignored. |

### File layout at runtime

```
/data/adb/modules/thrawl/        # Magisk module dir (read-only root of install)
  customize.sh
  service.sh
  post-fs-data.sh
  uninstall.sh
  action.sh
  module.prop
  system.prop
  config.conf                     # source of truth (read on every reload)
  scripts/                        # helper shell scripts
  system/bin/thrawld             # active binary (copied from aarch64/ or arm/)
  data/
    config.effective              # generated snapshot, consumed by scripts
    flags/thrawld.pid            # daemon PID
    flags/psi_available           # "1" or "0"
    flags/swappiness_backend      # "psi" or "legacy"
    flags/logcat.pid              # logging helper PID
    flags/swap.d/*.swap           # one file per active swap device

/data/adb/thrawl/                # user-mutable state
  config.conf                     # first-boot copy, may be edited
  logs/
    thrawl.log                   # daemon activity
    logcat.log                    # filtered system log
  swap/                           # swap file backing storage
```

---

## Requirements

| Component | Minimum | Notes |
|-----------|---------|-------|
| Magisk | 20.4+ | Required for `customize.sh`-only install (no `META-INF` needed). |
| Android | 8.0 - 13 (API 26-33) | Tested on Android 10 (Realme 5i). |
| Kernel | 3.18+ | Older kernels get the legacy backend automatically. |
| Kernel `CONFIG_PSI` | optional | Detected at boot. Unavailable -> fallback to meminfo. |
| Root | required | Magisk root or equivalent. |
| Free space | 256 MB+ | For build artifacts and the swap file backing. |

For building from source:

| Tool | Version |
|------|---------|
| Rust | 1.74+ |
| Android NDK | r25c or newer |
| `cargo-ndk` | 3.x or 4.x |

---

## Installation

### Option A: Download a prebuilt zip (recommended)

1. Download the latest `thrawl-vX.Y.Z.zip` from the [Releases](https://github.com/Fawrz/Thrawl/releases) page.
2. Open the **Magisk Manager** app.
3. Go to **Modules** -> **Install from storage** and pick the zip.
4. Reboot.

### Option B: Push via ADB

```bash
adb push thrawl-v1.0.0.zip /sdcard/
adb shell su -c 'magisk --install-module /sdcard/thrawl-v1.0.0.zip'
adb reboot
```

### Option C: Build from source

See [Building From Source](#building-from-source) below, then flash the produced zip with the same steps as Option A.

### Verify the install

```bash
adb shell su -c 'pgrep -af thrawld'
```

You should see at least one PID.

---

## Configuration

The user-facing config lives at `/data/adb/thrawl/config.conf` (a copy of the bundled `config.conf`, created on first boot). Lines starting with `#` and blank lines are ignored. Inline `#` comments after a value are also supported.

Every key has a fixed type, a default, and (for integers) a range. Invalid values fall back to the default with a warning printed to the daemon's stderr (visible in logcat). Out-of-range integers are clamped to the range.

### Key reference

#### PSI backend (used when `/proc/pressure/memory` exists)

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `PSI_AVAILABLE` | auto | `auto` | `auto` \| `0` \| `1` | `auto` resolves to `1` on PSI-capable kernels, `0` otherwise. |
| `PSI_THRESHOLD` | int | `70` | 0-100 | PSI `some avg60` percentage at which swappiness rises to `SWAPPINESS_HIGH`. |
| `PSI_POLL_TIMEOUT_MS` | int | `5000` | 100-60000 | Maximum time the daemon will wait between PSI samples. |

#### Legacy backend (used when PSI is unavailable)

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `LEGACY_PRESSURE_THRESHOLD` | int | `65` | 0-100 | `MemUsed / MemTotal` percentage that triggers `Raise`. |
| `LEGACY_HYSTERESIS` | int | `10` | 0-100 | Deadband below threshold. Within the band, swappiness is `Hold`-ed to prevent oscillation. |
| `LEGACY_POLL_INTERVAL_MS` | int | `5000` | 100-60000 | Wait between `/proc/meminfo` reads. |

#### Swappiness

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `SWAPPINESS_LOW` | int | `40` | 0-200 | Swappiness target when memory pressure is low. |
| `SWAPPINESS_HIGH` | int | `120` | 0-200 | Swappiness target when memory pressure is high. Clamped to the kernel's effective maximum. |

#### ZRAM

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `ZRAM_ENABLE` | bool | `1` | `0` / `1` | Enable ZRAM scaling. |
| `ZRAM_COUNT` | int | `4` | 0-32 | Number of ZRAM devices to maintain. |
| `ZRAM_SIZE_MB` | int | `0` | 0-65536 | Per-device size in MB. `0` = auto (`MemTotal / 4`, clamped 512 MB - 4 GB). |
| `ZRAM_COMP_ALGO` | string | `zstd` | kernel-supported | Compression algorithm (`zstd`, `lz4`, `lz4hc`, `deflate`, ...). |

#### Swap

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `SWAP_ENABLE` | bool | `1` | `0` / `1` | Enable the swap file. |
| `SWAP_SIZE_MB` | int | `0` | 0-65536 | Swap file size in MB. `0` = auto. |
| `SWAP_PATH` | string | `/data/adb/thrawl/swap` | valid path | Directory in which the swap file lives. |

#### VM controller (scaffold)

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `VM_POLL_INTERVAL_MS` | int | `5000` | 100-60000 | VM controller poll interval. |
| `VM_SWAP_USAGE_LOW` | int | `40` | 0-100 | Below this swap usage percent, deactivate. |
| `VM_SWAP_USAGE_HIGH` | int | `80` | 0-100 | Above this swap usage percent, activate. |
| `VM_IDLE_TIMEOUT_S` | int | `300` | 0-86400 | Idle time before considering deactivation. |

#### LMKD

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `LMKD_USE_PSI` | auto | `auto` | `auto` / `0` / `1` | `auto` follows PSI availability. |
| `LMKD_USE_MINFREE` | auto | `auto` | `auto` / `0` / `1` | `auto` is the inverse of `LMKD_USE_PSI`. |

#### Userfaultfd GC

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `UFFD_GC_ENABLE` | bool | `0` | `0` / `1` | Enable uffd GC. `0` clears any existing state. |

#### Logging

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `LOGGING_ENABLE` | bool | `1` | `0` / `1` | Start the persistent `logcat` capture. |
| `LOG_LEVEL` | string | `info` | any | Reserved for future log-level filtering. |
| `LOG_MAX_SIZE_KB` | int | `1024` | 64-1048576 | Rotated when the file exceeds this size. |
| `LOG_RETAIN_COUNT` | int | `3` | 0-100 | Number of rotated copies to keep. |

#### Internal

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `CONFIG_POLL_INTERVAL_MS` | int | `5000` | 100-60000 | Main loop sleep. Also acts as the config-file mtime check cadence. |

### Editing the config

Edit the file on-device with your preferred root editor, then send `SIGHUP`:

```bash
adb shell su -c 'kill -HUP $(cat /data/adb/modules/thrawl/data/flags/thrawld.pid)'
```

The daemon reloads the config and writes a fresh `data/config.effective` that the helper scripts (`lmkd.sh`, `uffd.sh`) consume.

---

## Verifying It Works

Run this one-liner for a full status dump:

```bash
adb shell su -c '
echo "=== Daemon PID ==="
pgrep -af thrawld || echo "(no daemon)"
echo
echo "=== Backend ==="
cat /data/adb/modules/thrawl/data/flags/swappiness_backend 2>/dev/null
echo
echo "=== PSI availability ==="
cat /data/adb/modules/thrawl/data/flags/psi_available 2>/dev/null
echo
echo "=== Current swappiness ==="
cat /proc/sys/vm/swappiness
echo
echo "=== Thrawl log (tail) ==="
tail -n 20 /data/adb/thrawl/logs/thrawl.log 2>/dev/null
echo
echo "=== Active swaps ==="
cat /proc/swaps
echo
echo "=== ZRAM devices ==="
ls /sys/block/ | grep ^zram || echo "(none)"
echo
echo "=== LMKD properties ==="
getprop | grep -E "lmk\." || echo "(none)"
'
```

A healthy install looks like this:

- `pgrep` returns one or more PIDs.
- `swappiness_backend` reads `psi` on modern kernels, `legacy` otherwise.
- `thrawl.log` shows `[thrawl] lmkd: use_psi=...` and `[thrawl] uffd: ...` lines.
- `cat /proc/swaps` shows the configured swap file or a `zram0` entry.

A per-subsystem diagnostics dump is also available:

```bash
adb shell sh /data/adb/modules/thrawl/scripts/diagnostics.sh
```

It writes a full report to `/data/adb/thrawl/logs/diagnostics.txt`.

---

## Troubleshooting

### "Installation failed" in Magisk Manager

Almost always a packaging problem, not a runtime bug. Verify the zip was built with forward-slash paths:

```bash
unzip -l thrawl-v1.0.0.zip
```

You should see `scripts/utils.sh`, not `scripts\utils.sh`. The bundled `build.ps1` uses the .NET `ZipArchive` API to force Unix-style paths; if you rebuilt the zip manually with `Compress-Archive`, you have the same bug we already fixed.

### Daemon is not running

```bash
adb shell su -c 'pgrep -af thrawld'
```

If empty:

1. Check the log: `adb shell su -c 'cat /data/adb/thrawl/logs/thrawl.log'`.
2. Check stderr capture: `adb shell su -c 'logcat -d -t 200 | grep -i thrawl'`.
3. Run the binary manually to see the error:

   ```bash
   adb shell su -c '/data/adb/modules/thrawl/system/bin/thrawld /data/adb/modules/thrawl'
   ```

4. Common causes:
   - Magisk denied `exec` to the binary. Check `chmod 0755` on `system/bin/thrawld`.
   - Stale PID file from a previous crash. The daemon self-recovers stale PIDs, but you can clear it manually: `rm /data/adb/modules/thrawl/data/flags/thrawld.pid`.
   - ABI mismatch: `customize.sh` aborts with `no thrawld binary for ABI: ...` if the device's primary ABI is neither `aarch64`, `arm64-v8a`, `armeabi-v7a`, nor `armv7l`. The bundled build includes both `aarch64` and `arm` binaries; a custom build with a single ABI must match.

### Swappiness never changes

- Your kernel may clamp swappiness to a maximum below `SWAPPINESS_HIGH`. The daemon auto-detects the kernel's effective max and clamps the target before writing. The actual value applied is the highest value in `[200, 180, 150, 120, 100, 60, 10, 0]` that the kernel accepted. Re-run `swappiness::detect_max` (or check `thrawl.log` after a `SIGHUP`) for the discovered maximum.
- If the device is in a low-pressure state for long periods, the value will stay at `SWAPPINESS_LOW`. That is by design.

### Swap file does not activate

- `SWAP_ENABLE=0` disables it.
- The first boot copies `config.conf` to `/data/adb/thrawl/config.conf`. Subsequent boots read the user copy, not the bundled one. Make sure you are editing the right file.
- Insufficient free space under `SWAP_PATH`. Free at least `SWAP_SIZE_MB` plus overhead.

### UFFD does nothing

- `UFFD_GC_ENABLE=0` (the default). Set it to `1` and `SIGHUP` the daemon.
- The device must be on Android 12 (SDK 31+) for the v1 path, or Android 13+ (SDK 33) for the v2 path. Older devices log `uffd: unsupported sdk=...` and the feature stays off.

### `customize.sh` aborts with `no thrawld binary for ABI`

The zip contains only `aarch64` and `arm` binaries. If the device is `x86_64` or `x86` you must build the binary for that target. Use `cargo ndk --target x86_64-linux-android --platform 30 build --release` and place the resulting `thrawld` in `system/bin/x86_64/thrawld` before zipping.

---

## Uninstall

### Recommended

Open **Magisk Manager** -> **Modules** -> **Thrawl** -> **Uninstall**, then reboot. The `uninstall.sh` script will:

- Deactivate every swap device in `flags/swap.d/`.
- Hot-remove every ZRAM device.
- Stop the logcat helper.
- Delete `ro.lmk.use_psi`, `ro.lmk.use_minfree_levels`, `ro.dalvik.vm.enable_uffd_gc`, and `enable_uffd_gc_2`.
- Remove `data/flags/thrawld.pid` and `data/flags/logcat.pid`.

### Manual cleanup

If Magisk is not available:

```bash
adb shell su -c '
sh /data/adb/modules/thrawl/uninstall.sh
rm -rf /data/adb/thrawl
rm -rf /data/adb/modules/thrawl
'
```

The user-mutable state under `/data/adb/thrawl/` (config, logs, swap file) is preserved on uninstall so that reinstalling keeps your configuration.

---

## Building From Source

### Install the toolchain

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Android NDK (download from developer.android.com, then)
export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/26.1.10909125

# cargo-ndk
cargo install cargo-ndk
```

### Build with the included script (Windows)

```powershell
.\build.ps1
```

The script:
- Locates the NDK in `ANDROID_NDK_HOME` or the default Windows SDK path.
- Cross-compiles `thrawld` for `aarch64-linux-android` and `armv7-linux-androideabi` with `cargo ndk`.
- Stages scripts, props, and binaries into `build-out/`.
- Packages everything into `build-out/thrawl-v1.0.0.zip` using the .NET `ZipArchive` API to enforce Unix forward-slash paths.

### Build with the included script (Linux / macOS)

The current repository ships a Windows-only `build.ps1`. On Unix, use the manual flow:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo ndk --target aarch64-linux-android --platform 30 build --release
cargo ndk --target armv7-linux-androideabi --platform 30 build --release

OUT=build-out
rm -rf $OUT && mkdir -p $OUT/{scripts,system/bin/aarch64,system/bin/arm}
cp customize.sh post-fs-data.sh service.sh uninstall.sh action.sh \
   module.prop system.prop config.conf $OUT/
cp scripts/*.sh $OUT/scripts/
cp target/aarch64-linux-android/release/thrawld $OUT/system/bin/aarch64/
cp target/armv7-linux-androideabi/release/thrawld $OUT/system/bin/arm/
(cd $OUT && zip -r ../thrawl-v1.0.0.zip .)
```

Release profile (`Cargo.toml`):

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
opt-level = "z"
```

- `lto = true` + `codegen-units = 1` produces the smallest, best-optimizing binary.
- `panic = abort` + `strip = true` keeps the release binary under 400 KB.
- `opt-level = "z"` is size-first; switch to `3` if your device is CPU-bound and not memory-bound.

---

## Project Structure

```
thrawl/
  Cargo.toml                       # crate manifest (name = thrawld)
  build.ps1                        # Windows build + package script
  customize.sh                     # Magisk installer entrypoint
  post-fs-data.sh                  # early-boot script
  service.sh                       # main boot entrypoint
  action.sh                        # Magisk action button
  uninstall.sh                     # cleanup on module remove
  module.prop                      # Magisk module metadata
  system.prop                      # default LMKD properties
  config.conf                      # user-facing configuration
  scripts/
    utils.sh                       # shared shell utilities
    install.sh                     # ABI selection + binary copy
    lmkd.sh                        # LMKD property helper
    uffd.sh                        # userfaultfd GC helper
    diagnostics.sh                 # full system dump
    logging.sh                     # persistent logcat capture
  src/
    main.rs                        # entrypoint, signal handlers, PID lock
    lib.rs                         # module declarations
    main_loop.rs                   # orchestration
    config.rs                      # typed KEYS table, parser, validator
    psi.rs                         # PSI backend
    legacy.rs                      # legacy (meminfo) backend with hysteresis
    swappiness.rs                  # kernel read/write + max detection
    zram.rs                        # ZRAM sysfs operations
    swap.rs                        # swap file + flag tracking
    vm_controller.rs               # idle/active state machine
    lmkd.rs                        # lmkd.sh invoker
    uffd.rs                        # uffd.sh invoker
    logging.rs                     # logging.sh invoker
    command.rs                     # timeout-bounded subprocess helper
    flags.rs                       # PID lock with stale recovery
  target/                          # cargo output (gitignored)
  build-out/                       # packaging staging (gitignored)
  *.zip                            # release artifacts (gitignored)
```

---

## Testing

The Rust crate ships with 37 unit tests covering:

- Config parser: comments, blanks, duplicate keys, unknown keys, type validation, integer clamping.
- Swappiness clamp helper and kernel-max detection.
- ZRAM `auto_size_bytes` bounds.
- PSI `is_available`, `read_avg60` parsers, constant sanity.
- Legacy `used_percent` and `decide` (Raise / Lower / Hold) state machine.
- VM controller `should_activate` / `should_deactivate` / `idle_deadline_reached`.
- Swap `record` / `list` / `unrecord` roundtrip on the temp directory.
- Command-runner timeout behaviour.

Run them all:

```bash
cargo test
```

Two `flags.rs` and `psi.rs` tests are gated on `/proc/sys/vm/swappiness` and `/proc/pressure/memory` actually existing; they will return `Ok(())` without asserting on Linux, and be a no-op on Windows.

---

## Security and Risks

- **Root is mandatory.** Thrawl writes to `/proc/sys/vm/swappiness` and triggers `resetprop` / `cmd device_config`. There is no unprivileged mode.
- **Kernel clamping is honored.** The daemon probes the kernel's effective swappiness maximum before writing and clamps the target. It will never write a value the kernel silently rejected (and would ignore anyway).
- **The swap file is plain bytes.** The daemon pre-allocates `SWAP_SIZE_MB` and `mkswap`s it. Do not point `SWAP_PATH` at a directory that holds user data; the file is truncated on creation.
- **Logcat capture is unfiltered.** The `logging.sh` helper runs `logcat -v threadtime` with no tag filter, so the captured log may contain other apps' output. Treat the file as potentially sensitive.
- **Configuration reload is hot but not atomic across processes.** A `SIGHUP` re-reads `config.conf` and rewrites `data/config.effective` before reapplying the helper scripts. There is a small window where a helper script reads the new effective file while the daemon has not yet finished writing it; this is mitigated by writing to `config.effective.tmp` and renaming atomically.

---

## Contributing

1. Fork the repository.
2. Create a topic branch: `git checkout -b fix/something`.
3. Make your change. Match the existing Rust 2021 edition style, run `cargo test`, and run `cargo clippy` if you have it installed.
4. Keep shell scripts POSIX-ish and sourceable. Use `set -e` and explicit error messages.
5. Open a pull request. Describe the problem and the fix; include a logcat excerpt if relevant.

Bug reports: use the [issue tracker](https://github.com/Fawrz/Thrawl/issues). Please include device model, Android version, kernel version, `uname -a`, and the output of the [diagnostics script](#verifying-it-works).

---

## License

MIT. See [LICENSE](LICENSE).

```
Copyright (c) 2026 Fawrz
```

---

## Credits

- Author: [Fawrz](https://github.com/Fawrz)
- Inspiration: the Android kernel memory subsystem, `lmkd`, and every shell-soup Magisk module that made me want a real daemon.
- Thanks to the Magisk project for making this kind of work possible in the first place.
