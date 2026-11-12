#!/usr/bin/env bash
# anirel installer — curl -fsSL https://raw.githubusercontent.com/Praveensenpai/anirel/main/install.sh | bash
set -euo pipefail

REPO="Praveensenpai/anirel"
BINARY_NAME="anirel"
INSTALL_DIR="${HOME}/.local/bin"

echo "🌸 Installing anirel..."

OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "${OS}" != "Linux" ]; then
    echo "❌ Currently only Linux is supported by this installer."
    exit 1
fi

if [ "${ARCH}" != "x86_64" ]; then
    echo "❌ Currently only x86_64 is supported. Build from source instead:"
    echo "   cargo install --git https://github.com/${REPO}.git"
    exit 1
fi

TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')"

if [ -z "${TAG}" ]; then
    echo "⚠️  No official release tag found. Building from source via cargo..."
    if command -v cargo >/dev/null 2>&1; then
        cargo install --git "https://github.com/${REPO}.git"
        echo "✔ Installed anirel via cargo!"
        exit 0
    fi
    echo "❌ Cargo is not installed. Please install Rust or wait for a binary release."
    exit 1
fi

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${BINARY_NAME}-x86_64-linux.tar.gz"
CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

mkdir -p "${INSTALL_DIR}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "📥 Downloading anirel ${TAG}..."
curl -fsSL "${DOWNLOAD_URL}" -o "${TMP_DIR}/${BINARY_NAME}.tar.gz"

echo "🔒 Verifying checksum..."
CHECKSUM_LINE="$(curl -fsSL "${CHECKSUM_URL}")"
EXPECTED="$(echo "${CHECKSUM_LINE}" | awk '{print $1}')"
ACTUAL="$(sha256sum "${TMP_DIR}/${BINARY_NAME}.tar.gz" | awk '{print $1}')"
if [ "${EXPECTED}" != "${ACTUAL}" ]; then
    echo "❌ Checksum mismatch! Aborting."
    echo "   expected: ${EXPECTED}"
    echo "   actual:   ${ACTUAL}"
    exit 1
fi
echo "✔ Checksum matches."

tar -xzf "${TMP_DIR}/${BINARY_NAME}.tar.gz" -C "${TMP_DIR}"
install -m 755 "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

echo "✔ Installed anirel ${TAG} to ${INSTALL_DIR}/${BINARY_NAME}!"

if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
    echo "⚠️  Note: ${INSTALL_DIR} is not in your PATH."
    echo "   Add it to your shell config (~/.bashrc or ~/.zshrc):"
    echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
