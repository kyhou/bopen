#!/usr/bin/env bash
# bopen installer script
# Usage: curl -sSL https://raw.githubusercontent.com/kyhou/bopen/main/install.sh | sh
# Or:    curl -sSL https://raw.githubusercontent.com/kyhou/bopen/main/install.sh | sh -s -- --dir /usr/local/bin

set -euo pipefail

# =============================================================================
# Constants
# =============================================================================
readonly REPO="kyhou/bopen"
readonly REPO_URL="https://github.com/${REPO}"
readonly API_URL="https://api.github.com/repos/${REPO}/releases"
readonly RAW_BASE="https://raw.githubusercontent.com/${REPO}"
readonly INSTALL_DIR="${HOME}/.local/bin"
readonly DESKTOP_DIR="${HOME}/.local/share/applications"
readonly ASSET_NAME="bopen-x86_64-unknown-linux-gnu.tar.gz"
readonly ARCHIVE_NAME="bopen"
readonly SCRIPT_VERSION="1.0.0"

# Colors
readonly COLOR_RESET='\033[0m'
readonly COLOR_RED='\033[0;31m'
readonly COLOR_GREEN='\033[0;32m'
readonly COLOR_YELLOW='\033[0;33m'
readonly COLOR_CYAN='\033[0;36m'

# =============================================================================
# Global Variables
# =============================================================================
INSTALL_PATH=""
SKIP_CONFIRMATIONS=false
SKIP_DESKTOP=false
SPECIFIC_VERSION=""
QUIET_MODE=false

# =============================================================================
# Helper Functions
# =============================================================================

info() {
    echo -e "${COLOR_CYAN}[INFO]${COLOR_RESET} $*"
}

success() {
    echo -e "${COLOR_GREEN}[OK]${COLOR_RESET} $*"
}

warn() {
    echo -e "${COLOR_YELLOW}[WARN]${COLOR_RESET} $*"
}

error() {
    echo -e "${COLOR_RED}[ERROR]${COLOR_RESET} $*" >&2
}

ask() {
    local prompt="$1"
    local default="${2:-}"

    if [[ "$SKIP_CONFIRMATIONS" == true ]]; then
        echo "$default"
        return 0
    fi

    while true; do
        echo -n "$prompt"
        if [[ -n "$default" ]]; then
            echo -n " [$default]"
        fi
        echo -n ": "
        read -r answer

        if [[ -z "$answer" ]]; then
            answer="$default"
        fi

        case "$answer" in
            y|Y|yes|Yes|YES)
                return 0
                ;;
            n|N|no|No|NO)
                return 1
                ;;
            *)
                echo "Please answer 'y' or 'n'."
                ;;
        esac
    done
}

show_help() {
    cat << EOF
bopen installer v${SCRIPT_VERSION}

USAGE:
    curl -sSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh [OPTIONS]

OPTIONS:
    -h, --help              Show this help message
    -d, --dir <path>        Install directory (default: ${INSTALL_DIR})
    -y, --yes                Skip all confirmations
    -q, --quiet              Quiet mode (only show errors)
    --skip-desktop           Skip desktop file installation
    --version <ver>          Install specific version (default: latest)

EXAMPLES:
    # Install with defaults
    curl -sSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh

    # Install to /usr/local/bin (requires sudo)
    curl -sSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- -d /usr/local/bin

    # Install specific version
    curl -sSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --version v0.1.0

    # Skip desktop file, silent install
    curl -sSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --skip-desktop -y

EOF
}

# =============================================================================
# Download Functions
# =============================================================================

download_file() {
    local url="$1"
    local dest="$2"
    local description="$3"

    info "Downloading ${description}..."

    if ! curl -fSL --progress-bar "$url" -o "$dest" 2>/dev/null; then
        if [[ "$QUIET_MODE" == true ]]; then
            error "Failed to download $description from $url"
        else
            error "Failed to download ${description}."
            error "URL: $url"
            error "Please check your internet connection and try again."
        fi
        return 1
    fi

    if [[ "$QUIET_MODE" != true ]]; then
        success "Downloaded ${description}"
    fi
}

get_latest_version() {
    local version

    if [[ -n "$SPECIFIC_VERSION" ]]; then
        echo "$SPECIFIC_VERSION"
        return 0
    fi

    info "Fetching latest version..."

    version=$(curl -sSL "${API_URL}/latest" | grep -o '"tag_name":.*' | sed 's/.*": "\([^"]*\)".*/\1/' | head -1)

    if [[ -z "$version" ]]; then
        error "Failed to fetch latest version from GitHub API"
        error "The API might be rate limiting. You can try:"
        error "  1. Wait a few minutes and try again"
        error "  2. Use --version to specify a version"
        error "  3. Install from source: cargo install bopen"
        return 1
    fi

    echo "$version"
}

