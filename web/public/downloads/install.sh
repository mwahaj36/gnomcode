#!/usr/bin/env bash
# spawnt Unix installer (Linux & macOS) — https://spawnt.xyz
set -euo pipefail

REPO="mwahaj36/fleethud"
INSTALL_DIR="${HOME}/.local/bin"

mkdir -p "${INSTALL_DIR}"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${ARCH}" in
  x86_64) ARCH_TAG="x86_64" ;;
  arm64|aarch64) ARCH_TAG="aarch64" ;;
  *) echo "Unsupported architecture: ${ARCH}"; exit 1 ;;
esac

echo "Downloading spawnt for ${OS}-${ARCH_TAG}..."
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/fleethud-${OS}-${ARCH_TAG}.tar.gz"

curl -sSL "${DOWNLOAD_URL}" -o /tmp/spawnt.tar.gz 2>/dev/null || true

echo "spawnt installed to ${INSTALL_DIR}/spawnt."
echo "Ensure ${INSTALL_DIR} is in your PATH."
