#!/bin/sh
set -eu

PREFIX=${PREFIX:-/usr/local}
BINARY="$PREFIX/bin/aiusage"
PACKAGE_DIR="$PREFIX/lib/aiusage"

if [ -e "$BINARY" ] || [ -L "$BINARY" ]; then
    rm -f -- "$BINARY"
fi
if [ -d "$PACKAGE_DIR" ]; then
    rm -rf -- "$PACKAGE_DIR"
fi

echo "AIUsage program files removed."
echo "User config preserved: ~/.config/aiusage/"

