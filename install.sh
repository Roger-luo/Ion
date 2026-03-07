#!/bin/sh
# Ion installer — https://github.com/Roger-luo/Ion
# Usage: curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh
set -eu

REPO="Roger-luo/Ion"
INSTALL_DIR="${ION_INSTALL_DIR:-${HOME}/.local/bin}"

main() {
    detect_platform
    fetch_latest_version
    download_and_install
    print_success
}

detect_platform() {
    OS=$(uname -s)
    ARCH=$(uname -m)

    case "$OS" in
        Linux)  OS_TARGET="unknown-linux-gnu" ;;
        Darwin) OS_TARGET="apple-darwin" ;;
        *)      err "Unsupported OS: $OS" ;;
    esac

    case "$ARCH" in
        x86_64|amd64)  ARCH_TARGET="x86_64" ;;
        aarch64|arm64) ARCH_TARGET="aarch64" ;;
        *)             err "Unsupported architecture: $ARCH" ;;
    esac

    TARGET="${ARCH_TARGET}-${OS_TARGET}"
    log "Detected platform: $TARGET"
}

fetch_latest_version() {
    log "Fetching latest release..."
    # Get the latest ion-v* release (skip ion-skill-v* releases)
    RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases?per_page=10" \
        -H "Accept: application/vnd.github+json")

    TAG=$(printf '%s' "$RELEASE_JSON" | grep -o '"tag_name": *"ion-v[^"]*"' | head -1 | sed 's/.*"ion-v//' | sed 's/"//')

    if [ -z "$TAG" ]; then
        err "Could not find latest ion release"
    fi

    VERSION="$TAG"
    log "Latest version: $VERSION"
}

download_and_install() {
    ARCHIVE="ion-${VERSION}-${TARGET}.tar.gz"
    URL="https://github.com/${REPO}/releases/download/ion-v${VERSION}/${ARCHIVE}"

    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    log "Downloading $ARCHIVE..."
    if ! curl -fsSL "$URL" -o "${TMPDIR}/${ARCHIVE}"; then
        err "Failed to download $URL\nNo prebuilt binary for $TARGET. Install from source:\n  cargo install --git https://github.com/${REPO}"
    fi

    log "Extracting..."
    tar xzf "${TMPDIR}/${ARCHIVE}" -C "$TMPDIR"

    mkdir -p "$INSTALL_DIR"
    mv "${TMPDIR}/ion" "${INSTALL_DIR}/ion"
    chmod +x "${INSTALL_DIR}/ion"
}

print_success() {
    log "Installed ion $VERSION to ${INSTALL_DIR}/ion"

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo ""
            warn "${INSTALL_DIR} is not in your PATH. Add it with:"
            echo ""
            echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
            echo ""
            echo "Or add that line to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
            ;;
    esac
}

log()  { printf '  \033[1;32m>\033[0m %s\n' "$*"; }
warn() { printf '  \033[1;33m!\033[0m %s\n' "$*"; }
err()  { printf '  \033[1;31mx\033[0m %s\n' "$*" >&2; exit 1; }

main
