#!/usr/bin/env bash
set -euo pipefail

# jc installer — builds and installs the Claude Code orchestrator.
# Usage: ./install.sh [--prefix /usr/local]

PREFIX="${1:-$HOME/.local}"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/jc-build}"

echo "==> Installing jc to $PREFIX/bin/"

# Check prerequisites.
command -v cargo >/dev/null 2>&1 || {
    echo "error: cargo is not installed. Install via https://rustup.rs"
    exit 1
}

# Check system deps.
missing_deps=()
pkg-config --exists fontconfig 2>/dev/null || missing_deps+=(libfontconfig1-dev)
pkg-config --exists openssl 2>/dev/null || missing_deps+=(libssl-dev)
if [ ${#missing_deps[@]} -gt 0 ]; then
    echo "error: missing system dependencies: ${missing_deps[*]}"
    echo "Install with: sudo apt install ${missing_deps[*]}"
    exit 1
fi

# Build release binary.
echo "==> Building release binary..."
CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo build --release -p jc-app

# Install binary.
mkdir -p "$PREFIX/bin"
cp "$CARGO_TARGET_DIR/release/jc-app" "$PREFIX/bin/jc"
strip "$PREFIX/bin/jc" 2>/dev/null || true
echo "==> Installed $PREFIX/bin/jc ($(du -h "$PREFIX/bin/jc" | cut -f1))"

# Install .desktop file (for WSLg / Linux desktop).
if [ -d "$HOME/.local/share/applications" ]; then
    cp jc-app/jc.desktop "$HOME/.local/share/applications/jc.desktop"
    echo "==> Installed desktop entry"
fi

# Verify PATH.
if ! echo "$PATH" | tr ':' '\n' | grep -q "^$PREFIX/bin$"; then
    echo ""
    echo "NOTE: $PREFIX/bin is not in your PATH."
    echo "Add to your shell config:"
    echo "  export PATH=\"$PREFIX/bin:\$PATH\""
fi

echo "==> Done. Run 'jc .' to start."
