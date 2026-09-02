#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX=${PREFIX:-/usr/local}
BINDIR="$PREFIX/bin"
LIBDIR="$PREFIX/lib"
PACKAGE_DIR="$LIBDIR/aiusage"

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

install -d -m 0755 "$BINDIR" "$PACKAGE_DIR"
for source in "$SCRIPT_DIR"/src/aiusage/*.py; do
    install -m 0644 "$source" "$PACKAGE_DIR/$(basename "$source")"
done

launcher_tmp="$BINDIR/.aiusage.tmp.$$"
trap 'rm -f "$launcher_tmp"' EXIT HUP INT TERM
sed "s|@LIBDIR@|$LIBDIR|g" "$SCRIPT_DIR/scripts/aiusage-launcher" > "$launcher_tmp"
chmod 0755 "$launcher_tmp"
mv -f "$launcher_tmp" "$BINDIR/aiusage"
trap - EXIT HUP INT TERM

echo "AIUsage installed: $BINDIR/aiusage"

