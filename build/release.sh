#!/bin/bash
# Release script for Ajigent

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

VERSION="${1:-$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)}"
RELEASE_DIR="$PROJECT_DIR/release"

echo "Building Ajigent v${VERSION}..."
echo ""

# Clean release directory
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

# Build for all targets
echo "Building for aarch64..."
cargo build --release --target aarch64-unknown-linux-gnu 2>/dev/null || echo "aarch64 build skipped (cross-compilation not configured)"

echo "Building for armv7..."
cargo build --release --target armv7-unknown-linux-gnueabihf 2>/dev/null || echo "armv7 build skipped (cross-compilation not configured)"

echo "Building for x86_64..."
cargo build --release --target x86_64-unknown-linux-gnu 2>/dev/null || cargo build --release

# Copy binaries to release directory
for target in aarch64-unknown-linux-gnu armv7-unknown-linux-gnueabihf x86_64-unknown-linux-gnu; do
    binary="target/${target}/release/ajigent"
    if [ -f "$binary" ]; then
        arch=$(echo "$target" | cut -d'-' -f1)
        cp "$binary" "$RELEASE_DIR/ajigent-${VERSION}-linux-${arch}"
        echo "Created: ajigent-${VERSION}-linux-${arch}"
    fi
done

# Also copy native build if exists
if [ -f "target/release/ajigent" ]; then
    cp "target/release/ajigent" "$RELEASE_DIR/ajigent-${VERSION}-native"
    echo "Created: ajigent-${VERSION}-native"
fi

# Generate checksums
cd "$RELEASE_DIR"
sha256sum ajigent-* > checksums.sha256
echo ""
echo "Checksums:"
cat checksums.sha256

echo ""
echo "Release artifacts in: $RELEASE_DIR"
ls -la "$RELEASE_DIR"
