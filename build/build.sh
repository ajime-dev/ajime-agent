#!/bin/bash
# Build script for Ajime Agent

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Default target
TARGET="${1:-native}"

echo "Building Ajime Agent..."
echo "Target: $TARGET"

case "$TARGET" in
    native)
        cargo build --release
        ;;
    aarch64|arm64|raspberry-pi-64|jetson)
        echo "Building for aarch64-unknown-linux-gnu..."
        cargo build --release --target aarch64-unknown-linux-gnu
        # Strip binary
        if command -v aarch64-linux-gnu-strip &> /dev/null; then
            aarch64-linux-gnu-strip target/aarch64-unknown-linux-gnu/release/ajime-agent
        fi
        ;;
    armv7|arm32|raspberry-pi-32)
        echo "Building for armv7-unknown-linux-gnueabihf..."
        cargo build --release --target armv7-unknown-linux-gnueabihf
        # Strip binary
        if command -v arm-linux-gnueabihf-strip &> /dev/null; then
            arm-linux-gnueabihf-strip target/armv7-unknown-linux-gnueabihf/release/ajime-agent
        fi
        ;;
    x86_64|amd64)
        echo "Building for x86_64-unknown-linux-gnu..."
        cargo build --release --target x86_64-unknown-linux-gnu
        strip target/x86_64-unknown-linux-gnu/release/ajime-agent
        ;;
    all)
        echo "Building for all targets..."
        $0 aarch64
        $0 armv7
        $0 x86_64
        ;;
    *)
        echo "Unknown target: $TARGET"
        echo "Usage: $0 [native|aarch64|armv7|x86_64|all]"
        exit 1
        ;;
esac

echo "Build complete!"
