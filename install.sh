#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX=${PREFIX:-/usr/local}
BINDIR="$PREFIX/bin"
LIBDIR="$PREFIX/lib"
PACKAGE_DIR="$LIBDIR/aiusage"
UNINSTALLER="$LIBDIR/aiusage-uninstall.sh"

ensure_dir() {
    dir=$1
    if [ -e "$dir" ] && [ ! -d "$dir" ]; then
        echo "Error: $dir exists but is not a directory." >&2
        exit 1
    fi
    if [ ! -d "$dir" ]; then
        install -d -m 0755 "$dir"
    fi
}

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

INSTALL_AI=1
existing_ai=$(command -v ai 2>/dev/null || true)
if [ -n "$existing_ai" ] && ! grep -q 'aiusage.manager' "$existing_ai" 2>/dev/null; then
    INSTALL_AI=0
elif [ -e "$BINDIR/ai" ] && ! grep -q 'aiusage.manager' "$BINDIR/ai" 2>/dev/null; then
    INSTALL_AI=0
fi
if [ -e "$BINDIR/aiusage" ] && ! grep -q 'aiusage.cli' "$BINDIR/aiusage" 2>/dev/null; then
    echo "Error: $BINDIR/aiusage already exists and is not managed by AIUsage." >&2
    exit 1
fi
if { [ -e "$PACKAGE_DIR" ] || [ -L "$PACKAGE_DIR" ]; } && ! { [ -d "$PACKAGE_DIR" ] && { [ -f "$PACKAGE_DIR/.aiusage-owned" ] || { [ -f "$PACKAGE_DIR/__init__.py" ] && [ -f "$PACKAGE_DIR/cli.py" ] && grep -q 'AI usage limits' "$PACKAGE_DIR/__init__.py" 2>/dev/null && grep -q 'aiusage' "$PACKAGE_DIR/cli.py" 2>/dev/null; }; }; }; then
    echo "Error: $PACKAGE_DIR already exists and is not managed by AIUsage." >&2
    exit 1
fi
if { [ -e "$UNINSTALLER" ] || [ -L "$UNINSTALLER" ]; } && ! grep -q 'AIUsage program files removed' "$UNINSTALLER" 2>/dev/null; then
    echo "Error: $UNINSTALLER already exists and is not managed by AIUsage." >&2
    exit 1
fi

ensure_dir "$BINDIR"
ensure_dir "$LIBDIR"
package_tmp="$LIBDIR/.aiusage.package.$$"
rm -rf -- "$package_tmp"
install -d -m 0755 "$package_tmp"
for source in "$SCRIPT_DIR"/src/aiusage/*.py; do
    install -m 0644 "$source" "$package_tmp/$(basename "$source")"
done
install -m 0644 /dev/null "$package_tmp/.aiusage-owned"

aiusage_tmp="$BINDIR/.aiusage.tmp.$$"
ai_tmp="$BINDIR/.ai.tmp.$$"
uninstaller_tmp="$LIBDIR/.aiusage-uninstall.tmp.$$"
trap 'rm -rf -- "$package_tmp"; rm -f -- "$aiusage_tmp" "$ai_tmp" "$uninstaller_tmp"' EXIT HUP INT TERM
sed "s|@LIBDIR@|$LIBDIR|g" "$SCRIPT_DIR/scripts/aiusage-launcher" > "$aiusage_tmp"
if [ "$INSTALL_AI" -eq 1 ]; then
    sed "s|@LIBDIR@|$LIBDIR|g" "$SCRIPT_DIR/scripts/ai-launcher" > "$ai_tmp"
    chmod 0755 "$ai_tmp"
fi
install -m 0755 "$SCRIPT_DIR/uninstall.sh" "$uninstaller_tmp"
chmod 0755 "$aiusage_tmp"
rm -rf -- "$PACKAGE_DIR"
mv "$package_tmp" "$PACKAGE_DIR"
mv -f "$aiusage_tmp" "$BINDIR/aiusage"
if [ "$INSTALL_AI" -eq 1 ]; then
    mv -f "$ai_tmp" "$BINDIR/ai"
fi
mv -f "$uninstaller_tmp" "$UNINSTALLER"
trap - EXIT HUP INT TERM

echo "✓ AIUsage 安装完成"
echo
if [ "$INSTALL_AI" -eq 1 ]; then
    echo "管理菜单："
    echo "    ai"
else
    echo "管理菜单："
    echo "    aiusage --menu"
fi
echo
echo "直接启动额度看板："
echo "    aiusage"
echo
echo "GitHub: https://github.com/hyd1aa/aiusage"
if [ "$INSTALL_AI" -eq 0 ]; then
    echo
    echo "检测到系统已有 ai 命令，因此没有安装 AIUsage 的 ai 快捷入口。"
    echo "请使用："
    echo "    aiusage --menu"
    echo "进入 AIUsage 管理菜单。"
fi