get_download_url() {
    local version="$1"

    if [[ -n "$SPECIFIC_VERSION" ]]; then
        echo "${REPO_URL}/releases/download/${version}/${ASSET_NAME}"
    else
        echo "${REPO_URL}/releases/latest/download/${ASSET_NAME}"
    fi
}

get_desktop_file_url() {
    local branch="${1:-main}"
    echo "${RAW_BASE}/${branch}/data/bopen.desktop"
}

# =============================================================================
# Installation Functions
# =============================================================================

check_prerequisites() {
    local missing=()

    if ! command -v curl &>/dev/null; then
        missing+=("curl")
    fi

    if ! command -v tar &>/dev/null; then
        missing+=("tar")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        error "Missing required dependencies: ${missing[*]}"
        error "Please install them and try again."
        return 1
    fi

    return 0
}

check_install_dir() {
    local dir="$1"

    # Check if directory exists
    if [[ ! -d "$dir" ]]; then
        if ask "Directory '$dir' does not exist. Create it?" "y"; then
            mkdir -p "$dir"
            success "Created directory: $dir"
        else
            error "Cannot install without a valid directory."
            return 1
        fi
    fi

    # Check if directory is writable
    if [[ ! -w "$dir" ]]; then
        error "Directory '$dir' is not writable."
        error "Try running with sudo, or choose a different directory."
        error "Example: curl ... | sh -s -- -d ~/.local/bin"
        return 1
    fi

    return 0
}

download_and_install() {
    local version="$1"
    local download_url
    local desktop_url
    local temp_dir
    local archive_path

    download_url=$(get_download_url "$version")
    desktop_url=$(get_desktop_file_url)

    # Create temp directory
    temp_dir=$(mktemp -d)
    archive_path="${temp_dir}/${ASSET_NAME}"

    # Cleanup on exit
    trap "rm -rf $temp_dir" EXIT

    # Download archive
    if ! download_file "$download_url" "$archive_path" "bopen binary"; then
        error "Failed to download bopen. This might mean:"
        error "  - No release is available yet (build from source first)"
        error "  - The release doesn't have a Linux x86_64 binary"
        error ""
        error "To create a release:"
        error "  1. Go to ${REPO_URL}/releases/new"
        error "  2. Create a new tag (e.g., v0.1.0)"
        error "  3. Upload the binary: ${ASSET_NAME}"
        error "  4. Publish the release"
        error ""
        error "To build from source instead:"
        error "  cargo install bopen"
        return 1
    fi

    # Extract
    info "Extracting archive..."
    tar -xzf "$archive_path" -C "$temp_dir"

    # Check if extraction was successful
    if [[ ! -f "${temp_dir}/${ARCHIVE_NAME}/bopen" ]]; then
        error "Extracted archive doesn't contain expected files."
        error "Archive contents:"
        ls -la "$temp_dir/"
        return 1
    fi

    # Install binary
    info "Installing to ${INSTALL_PATH}..."
    cp "${temp_dir}/${ARCHIVE_NAME}/bopen" "${INSTALL_PATH}/bopen"
    chmod +x "${INSTALL_PATH}/bopen"
    success "Installed bopen to ${INSTALL_PATH}/bopen"

    # Install desktop file if requested
    if [[ "$SKIP_DESKTOP" == false ]]; then
        install_desktop_file "$version"
    fi

    # Cleanup temp files
    rm -rf "$temp_dir"
    trap - EXIT

    return 0
}

install_desktop_file() {
    local version="$1"
    local desktop_url
    local desktop_path
    local desktop_content

    desktop_url=$(get_desktop_file_url)

    # Create desktop directory if needed
    mkdir -p "$DESKTOP_DIR"

    # Download desktop file
    info "Downloading desktop file..."

    if desktop_content=$(curl -sSL "$desktop_url" 2>/dev/null); then
        # Update Exec path to use installed location
        desktop_content="${desktop_content//Exec=bopen/Exec=${INSTALL_PATH}/bopen}"
        desktop_content="${desktop_content//Exec=\/usr\/local\/bin\/bopen/Exec=${INSTALL_PATH}/bopen}"

        desktop_path="${DESKTOP_DIR}/bopen.desktop"
        echo "$desktop_content" > "$desktop_path"

        chmod +x "$desktop_path" 2>/dev/null || true
        chmod 644 "$desktop_path"

        success "Installed desktop file to ${desktop_path}"
    else
        warn "Failed to download desktop file, but installation succeeded."
        return 1
    fi
}

