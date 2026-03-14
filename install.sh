#!/bin/sh
# Ion installer — https://github.com/Roger-luo/Ion
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh -s -- 0.1.2
#   curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh -s -- --yes
#
# Flags:
#   -y, --yes   Non-interactive: error if another ion is found (instead of prompting for alias)
set -eu

REPO="Roger-luo/Ion"
INSTALL_DIR="${ION_INSTALL_DIR:-${HOME}/.local/bin}"
BIN_NAME="ion"
VERSION=""
YES=false

log()  { printf '  \033[1;32m>\033[0m %s\n' "$*"; }
warn() { printf '  \033[1;33m!\033[0m %s\n' "$*"; }
err()  { printf '  \033[1;31mx\033[0m %s\n' "$*" >&2; exit 1; }

# Parse flags
while [ $# -gt 0 ]; do
    case "$1" in
        -y|--yes) YES=true; shift ;;
        -*)       err "Unknown flag: $1" ;;
        *)        VERSION="$1"; shift ;;
    esac
done

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

        # Find all ion-v* releases that have assets, then pick the highest
        # semver. GitHub sorts by creation date, not version, so a re-published
        # older release can appear first.
        CANDIDATES=""
        for candidate in $(printf '%s' "$RELEASE_JSON" | grep -o '"tag_name": *"ion-v[^"]*"' | sed 's/.*"ion-v//' | sed 's/"//'); do
            # Check if this release has any .tar.gz assets
            if printf '%s' "$RELEASE_JSON" | grep -q "ion-${candidate}-.*\\.tar\\.gz"; then
                CANDIDATES="${CANDIDATES}${candidate}\n"
            fi
        done

        if [ -z "$CANDIDATES" ]; then
            err "Could not find latest ion release with prebuilt binaries"
        fi

        # Sort by semver (major.minor.patch) descending and take the highest
        VERSION=$(printf '%b' "$CANDIDATES" | sort -t. -k1,1n -k2,2n -k3,3n | tail -1)
        log "Latest version: $VERSION"
    fi
}

check_existing() {
    # 1. Check our own install directory
    if [ -x "${INSTALL_DIR}/ion" ]; then
        INSTALLED_VERSION=$(get_version "${INSTALL_DIR}/ion")
        if [ "$INSTALLED_VERSION" = "$VERSION" ]; then
            log "ion $VERSION is already installed at ${INSTALL_DIR}/ion"
            exit 0
        fi
        log "Upgrading ion $INSTALLED_VERSION -> $VERSION at ${INSTALL_DIR}/ion"
        return
    fi

    # 2. Check if ion exists elsewhere on PATH (installed by another tool)
    if ! command -v ion >/dev/null 2>&1; then
        return
    fi

    EXISTING=$(command -v ion)
    INSTALLED_VERSION=$(get_version "$EXISTING")

    # Determine what installed it
    MANAGER=""
    case "$EXISTING" in
        */.cargo/bin/*)  MANAGER="cargo" ;;
        */Cellar/*)      MANAGER="Homebrew" ;;
        */homebrew/bin/*) MANAGER="Homebrew" ;;
    esac

    if [ -n "$MANAGER" ]; then
        warn "Found ion $INSTALLED_VERSION at $EXISTING (installed via $MANAGER)"
    else
        warn "Found ion $INSTALLED_VERSION at $EXISTING"
    fi

    echo ""
    echo "  To avoid conflicts, you can either:"
    echo ""
    if [ -n "$MANAGER" ]; then
        case "$MANAGER" in
            cargo)    echo "    1. Uninstall the existing one:  cargo uninstall ion" ;;
            Homebrew) echo "    1. Uninstall the existing one:  brew uninstall ion" ;;
        esac
    else
        echo "    1. Remove the existing binary:   rm $EXISTING"
    fi
    echo "    2. Install with a different name (e.g. 'ion-skill')"
    echo ""

    # With --yes: just error, don't prompt
    if [ "$YES" = true ]; then
        err "Existing ion found at $EXISTING. Remove it first or omit --yes to choose an alias."
    fi

    # Non-interactive: error
    if [ ! -t 0 ] || [ ! -t 1 ]; then
        err "Existing ion found at $EXISTING. Remove it first or run interactively to choose an alias."
    fi

    printf '  \033[1;33m!\033[0m Install with a different name? Enter name (or press Enter to abort): '
    read -r REPLY </dev/tty

    if [ -z "$REPLY" ]; then
        log "Aborted."
        exit 0
    fi

    BIN_NAME="$REPLY"
    log "Will install as '$BIN_NAME'"
}

get_version() {
    _BIN="$1"
    _VER=""
    _OUT=$("$_BIN" --version 2>/dev/null) && \
        _VER=$(printf '%s' "$_OUT" | head -1 | sed 's/[^0-9.]*//')
    if [ -z "$_VER" ]; then
        _OUT=$("$_BIN" self info 2>/dev/null) && \
            _VER=$(printf '%s' "$_OUT" | head -1 | sed 's/[^0-9.]*//')
    fi
    printf '%s' "${_VER:-unknown}"
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
    mv "${TMPDIR}/ion" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
}

print_success() {
    log "Installed ion $VERSION to ${INSTALL_DIR}/${BIN_NAME}"

    if [ "$BIN_NAME" != "ion" ]; then
        echo ""
        log "Installed as '$BIN_NAME' (use '$BIN_NAME' instead of 'ion')"
    fi

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

    ION="${INSTALL_DIR}/${BIN_NAME}"

    # Verify binary works before offering completions
    if ! "$ION" --version >/dev/null 2>&1; then
        return
    fi

    # Skip completions for aliased installs — they'd register as "ion" not the alias
    if [ "$BIN_NAME" != "ion" ]; then
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
    ION="${INSTALL_DIR}/${BIN_NAME}"

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

main
