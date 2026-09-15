#!/bin/sh
set -eu
PREFIX=${PREFIX:-/usr/local}
PACKAGE="$PREFIX/lib/aiusage"
if [ -d "$PACKAGE" ] && [ ! -L "$PACKAGE" ] && [ -f "$PACKAGE/.aiusage-rust-owned" ]; then
    for name in aiusage ai; do
        target="$PREFIX/bin/$name"
        if [ -L "$target" ] && [ "$(readlink "$target")" = '../lib/aiusage/aiusage' ]; then rm -f -- "$target"; fi
    done
    rm -rf -- "$PACKAGE"
fi
target="$PREFIX/lib/aiusage-uninstall.sh"
if [ -f "$target" ] && grep -q 'aiusage-rust-owned' "$target"; then rm -f -- "$target"; fi
echo 'AIUsage program files removed.'
echo 'User config preserved: ~/.config/aiusage/'
