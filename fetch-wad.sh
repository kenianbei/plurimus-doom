#!/usr/bin/env sh
set -eu

VERSION="0.13.0"
URL="https://github.com/freedoom/freedoom/releases/download/v${VERSION}/freedoom-${VERSION}.zip"

if [ -f freedoom1.wad ]; then
    echo "freedoom1.wad already present"
    exit 0
fi
command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v unzip >/dev/null || { echo "unzip is required" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fL "$URL" -o "$tmp/freedoom.zip"
unzip -j "$tmp/freedoom.zip" "freedoom-${VERSION}/freedoom1.wad" -d .
echo "freedoom1.wad ready"
