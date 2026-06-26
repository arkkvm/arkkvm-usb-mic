#!/bin/bash
# set -e

if bindgen --version &>/dev/null; then :; else
    cargo install --locked bindgen-cli
fi


export BUILDKIT_ROOT="/opt/jetkvm-native-buildkit"
# export BUILDKIT_ROOT="$(realpath $PWD/../arm-rockchip830-linux-uclibcgnueabihf)"
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_UCLIBCEABIHF_LINKER="$BUILDKIT_ROOT/bin/arm-rockchip830-linux-uclibcgnueabihf-gcc"
cargo build -Z build-std --release --target armv7-unknown-linux-uclibceabihf