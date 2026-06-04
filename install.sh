#!/bin/sh
set -eu

REPO="${REPO:-caiocesaralves/shikigami}"
BIN="${BIN:-/usr/local/bin}"

case "$(uname -s)" in
  Linux)  OS=linux   ;;
  Darwin) OS=darwin  ;;
  *)      echo "Unsupported OS: $(uname -s)"; exit 1 ;;
esac

case "$(uname -m)" in
  x86_64|amd64) ARCH=x86_64 ;;
  aarch64|arm64) ARCH=aarch64 ;;
  *)            echo "Unsupported arch: $(uname -m)"; exit 1 ;;
esac

ASSET="shikigami-${ARCH}-${OS}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
SUM_URL="${URL}.sha256"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "Downloading ${URL}..."
curl -sSfL "$URL" -o "$tmp/$ASSET"
curl -sSfL "$SUM_URL" -o "$tmp/$ASSET.sha256"

echo "Verifying checksum..."
(cd "$tmp" && sha256sum -c "$ASSET.sha256")

echo "Extracting..."
tar xzf "$tmp/$ASSET" -C "$tmp"

install -m755 "$tmp/shikigami" "$BIN/shikigami"
echo "Installed shikigami to $BIN/shikigami"
