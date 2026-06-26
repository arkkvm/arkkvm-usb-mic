# arkkvm-usb-mic

[![License: GPL v2 or later](https://img.shields.io/badge/License-GPL%20v2%2B-blue.svg)](LICENSE)

Virtual-microphone audio playback daemon for ArkKVM (crate name `arkkvm_mic`). Runs on Rockchip RV1106 hardware, subscribes to PCM audio over Zenoh, and outputs it to a USB UAC1 sound card via Rockchip MPI AO.

[ArkKVM](https://www.arkkvm.com/) · [arkkvm-app](https://github.com/arkkvm/arkkvm-app) · [GitHub](https://github.com/arkkvm/arkkvm-usb-mic)

## Features

- Low-latency audio streaming over Zenoh
- Hardware audio output via Rockchip MPI AO
- Cross-compilation for `armv7-unknown-linux-uclibceabihf` (uclibc)
- Ships as both a binary and `rlib` / `staticlib` library

## Role in ArkKVM

ArkKVM firmware runs as three cooperating processes (see [arkkvm-app README — Architecture](https://github.com/arkkvm/arkkvm-app#architecture)):

| Process | Binary | Repository |
|---------|--------|--------------|
| Main firmware | `arkkvm_app` | [arkkvm-app](https://github.com/arkkvm/arkkvm-app) |
| USB sidecar | `arkkvm_usb` | [arkkvm-app](https://github.com/arkkvm/arkkvm-app) (`crates/usb_devices/`) |
| Virtual mic | `arkkvm_mic` | This repo |

`arkkvm_app` decodes remote WebRTC audio and forwards PCM; `arkkvm_mic` subscribes and plays it to the USB audio path; `arkkvm_usb` exposes the UAC1 gadget and manages the `arkkvm_mic` lifecycle.

```mermaid
flowchart LR
    App["arkkvm_app\nVirtual mic forward"] -->|"arkkvm_mic/data"| ZenohBus["Zenoh mic bus\nunixsock-stream"]
    ZenohBus -->|"connect: /tmp/zenoh_mic.sock"| MicService["arkkvm_mic\nAudioSubscriber"]
    MicService --> AudioOutput["AudioOutput\nRockchip MPI AO"]
    AudioOutput --> USBMic["USB UAC1\nhw:1,0"]
    USBMic --> HostPC["Host PC"]
```

## Requirements

This repository shares the same cross-compilation toolchain and sysroot as [arkkvm-app](https://github.com/arkkvm/arkkvm-app). For full environment setup (toolchain, sysroot, Rust stage2, etc.), follow **[arkkvm-app README — Building](https://github.com/arkkvm/arkkvm-app#building)**.

### Hardware

- Rockchip RV1106 (`armv7-unknown-linux-uclibceabihf`)
- USB UAC1 audio gadget (configured by `arkkvm_usb`)

### Build environment

| Dependency | Description |
|------------|-------------|
| Linux host (amd64/x86_64) | [rustup](https://rustup.rs/) installed |
| **C cross toolchain** | `arm-rockchip830-linux-uclibcgnueabihf/` (`BUILDKIT_ROOT`) |
| **Sysroot libraries** | MPI libs from the system build (e.g. `rockit`, `rockchip_mpp`, `rga`); see [arkkvm-system-v1](https://github.com/arkkvm/arkkvm-system-v1) |
| **Rust stage2 toolchain** | Built from Rust source tag **`1.94.1`**; use `cargo +stage2` |
| **Cross linker** | `$BUILDKIT_ROOT/bin/arm-rockchip830-linux-uclibcgnueabihf-gcc` |
| `bindgen-cli` | Generates Rockchip MPI FFI bindings (`build.sh` installs it if missing) |

Suggested sibling directory layout (same as [arkkvm-app](https://github.com/arkkvm/arkkvm-app#building)):

```text
parent/
├── arkkvm-app/
├── arkkvm-usb-mic/                          # this repo
├── arm-rockchip830-linux-uclibcgnueabihf/   # BUILDKIT_ROOT
└── rust/                                    # Rust source (stage2 toolchain)
```

### Rust toolchain

| Item | Value |
|------|-------|
| Rust source tag | **`1.94.1`** |
| Linked toolchain | `stage2` |
| Firmware target | `armv7-unknown-linux-uclibceabihf` |

For stage2 build steps and `bootstrap.toml` configuration, see [arkkvm-app README — Rust toolchain](https://github.com/arkkvm/arkkvm-app#rust-toolchain).

### Runtime dependencies

- Zenoh mic session: connects to `unixsock-stream//tmp/zenoh_mic.sock` (provided by `arkkvm_usb`)
- Rockchip MPI libraries on device: `librockit`, `librockchip_mpp`, `librga`, `librockiva`, `librknnmrt`, `librkaudio`

## Quick start

### Build

Ensure the toolchain and stage2 setup from [arkkvm-app — Building](https://github.com/arkkvm/arkkvm-app#building) are complete, then:

```bash
./build.sh
```

`build.sh` sets `BUILDKIT_ROOT` (default: `../arm-rockchip830-linux-uclibcgnueabihf`) and the cross linker automatically.

For manual builds:

```bash
export BUILDKIT_ROOT="$(realpath ../arm-rockchip830-linux-uclibcgnueabihf)"
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_UCLIBCEABIHF_LINKER="$BUILDKIT_ROOT/bin/arm-rockchip830-linux-uclibcgnueabihf-gcc"

cargo +stage2 build -Z build-std --release --target armv7-unknown-linux-uclibceabihf
```

Output:

```
target/armv7-unknown-linux-uclibceabihf/release/arkkvm_mic
```

### Run

`arkkvm_mic` is normally spawned and monitored by `arkkvm_usb`; running it standalone on the device is not recommended for production. For local debugging:

```bash
./target/armv7-unknown-linux-uclibceabihf/release/arkkvm_mic
# Ctrl-C to exit
```

On startup the process will:

1. Initialize a Zenoh session (connect to `/tmp/zenoh_mic.sock`, multicast discovery disabled)
2. Subscribe to the `arkkvm_mic/data` topic
3. Play received PCM via MPI AO to `hw:1,0`

## Configuration

Settings are currently hard-coded in source:

| Setting | Default | Location |
|---------|---------|----------|
| Zenoh connect endpoint | `unixsock-stream//tmp/zenoh_mic.sock` | [`src/zenoh_bus.rs`](src/zenoh_bus.rs) |
| Audio topic | `arkkvm_mic/data` | [`src/usb/mic_sub.rs`](src/usb/mic_sub.rs) |
| Sample rate / channels / format | 48000 Hz / 2ch / S16 | [`src/usb/mic_sub.rs`](src/usb/mic_sub.rs) |
| ALSA device | `hw:1,0` | [`src/usb/mic_sub.rs`](src/usb/mic_sub.rs) |

### Audio format

- Raw PCM with no container header (e.g. no WAV header)
- Publisher frame rate must match receiver sample rate and channel count

## Repository layout

```
arkkvm-usb-mic/
├── src/
│   ├── main.rs              # Entry: Zenoh init + audio subscriber
│   ├── lib.rs               # Library entry
│   ├── zenoh_bus.rs         # Zenoh session management
│   └── usb/
│       ├── mod.rs
│       ├── mic.rs           # AudioOutput high-level wrapper
│       ├── mic_c.rs         # Rockchip MPI AO bindings
│       └── mic_sub.rs       # Zenoh subscribe + playback loop
├── crates/
│   └── rockchip_mpi_sys/    # bindgen-generated MPI FFI
├── cshim/
│   └── getauxval.c          # uclibc getauxval compatibility shim
├── build.sh                 # Cross-compile entry script
└── build.rs                 # C shim build logic
```

## Development notes

- `BUILDKIT_ROOT` is **required**; both `build.rs` and `crates/rockchip_mpi_sys/build.rs` depend on it
- Use `cargo +stage2` for firmware builds — do not substitute an arbitrary nightly for stage2
- The library crate can be linked as a static `arkkvm_mic` library by other ArkKVM components

**Troubleshooting:** `BUILDKIT_ROOT not set` — export toolchain path · `Cross-compile getauxval.c failed` — incomplete sysroot · More build issues: [arkkvm-app README — Building](https://github.com/arkkvm/arkkvm-app#building)

## Related repositories

| Component | Repository |
|-----------|--------------|
| Product | [arkkvm.com](https://www.arkkvm.com/) |
| Firmware | [arkkvm-app](https://github.com/arkkvm/arkkvm-app) |
| Web UI | [arkkvm-app-frontend](https://github.com/arkkvm/arkkvm-app-frontend) |
| Virtual microphone | [arkkvm-usb-mic](https://github.com/arkkvm/arkkvm-usb-mic) (this repo) |
| System / toolchain | [arkkvm-system-v1](https://github.com/arkkvm/arkkvm-system-v1) |

## License

**Source code in this repository** is licensed under the [GNU General Public License v2.0 or later](LICENSE) (GPL-2.0-or-later).

### What this license covers

- All original source in this repo (`src/`, `cshim/`, `crates/rockchip_mpi_sys/` bindings, etc.)
- Corresponding source for `arkkvm_mic` binaries built from this tree

### What this license does not cover

- **ArkKVM system images** distributed for device flashing (proprietary product delivery)
- **Rockchip SDK libraries** (`librockit`, `librockchip_mpp`, `librga`, `librockiva`, `librknnmrt`, `librkaudio`) — proprietary; obtain via [arkkvm-system-v1](https://github.com/arkkvm/arkkvm-system-v1) or Rockchip SDK
- Other ArkKVM components (`arkkvm_app`, `arkkvm_usb`) in separate repositories

### Third-party components

See [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES). Zenoh is used under **Apache-2.0** (dual-license election). Prebuilt binaries linking Zenoh are subject to **GPL-3.0-or-later** obligations for the combined work.

### Obtaining source

GPL source for `arkkvm_mic`: this repository at [github.com/arkkvm/arkkvm-usb-mic](https://github.com/arkkvm/arkkvm-usb-mic).

When `arkkvm_mic` is distributed as part of an ArkKVM system image or product, the corresponding source for this component remains available at the URL above for at least three years from the date of binary distribution.
