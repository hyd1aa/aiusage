#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX=${PREFIX:-/usr/local}
BINDIR="$PREFIX/bin"
LIBDIR="$PREFIX/lib"
PACKAGE_DIR="$LIBDIR/aiusage"
UNINSTALLER="$LIBDIR/aiusage-uninstall.sh"

if ! command -v python3 >/dev/null 2>&1; then
    echo "Error: Python 3.10 or newer is required." >&2
    exit 1
fi

if ! python3 -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)'; then
    echo "Error: Python 3.10 or newer is required." >&2
    exit 1
fi

if [ ! -f "$SCRIPT_DIR/src/aiusage/cli.py" ]; then
    echo "Error: run install.sh from an intact AIUsage source checkout." >&2
    exit 1
fi

if [ -e "$BINDIR/ai" ] && ! grep -q 'aiusage.manager' "$BINDIR/ai" 2>/dev/null; then
    echo "Error: $BINDIR/ai already exists and is not managed by AIUsage." >&2
    exit 1
fi
if [ -e "$BINDIR/aiusage" ] && ! grep -q 'aiusage.cli' "$BINDIR/aiusage" 2>/dev/null; then
    echo "Error: $BINDIR/aiusage already exists and is not managed by AIUsage." >&2
    exit 1
fi

install -d -m 0755 "$BINDIR" "$LIBDIR"
package_tmp="$LIBDIR/.aiusage.package.$$"
rm -rf -- "$package_tmp"
install -d -m 0755 "$package_tmp"
for source in "$SCRIPT_DIR"/src/aiusage/*.py; do
    install -m 0644 "$source" "$package_tmp/$(basename "$source")"
done

aiusage_tmp="$BINDIR/.aiusage.tmp.$$"
ai_tmp="$BINDIR/.ai.tmp.$$"
uninstaller_tmp="$LIBDIR/.aiusage-uninstall.tmp.$$"
trap 'rm -rf -- "$package_tmp"; rm -f -- "$aiusage_tmp" "$ai_tmp" "$uninstaller_tmp"' EXIT HUP INT TERM
sed "s|@LIBDIR@|$LIBDIR|g" "$SCRIPT_DIR/scripts/aiusage-launcher" > "$aiusage_tmp"
sed "s|@LIBDIR@|$LIBDIR|g" "$SCRIPT_DIR/scripts/ai-launcher" > "$ai_tmp"
install -m 0755 "$SCRIPT_DIR/uninstall.sh" "$uninstaller_tmp"
chmod 0755 "$aiusage_tmp" "$ai_tmp"
rm -rf -- "$PACKAGE_DIR"
mv "$package_tmp" "$PACKAGE_DIR"
mv -f "$aiusage_tmp" "$BINDIR/aiusage"
mv -f "$ai_tmp" "$BINDIR/ai"
mv -f "$uninstaller_tmp" "$UNINSTALLER"
trap - EXIT HUP INT TERM

echo "✓ AIUsage 安装完成"
echo
echo "管理菜单："
echo "    ai"
echo
echo "直接启动额度看板："
echo "    aiusage"
echo
echo "GitHub: https://github.com/hyd1aa/aiusage"
