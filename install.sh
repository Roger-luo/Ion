#!/bin/sh
# Ion installer — https://github.com/Roger-luo/Ion
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh -s -- 0.1.2
set -eu

REPO="Roger-luo/Ion"
INSTALL_DIR="${ION_INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${1:-}"

main() {
    detect_platform
    resolve_version
    check_existing
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

resolve_version() {
    if [ -n "$VERSION" ]; then
        # Strip leading 'v' if provided (e.g. v0.1.2 -> 0.1.2)
        VERSION="${VERSION#v}"
        log "Requested version: $VERSION"
    else
        log "Fetching latest release..."
        RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases?per_page=10" \
            -H "Accept: application/vnd.github+json")

        # Find the first ion-v* release that has assets (skip empty releases
        # where CI hasn't finished building binaries yet)
        TAG=""
        for candidate in $(printf '%s' "$RELEASE_JSON" | grep -o '"tag_name": *"ion-v[^"]*"' | sed 's/.*"ion-v//' | sed 's/"//'); do
            # Check if this release has any .tar.gz assets
            if printf '%s' "$RELEASE_JSON" | grep -q "ion-${candidate}-.*\\.tar\\.gz"; then
                TAG="$candidate"
                break
            fi
        done

        if [ -z "$TAG" ]; then
            err "Could not find latest ion release with prebuilt binaries"
        fi

        VERSION="$TAG"
        log "Latest version: $VERSION"
    fi
}

check_existing() {
    EXISTING=""

    # Check the target install directory first
    if [ -x "${INSTALL_DIR}/ion" ]; then
        EXISTING="${INSTALL_DIR}/ion"
    # Also check if ion is elsewhere on PATH
    elif command -v ion >/dev/null 2>&1; then
        EXISTING=$(command -v ion)
    fi

    if [ -z "$EXISTING" ]; then
        return
    fi

    # Get the installed version
    # Try --version first (>= 0.1.12), then self info (older), then "unknown"
    INSTALLED_VERSION=""
    VER_OUTPUT=$("$EXISTING" --version 2>/dev/null) && \
        INSTALLED_VERSION=$(printf '%s' "$VER_OUTPUT" | head -1 | sed 's/[^0-9.]*//')
    if [ -z "$INSTALLED_VERSION" ]; then
        VER_OUTPUT=$("$EXISTING" self info 2>/dev/null) && \
            INSTALLED_VERSION=$(printf '%s' "$VER_OUTPUT" | head -1 | sed 's/[^0-9.]*//')
    fi
    if [ -z "$INSTALLED_VERSION" ]; then
        INSTALLED_VERSION="unknown"
    fi

    if [ "$INSTALLED_VERSION" = "$VERSION" ]; then
        log "ion $VERSION is already installed at $EXISTING"
        exit 0
    fi

    log "Found existing ion $INSTALLED_VERSION at $EXISTING"

    # Non-interactive (piped) — proceed without prompting
    if [ ! -t 0 ] || [ ! -t 1 ]; then
        log "Upgrading ion $INSTALLED_VERSION -> $VERSION"
        return
    fi

    printf '  \033[1;33m!\033[0m Replace ion %s with %s? [Y/n] ' "$INSTALLED_VERSION" "$VERSION"
    read -r REPLY </dev/tty
    case "$REPLY" in
        [nN]|[nN][oO])
            log "Aborted."
            exit 0
            ;;
    esac
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

    setup_completions
}

detect_shell() {
    CURRENT_SHELL=""
    if [ -n "${SHELL:-}" ]; then
        case "$SHELL" in
            */bash) CURRENT_SHELL="bash" ;;
            */zsh)  CURRENT_SHELL="zsh" ;;
            */fish) CURRENT_SHELL="fish" ;;
        esac
    fi
}

setup_completions() {
    # Only prompt in interactive terminals
    if [ ! -t 0 ] || [ ! -t 1 ]; then
        return
    fi

    detect_shell

    ION="${INSTALL_DIR}/ion"

    # Verify ion binary works before offering completions
    if ! "$ION" --version >/dev/null 2>&1; then
        return
    fi

    echo ""
    if [ -n "$CURRENT_SHELL" ]; then
        printf '  \033[1;32m>\033[0m Set up shell completions for %s? [Y/n] ' "$CURRENT_SHELL"
        read -r REPLY </dev/tty
        case "$REPLY" in
            [nN]|[nN][oO]) return ;;
        esac
        install_completion "$CURRENT_SHELL"
    else
        printf '  \033[1;32m>\033[0m Set up shell completions? (bash/zsh/fish/n) '
        read -r REPLY </dev/tty
        case "$REPLY" in
            bash|zsh|fish) install_completion "$REPLY" ;;
            *) return ;;
        esac
    fi
}

install_completion() {
    COMP_SHELL="$1"
    ION="${INSTALL_DIR}/ion"

    case "$COMP_SHELL" in
        bash)
            COMP_FILE="${HOME}/.bashrc"
            echo "" >> "$COMP_FILE"
            echo '# Ion shell completions' >> "$COMP_FILE"
            echo 'eval "$('"$ION"' completion bash)"' >> "$COMP_FILE"
            log "Added completions to $COMP_FILE"
            log "Run 'source $COMP_FILE' or restart your shell to activate"
            ;;
        zsh)
            COMP_DIR="${HOME}/.zfunc"
            mkdir -p "$COMP_DIR"
            "$ION" completion zsh > "${COMP_DIR}/_ion"
            log "Installed completions to ${COMP_DIR}/_ion"
            if ! grep -q 'fpath.*\.zfunc' "${HOME}/.zshrc" 2>/dev/null; then
                echo "" >> "${HOME}/.zshrc"
                echo '# Ion shell completions' >> "${HOME}/.zshrc"
                echo 'fpath=(~/.zfunc $fpath)' >> "${HOME}/.zshrc"
                echo 'autoload -Uz compinit && compinit' >> "${HOME}/.zshrc"
                log "Added ~/.zfunc to fpath in ~/.zshrc"
            fi
            log "Run 'source ~/.zshrc' or restart your shell to activate"
            ;;
        fish)
            COMP_DIR="${HOME}/.config/fish/completions"
            mkdir -p "$COMP_DIR"
            "$ION" completion fish > "${COMP_DIR}/ion.fish"
            log "Installed completions to ${COMP_DIR}/ion.fish"
            log "Completions will be active in new fish sessions"
            ;;
    esac
}

log()  { printf '  \033[1;32m>\033[0m %s\n' "$*"; }
warn() { printf '  \033[1;33m!\033[0m %s\n' "$*"; }
err()  { printf '  \033[1;31mx\033[0m %s\n' "$*" >&2; exit 1; }

main