register_as_default() {
    local desktop_path="${DESKTOP_DIR}/bopen.desktop"

    if [[ ! -f "$desktop_path" ]]; then
        warn "Desktop file not installed. Cannot register as default browser."
        return 1
    fi

    echo ""
    echo "=========================================="
    echo "  Set bopen as default browser?"
    echo "=========================================="
    echo ""
    echo "This allows opening links from external apps (Telegram, etc.) with bopen."
    echo ""
    echo "To revert to another browser later, run:"
    echo "  xdg-mime default firefox.desktop x-scheme-handler/http"
    echo "  xdg-mime default firefox.desktop x-scheme-handler/https"
    echo ""

    if ask "Set bopen as default browser?" "n"; then
        info "Registering bopen as default browser..."

        xdg-mime default bopen.desktop x-scheme-handler/http
        xdg-mime default bopen.desktop x-scheme-handler/https
        xdg-mime default bopen.desktop x-scheme-handler/https
        xdg-mime default bopen.desktop text/html

        success "bopen is now the default browser for HTTP, HTTPS, and HTML files."
        info "You can change this anytime with: xdg-mime default <browser>.desktop ..."
    else
        info "Skipped. You can set bopen as default manually:"
        info "  xdg-mime default bopen.desktop x-scheme-handler/http"
        info "  xdg-mime default bopen.desktop x-scheme-handler/https"
    fi
}

# =============================================================================
# Parse Arguments
# =============================================================================

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                show_help
                exit 0
                ;;
            -d|--dir)
                INSTALL_PATH="${2}"
                shift 2
                ;;
            -y|--yes)
                SKIP_CONFIRMATIONS=true
                shift
                ;;
            -q|--quiet)
                QUIET_MODE=true
                shift
                ;;
            --skip-desktop)
                SKIP_DESKTOP=true
                shift
                ;;
            --version)
                SPECIFIC_VERSION="${2}"
                shift 2
                ;;
            -*)
                error "Unknown option: $1"
                show_help
                exit 1
                ;;
            *)
                error "Unexpected argument: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Set default install path if not specified
    if [[ -z "$INSTALL_PATH" ]]; then
        INSTALL_PATH="$INSTALL_DIR"
    fi
}

# =============================================================================
# Main
# =============================================================================

main() {
    local version

    # Parse command line arguments
    parse_arguments "$@"

    # Check prerequisites
    check_prerequisites || exit 1

    # Check install directory
    check_install_dir "$INSTALL_PATH" || exit 1

    # Get version
    version=$(get_latest_version) || exit 1

    # Show installation info
    if [[ "$QUIET_MODE" != true ]]; then
        echo ""
        echo "=========================================="
        echo "  bopen Installer"
        echo "=========================================="
        echo ""
        echo "  Version:  $version"
        echo "  Platform: Linux x86_64"
        echo "  Location: $INSTALL_PATH"
        echo ""
    fi

    # Confirm installation
    if [[ "$SKIP_CONFIRMATIONS" != true ]]; then
        echo ""
        if ! ask "Proceed with installation?" "y"; then
            info "Installation cancelled."
            exit 0
        fi
    fi

    # Download and install
    download_and_install "$version" || exit 1

    # Optionally register as default browser
    if [[ "$SKIP_DESKTOP" == false ]]; then
        register_as_default
    fi

    # Success message
    if [[ "$QUIET_MODE" != true ]]; then
        echo ""
        echo "=========================================="
        echo "  Installation Complete!"
        echo "=========================================="
        echo ""
        echo "  bopen has been installed to: ${INSTALL_PATH}/bopen"
        echo ""
        echo "  To use bopen, run:"
        echo "    ${INSTALL_PATH}/bopen"
        echo ""

        # Check if PATH includes install directory
        if [[ ":$PATH:" == *":${INSTALL_PATH}:"* ]]; then
            success "bopen is in your PATH. Just run 'bopen'."
        else
            warn "bopen is not in your PATH."
            info "Add this to your ~/.bashrc or ~/.zshrc:"
            info "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
        fi

        echo ""
    fi
}

main "$@"
