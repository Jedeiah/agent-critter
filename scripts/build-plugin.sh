#!/usr/bin/env bash
# Build and package the traffic-light plugin.
#
# Usage:
#   build-plugin.sh                                # build current platform + package
#   build-plugin.sh windows                        # package windows (uses target/release/)
#   build-plugin.sh macos   path/to/mac-binary     # package macos with pre-built binary
#   build-plugin.sh macos-arm64 path/to/binary     # package macos-arm64 with pre-built binary
#   build-plugin.sh linux   path/to/linux-binary   # package linux with pre-built binary

set -euo pipefail

echo "=== Building Traffic Light Plugin ==="

PLATFORM="${1:-}"
BINARY_PATH="${2:-}"

# No args: detect platform and build
if [[ -z "$PLATFORM" ]]; then
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
        PLATFORM="windows"
        BINARY_PATH="target/release/traffic-light-daemon.exe"
    elif [[ "$(uname -s)" == "Darwin" ]]; then
        if [[ "$(uname -m)" == "arm64" ]]; then
            PLATFORM="macos-arm64"
        else
            PLATFORM="macos"
        fi
        BINARY_PATH="target/release/traffic-light-daemon"
    else
        PLATFORM="linux"
        BINARY_PATH="target/release/traffic-light-daemon"
    fi
    echo "Building for current platform (${PLATFORM})..."
    cargo build --release
fi

# Binary path provided directly
if [[ -n "$BINARY_PATH" ]]; then
    if [[ ! -f "$BINARY_PATH" ]]; then
        echo "Error: Binary not found: ${BINARY_PATH}" >&2
        exit 1
    fi
    echo "[PRE-BUILT] ${BINARY_PATH}"
else
    # Platform specified but no binary path — check default location
    if [[ "$PLATFORM" == "windows" ]]; then
        BINARY_PATH="target/release/traffic-light-daemon.exe"
    else
        BINARY_PATH="target/release/traffic-light-daemon"
    fi
    if [[ ! -f "$BINARY_PATH" ]]; then
        echo "Error: Binary not found at ${BINARY_PATH}. Build it first or provide a path." >&2
        exit 1
    fi
fi

PACKAGE="dist/claude-traffic-light-plugin-${PLATFORM}.zip"

echo "Platform: ${PLATFORM}"
echo "Packaging to ${PACKAGE}..."

mkdir -p dist

TMPDIR=$(mktemp -d)
trap "rm -rf ${TMPDIR}" EXIT

cp -r .claude-plugin "${TMPDIR}/"
cp -r hooks "${TMPDIR}/"
cp -r assets "${TMPDIR}/"
cp README.md "${TMPDIR}/" 2>/dev/null || true

cp "${BINARY_PATH}" "${TMPDIR}/"

cd "${TMPDIR}"
zip -r "${OLDPWD}/${PACKAGE}" .
cd "${OLDPWD}"

SIZE=$(du -h "${PACKAGE}" | cut -f1)
echo ""
echo "=== Package created ==="
echo "File: ${PACKAGE}"
echo "Size: ${SIZE}"
