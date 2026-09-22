#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX=${PREFIX:-/usr/local}
BINDIR="$PREFIX/bin"
LIBDIR="$PREFIX/lib"
PACKAGE="$LIBDIR/aiusage"
BINARY=${AIUSAGE_BINARY:-"$SCRIPT_DIR/rust/target/release/aiusage"}
UNINSTALLER="$LIBDIR/aiusage-uninstall.sh"

ensure_dir() {
    if { [ -e "$1" ] || [ -L "$1" ]; } && [ ! -d "$1" ]; then
        echo "Error: $1 exists but is not a directory." >&2
        exit 1
    fi
    if [ ! -d "$1" ]; then install -d -m 0755 "$1"; fi
}

owned_launcher() {
    [ -L "$1" ] && [ "$(readlink "$1")" = '../lib/aiusage/aiusage' ] &&
        [ -f "$PACKAGE/.aiusage-rust-owned" ]
}

if [ ! -f "$BINARY" ] || [ ! -x "$BINARY" ]; then
    echo 'Error: build or supply a verified AIUsage Rust binary first.' >&2
    exit 1
fi
if [ -L "$PACKAGE" ] || { [ -e "$PACKAGE" ] && ! { [ -d "$PACKAGE" ] && [ -f "$PACKAGE/.aiusage-rust-owned" ]; }; }; then
    echo 'Error: package path is not managed by AIUsage.' >&2
    exit 1
fi
if { [ -e "$BINDIR/aiusage" ] || [ -L "$BINDIR/aiusage" ]; } &&
    ! owned_launcher "$BINDIR/aiusage"; then
    echo 'Error: aiusage command is not managed by AIUsage.' >&2
    exit 1
fi
if { [ -e "$UNINSTALLER" ] || [ -L "$UNINSTALLER" ]; } &&
    ! grep -q 'AIUsage program files removed' "$UNINSTALLER" 2>/dev/null; then
    echo 'Error: uninstaller is not managed by AIUsage.' >&2
    exit 1
fi

INSTALL_AI=1
existing_ai=$(command -v ai 2>/dev/null || true)
if [ -n "$existing_ai" ] && ! owned_launcher "$existing_ai"; then
    INSTALL_AI=0
fi
if { [ -e "$BINDIR/ai" ] || [ -L "$BINDIR/ai" ]; } &&
    ! owned_launcher "$BINDIR/ai"; then
    INSTALL_AI=0
fi

ensure_dir "$BINDIR"
ensure_dir "$LIBDIR"
stage=$(mktemp -d "$LIBDIR/.aiusage-rust.XXXXXX")
success=0
publishing=0
old_package=0
new_package=0
had_usage=0
had_ai=0
had_uninstaller=0

cleanup() {
    result=$?
    trap - EXIT HUP INT TERM
    if [ "$success" -eq 0 ] && [ "$publishing" -eq 1 ]; then
        if [ "$new_package" -eq 1 ]; then rm -rf -- "$PACKAGE"; fi
        if [ "$old_package" -eq 1 ]; then mv "$stage/previous-package" "$PACKAGE"; fi
        if [ "$had_usage" -eq 1 ]; then
            mv -f "$stage/previous-usage" "$BINDIR/aiusage"
        else
            rm -f "$BINDIR/aiusage"
        fi
        if [ "$INSTALL_AI" -eq 1 ]; then
            if [ "$had_ai" -eq 1 ]; then
                mv -f "$stage/previous-ai" "$BINDIR/ai"
            else
                rm -f "$BINDIR/ai"
            fi
        fi
        if [ "$had_uninstaller" -eq 1 ]; then
            mv -f "$stage/previous-uninstaller" "$UNINSTALLER"
        else
            rm -f "$UNINSTALLER"
        fi
    fi
    rm -rf -- "$stage"
    exit "$result"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

install -d -m 0755 "$stage/package"
install -m 0755 "$BINARY" "$stage/package/aiusage"
install -m 0644 /dev/null "$stage/package/.aiusage-owned"
install -m 0644 /dev/null "$stage/package/.aiusage-rust-owned"
install -m 0755 "$SCRIPT_DIR/uninstall.sh" "$stage/uninstall.sh"
ln -s '../lib/aiusage/aiusage' "$stage/aiusage"
ln -s '../lib/aiusage/aiusage' "$stage/ai"

if [ -e "$BINDIR/aiusage" ] || [ -L "$BINDIR/aiusage" ]; then
    cp -pP "$BINDIR/aiusage" "$stage/previous-usage"
    had_usage=1
fi
if [ "$INSTALL_AI" -eq 1 ] && { [ -e "$BINDIR/ai" ] || [ -L "$BINDIR/ai" ]; }; then
    cp -pP "$BINDIR/ai" "$stage/previous-ai"
    had_ai=1
fi
if [ -e "$UNINSTALLER" ] || [ -L "$UNINSTALLER" ]; then
    cp -pP "$UNINSTALLER" "$stage/previous-uninstaller"
    had_uninstaller=1
fi

publishing=1
if [ -d "$PACKAGE" ]; then
    mv "$PACKAGE" "$stage/previous-package"
    old_package=1
fi
mv "$stage/package" "$PACKAGE"
new_package=1
mv -f "$stage/aiusage" "$BINDIR/aiusage"
if [ "$INSTALL_AI" -eq 1 ]; then mv -f "$stage/ai" "$BINDIR/ai"; fi
mv -f "$stage/uninstall.sh" "$UNINSTALLER"
success=1

echo '✓ AIUsage Rust installation complete'
echo 'Management menu: aiusage --menu'
echo 'Dashboard: aiusage'
if [ "$INSTALL_AI" -eq 0 ]; then echo 'The third-party ai command was preserved.'; fi
