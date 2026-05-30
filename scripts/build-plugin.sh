#!/usr/bin/env bash
# Build and package the agent-critter plugin.
set -euo pipefail

echo "=== Building Agent Critter Plugin ==="

PLATFORM="${1:-}"
BINARY_PATH="${2:-}"

if [[ -z "$PLATFORM" ]]; then
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
        PLATFORM="windows"; BINARY_PATH="target/release/agent-critter.exe"
    elif [[ "$(uname -s)" == "Darwin" ]]; then
        PLATFORM="macos"; BINARY_PATH="target/release/agent-critter"
    else
        PLATFORM="linux"; BINARY_PATH="target/release/agent-critter"
    fi
    echo "Building for ${PLATFORM}..."
    cargo build --release
fi

if [[ -z "$BINARY_PATH" ]]; then
    if [[ "$PLATFORM" == "windows" ]]; then
        BINARY_PATH="target/release/agent-critter.exe"
    else
        BINARY_PATH="target/release/agent-critter"
    fi
fi

if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Error: Binary not found: ${BINARY_PATH}" >&2
    exit 1
fi

PACKAGE="dist/agent-critter-plugin-${PLATFORM}.zip"
echo "Platform: ${PLATFORM}"
echo "Packaging to ${PACKAGE}..."

mkdir -p dist

TMPDIR=$(mktemp -d)
trap "rm -rf ${TMPDIR}" EXIT

cp -r .claude-plugin "${TMPDIR}/"
cp .claude-plugin/marketplace.json "${TMPDIR}/"
cp -r hooks "${TMPDIR}/"
cp -r assets "${TMPDIR}/"
cp PLUGIN.md "${TMPDIR}/" 2>/dev/null || true
cp README.md "${TMPDIR}/" 2>/dev/null || true

mkdir -p "${TMPDIR}/bin"
cp "${BINARY_PATH}" "${TMPDIR}/bin/"

cd "${TMPDIR}"
zip -r "${OLDPWD}/${PACKAGE}" .
cd "${OLDPWD}"

SIZE=$(du -h "${PACKAGE}" | cut -f1)
echo ""
echo "=== Package created ==="
echo "File: ${PACKAGE}"
echo "Size: ${SIZE}"
