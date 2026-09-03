#!/bin/sh
set -eu

PREFIX=${PREFIX:-/usr/local}
BINARY="$PREFIX/bin/aiusage"
MANAGER="$PREFIX/bin/ai"
PACKAGE_DIR="$PREFIX/lib/aiusage"
UNINSTALLER="$PREFIX/lib/aiusage-uninstall.sh"

if { [ -e "$BINARY" ] || [ -L "$BINARY" ]; } && grep -q 'aiusage.cli' "$BINARY" 2>/dev/null; then
    rm -f -- "$BINARY"
fi
if { [ -e "$MANAGER" ] || [ -L "$MANAGER" ]; } && grep -q 'aiusage.manager' "$MANAGER" 2>/dev/null; then
    rm -f -- "$MANAGER"
fi
if [ -d "$PACKAGE_DIR" ] && { [ -f "$PACKAGE_DIR/.aiusage-owned" ] || { [ -f "$PACKAGE_DIR/__init__.py" ] && [ -f "$PACKAGE_DIR/cli.py" ] && grep -q 'AI usage limits' "$PACKAGE_DIR/__init__.py" 2>/dev/null && grep -q 'aiusage' "$PACKAGE_DIR/cli.py" 2>/dev/null; }; }; then
    rm -rf -- "$PACKAGE_DIR"
fi
if { [ -e "$UNINSTALLER" ] || [ -L "$UNINSTALLER" ]; } && grep -q 'AIUsage program files removed' "$UNINSTALLER" 2>/dev/null; then
    rm -f -- "$UNINSTALLER"
fi

echo "AIUsage program files removed."
echo "User config preserved: ~/.config/aiusage/"
