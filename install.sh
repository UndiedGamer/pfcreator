#!/bin/bash

set -e

echo "Welcome to the pfcreator installer!"
echo "This script will install pfcreator on your system."
RELEASE_URL="https://github.com/UndiedGamer/pfcreator/releases/download/v1.0.0"

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

TARBALL=""
if [ "$OS" = "linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        TARBALL="pfcreator-linux-x86_64.tar.gz"
    elif [ "$ARCH" = "arm64" ]; then
        TARBALL="pfcreator-linux-aarch64.tar.gz"
    fi
elif [ "$OS" = "darwin" ]; then
    TARBALL="pfcreator-darwin-aarch64.tar.gz"
else
    echo "Unsupported OS: $OS"
    exit 1
fi

if [ -z "$TARBALL" ]; then
    echo "Unsupported architecture: $ARCH"
    exit 1
fi

echo "Downloading $TARBALL..."
curl -fsSL "$RELEASE_URL/$TARBALL" -o "$TEMP_DIR/$TARBALL"

echo "Extracting $TARBALL..."
tar -xzf "$TEMP_DIR/$TARBALL" -C "$TEMP_DIR"

echo "Installing binaries..."
for binary in "$TEMP_DIR"/*/*; do
    if [ -x "$binary" ] && [ ! -d "$binary" ]; then
        echo "Copying $(basename "$binary") to /usr/local/bin"
        sudo cp "$binary" /usr/local/bin/
        sudo chmod +x "/usr/local/bin/$(basename "$binary")"
    fi
done

echo "pfcreator has been successfully installed!"
exit 0
