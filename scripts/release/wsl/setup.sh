#!/bin/bash
# One-time setup of WSL's Ubuntu 22.04 for `just release-wsl` (RUNBOOK "A hand-given build").
# Run it as root from the repository's root, again whenever this file changes (every step is
# skipped when already done):
#
#   wsl -d Ubuntu-22.04 -u root -- bash scripts/release/wsl/setup.sh
#
# Ubuntu 22.04 because its C library (glibc 2.35) is the release workflow's floor: a binary
# built on a newer one (the VPS's 2.39) does not start on 22.04, Debian 12 or Mint 21.
set -euo pipefail

# `cargo-about` at the release workflow's version; `just` at the version the maintainer's
# machine runs, so the two parse the justfile alike.
JUST_VERSION=1.58.0
CARGO_ABOUT_VERSION=0.9.2
# Release builds run as this user, not root: `release.py` refuses a binary holding the build
# machine's home folder, and `/root` also sits inside a dependency's D-Bus paths
# (`/org/a11y/atspi/accessible/root`).
BUILD_USER=utwbuild

repo=$(pwd)
channel=$(sed -n 's/^channel = "\(.*\)"$/\1/p' "$repo/rust-toolchain.toml")
[ -n "$channel" ] || { echo "error: run from the repository's root (no rust-toolchain.toml channel)"; exit 1; }

export DEBIAN_FRONTEND=noninteractive
apt-get update -q
# The compiler's tools, `release.py`'s (objcopy and readelf are binutils), and the GUI crates'
# build dependencies.
apt-get install -y -q build-essential pkg-config curl git binutils software-properties-common \
    libssl-dev libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev \
    libxi-dev libgl1-mesa-dev libfontconfig1-dev libudev-dev
# Python 3.13, the release workflow's: the release scripts read TOML with `tomllib` (3.11+),
# and 22.04's own python3 is 3.10. It stays the system's python3, which apt needs.
if [ ! -x /usr/bin/python3.13 ]; then
    add-apt-repository -y ppa:deadsnakes/ppa
    apt-get install -y -q python3.13
fi

id "$BUILD_USER" >/dev/null 2>&1 || useradd -m -s /bin/bash "$BUILD_USER"
su - "$BUILD_USER" -c "set -euo pipefail
    if [ ! -x \"\$HOME/.cargo/bin/rustup\" ]; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none --profile minimal
    fi
    . \"\$HOME/.cargo/env\"
    rustup toolchain install $channel --profile minimal
    # python3 is 3.13 for the build alone: build.sh puts ~/bin first on its PATH.
    mkdir -p \"\$HOME/bin\"
    ln -sf /usr/bin/python3.13 \"\$HOME/bin/python3\"
    ln -sf /usr/bin/python3.13 \"\$HOME/bin/python\"
    # The source repository sits on the Windows drive, owned by another user.
    git config --global --get-all safe.directory | grep -qx '\*' || git config --global --add safe.directory '*'
    just --version 2>/dev/null | grep -q ' $JUST_VERSION\$' || cargo +$channel install --locked just@$JUST_VERSION
    cargo about --version 2>/dev/null | grep -q ' $CARGO_ABOUT_VERSION\$' || cargo +$channel install --locked cargo-about@$CARGO_ABOUT_VERSION --features cli
    just --version; cargo about --version; \"\$HOME/bin/python3\" --version"
echo "ready: $(ldd --version | sed -n 1p)"
