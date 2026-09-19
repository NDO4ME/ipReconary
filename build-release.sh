#!/bin/bash
# Release build script for ipReconary
# Builds for available targets on this system

set -e

VERSION="1.0.0"
NAME="ipReconary"
RELEASE_DIR="release"

# Create release directory
mkdir -p "$RELEASE_DIR"

echo "Building $NAME v$VERSION..."
echo ""

# Function to build for a target
build_target() {
    local TARGET="$1"
    local OUTPUT_NAME="$2"

    echo "==> Building for $TARGET..."

    # Add target if not exists
    rustup target add "$TARGET" 2>/dev/null || true

    # Build
    if cargo build --release --target "$TARGET" 2>/dev/null; then
        # Determine binary name
        if [[ "$TARGET" == *"windows"* ]]; then
            BIN_NAME="${NAME}.exe"
        else
            BIN_NAME="$NAME"
        fi

        SRC="target/$TARGET/release/$BIN_NAME"
        if [ -f "$SRC" ]; then
            cp "$SRC" "$RELEASE_DIR/$OUTPUT_NAME"
            echo "    [OK] $OUTPUT_NAME ($(du -h "$RELEASE_DIR/$OUTPUT_NAME" | cut -f1))"
            return 0
        fi
    fi

    echo "    [SKIP] $TARGET"
    return 1
}

# Build for Linux x86_64 (glibc)
build_target "x86_64-unknown-linux-gnu" "ipReconary-linux-amd64"

# Build for Linux x86_64 (musl - static)
build_target "x86_64-unknown-linux-musl" "ipReconary-linux-amd64-static"

# Build for Linux ARM64 (if cross-compiler available)
if command -v aarch64-linux-gnu-gcc &> /dev/null; then
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
    build_target "aarch64-unknown-linux-gnu" "ipReconary-linux-arm64"
fi

# Build for Windows (if mingw available)
if command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    build_target "x86_64-pc-windows-gnu" "ipReconary-windows-amd64.exe"
fi

echo ""
echo "Release builds complete:"
echo ""
ls -lh "$RELEASE_DIR/"
