#!/usr/bin/env bash
set -euo pipefail

# Alejandria Installer v4
# Intelligent installer with auto-download, MCP client detection, and auto-configuration
# Usage: curl -fsSL https://raw.githubusercontent.com/USER/alejandria/main/scripts/install-mcp-v4.sh | bash

VERSION="${ALEJANDRIA_VERSION:-latest}"
INSTALL_DIR="${ALEJANDRIA_INSTALL_DIR:-$HOME/.local/bin}"
GITLAB_PROJECT="${GITLAB_PROJECT:-appsec/alejandria}"
GITLAB_HOST="${GITLAB_HOST:-gitlab.veritran.net}"
GITHUB_REPO="${GITHUB_REPO:-}"  # Fallback for public GitHub mirrors
FORCE_BUILD="${FORCE_BUILD:-false}"
KEEP_BUILD_CACHE="${KEEP_BUILD_CACHE:-false}"  # Set to true to preserve build artifacts

# Determine source (prefer GitLab for Veritran internal use)
if [ -n "$GITHUB_REPO" ]; then
    SOURCE_TYPE="github"
else
    SOURCE_TYPE="gitlab"
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}ℹ${NC} $*"; }
log_success() { echo -e "${GREEN}✓${NC} $*"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $*"; }
log_error() { echo -e "${RED}✗${NC} $*"; }

# Detect platform and architecture
detect_platform() {
    local os arch target

    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        linux)
            case "$arch" in
                x86_64) target="x86_64-unknown-linux-gnu" ;;
                *) log_error "Unsupported Linux architecture: $arch"; return 1 ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64) target="x86_64-apple-darwin" ;;
                arm64) target="aarch64-apple-darwin" ;;
                *) log_error "Unsupported macOS architecture: $arch"; return 1 ;;
            esac
            ;;
        *)
            log_error "Unsupported operating system: $os"
            return 1
            ;;
    esac

    echo "$target"
}

# Get latest release version from GitLab or GitHub
get_latest_version() {
    local api_url
    
    if [ "$SOURCE_TYPE" = "gitlab" ]; then
        # GitLab API: get latest tag
        local project_path_encoded=$(echo "$GITLAB_PROJECT" | sed 's/\//%2F/g')
        api_url="https://${GITLAB_HOST}/api/v4/projects/${project_path_encoded}/repository/tags"
        
        if command -v curl >/dev/null 2>&1; then
            # Get first tag from array and extract name field
            curl -fsSL "$api_url" | grep -m 1 '"name":' | sed -E 's/.*"name":\s*"([^"]+)".*/\1/'
        elif command -v wget >/dev/null 2>&1; then
            wget -qO- "$api_url" | grep -m 1 '"name":' | sed -E 's/.*"name":\s*"([^"]+)".*/\1/'
        else
            log_error "Neither curl nor wget found. Please install one of them."
            return 1
        fi
    else
        # GitHub API: get latest release
        api_url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
        
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL "$api_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
        elif command -v wget >/dev/null 2>&1; then
            wget -qO- "$api_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
        else
            log_error "Neither curl nor wget found. Please install one of them."
            return 1
        fi
    fi
}

# Download and verify binary
download_binary() {
    local version=$1
    local target=$2
    local archive_name="alejandria-${version}-${target}.tar.gz"
    local download_url
    local checksum_url
    local tmp_dir
    tmp_dir=$(mktemp -d)
    
    # Construct download URLs based on source type
    if [ "$SOURCE_TYPE" = "gitlab" ]; then
        local project_path_encoded=$(echo "$GITLAB_PROJECT" | sed 's/\//%2F/g')
        # First try: Release assets
        local release_url="https://${GITLAB_HOST}/api/v4/projects/${project_path_encoded}/releases/${version}"
        local release_data=$(curl -fsSL "$release_url" 2>/dev/null || echo "{}")
        
        # Extract download URLs from release assets
        download_url=$(echo "$release_data" | grep -o '"url":"[^"]*'"${archive_name}"'"' | sed 's/"url":"//' | sed 's/"$//' | head -1)
        checksum_url=$(echo "$release_data" | grep -o '"url":"[^"]*'"${archive_name}.sha256"'"' | sed 's/"url":"//' | sed 's/"$//' | head -1)
        
        # Fallback: Generic Package Registry (old method)
        if [ -z "$download_url" ]; then
            download_url="https://${GITLAB_HOST}/api/v4/projects/${project_path_encoded}/packages/generic/alejandria/${version}/${archive_name}"
            checksum_url="${download_url}.sha256"
        fi
    else
        download_url="https://github.com/${GITHUB_REPO}/releases/download/${version}/${archive_name}"
        checksum_url="${download_url}.sha256"
    fi

    log_info "Downloading Alejandria ${version} for ${target} from ${SOURCE_TYPE}..."
    
    # Download archive
    if command -v curl >/dev/null 2>&1; then
        curl -fL "$download_url" -o "${tmp_dir}/${archive_name}" || return 1
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$download_url" -O "${tmp_dir}/${archive_name}" || return 1
    else
        log_error "Neither curl nor wget found"
        return 1
    fi

    # Download checksum
    if command -v curl >/dev/null 2>&1; then
        curl -fL "$checksum_url" -o "${tmp_dir}/${archive_name}.sha256" 2>/dev/null || log_warn "Checksum not available"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$checksum_url" -O "${tmp_dir}/${archive_name}.sha256" 2>/dev/null || log_warn "Checksum not available"
    fi

    # Verify checksum if available
    if [ -f "${tmp_dir}/${archive_name}.sha256" ]; then
        log_info "Verifying checksum..."
        cd "$tmp_dir"
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum -c "${archive_name}.sha256" || { log_error "Checksum verification failed"; return 1; }
        elif command -v shasum >/dev/null 2>&1; then
            shasum -a 256 -c "${archive_name}.sha256" || { log_error "Checksum verification failed"; return 1; }
        else
            log_warn "sha256sum/shasum not found, skipping verification"
        fi
        cd - >/dev/null
        log_success "Checksum verified"
    fi

    # Extract binary
    log_info "Extracting binary..."
    tar xzf "${tmp_dir}/${archive_name}" -C "$tmp_dir"
    
    # Install binary
    mkdir -p "$INSTALL_DIR"
    
    # If binary is in use, install with temp name
    if ! cp "${tmp_dir}/alejandria-${version}-${target}/alejandria" "$INSTALL_DIR/alejandria" 2>/dev/null; then
        cp "${tmp_dir}/alejandria-${version}-${target}/alejandria" "$INSTALL_DIR/alejandria.new"
        chmod +x "$INSTALL_DIR/alejandria.new"
        log_warn "Installed as 'alejandria.new' (current binary is running)"
        log_warn "After terminating any running instances, run:"
        log_warn "  mv $INSTALL_DIR/alejandria.new $INSTALL_DIR/alejandria"
    else
        chmod +x "$INSTALL_DIR/alejandria"
    fi
    
    # Cleanup
    rm -rf "$tmp_dir"
    
    log_success "Binary installed to $INSTALL_DIR/alejandria"
    return 0
}

# Build from source as fallback
build_from_source() {
    log_warn "Building from source (this may take 10-20 minutes)..."
    
    # Check for cargo
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "cargo not found. Please install Rust: https://rustup.rs/"
        return 1
    fi

    # Clone or update repository
    local repo_dir="$HOME/.cache/alejandria-build"
    local repo_url
    
    if [ "$SOURCE_TYPE" = "gitlab" ]; then
        repo_url="https://${GITLAB_HOST}/${GITLAB_PROJECT}.git"
    else
        repo_url="https://github.com/${GITHUB_REPO}.git"
    fi
    
    if [ -d "$repo_dir" ]; then
        log_info "Updating existing repository..."
        cd "$repo_dir"
        git fetch --tags
        git checkout main
        git pull
    else
        log_info "Cloning repository from ${SOURCE_TYPE}..."
        if ! git clone "$repo_url" "$repo_dir" 2>/dev/null; then
            log_warn "Failed to clone from ${SOURCE_TYPE}. Checking for local repository..."
            
            # Try to use current directory if it's the Alejandria repo
            if [ -f "Cargo.toml" ] && grep -q "alejandria" Cargo.toml 2>/dev/null; then
                log_info "Using current directory as source"
                repo_dir="$(pwd)"
            else
                log_error "Could not clone repository and no local source found"
                return 1
            fi
        else
            cd "$repo_dir"
        fi
    fi

    # Build
    log_info "Building release binary..."
    cargo build --release --package alejandria-cli --bin alejandria

    # Install
    mkdir -p "$INSTALL_DIR"
    
    # If binary is in use, install with temp name and instruct user to restart
    if ! cp target/release/alejandria "$INSTALL_DIR/alejandria" 2>/dev/null; then
        cp target/release/alejandria "$INSTALL_DIR/alejandria.new"
        chmod +x "$INSTALL_DIR/alejandria.new"
        log_warn "Installed as 'alejandria.new' (current binary is running)"
        log_warn "After terminating any running instances, run:"
        log_warn "  mv $INSTALL_DIR/alejandria.new $INSTALL_DIR/alejandria"
    else
        chmod +x "$INSTALL_DIR/alejandria"
    fi

    # Clean up build artifacts to save disk space
    if [ "$KEEP_BUILD_CACHE" = "false" ]; then
        log_info "Cleaning build cache to free disk space..."
        if [ "$repo_dir" != "$(pwd)" ]; then
            # Only remove if it's in cache directory (not current dir)
            local cache_size
            cache_size=$(du -sh "$repo_dir" 2>/dev/null | cut -f1)
            rm -rf "$repo_dir"
            log_success "Cleaned build cache (freed ~$cache_size)"
        else
            # If using current directory, just clean target/
            cargo clean 2>/dev/null || true
            log_success "Cleaned build artifacts"
        fi
    else
        log_info "Build cache preserved at: $repo_dir"
    fi

    log_success "Binary built and installed to $INSTALL_DIR/alejandria"
    return 0
}

# Detect MCP clients
detect_mcp_clients() {
    local clients=()

    # OpenCode
    if [ -f "$HOME/.config/opencode/opencode.json" ]; then
        clients+=("opencode:$HOME/.config/opencode/opencode.json")
    fi

    # Claude Desktop
    if [ -f "$HOME/.config/Claude/claude_desktop_config.json" ]; then
        clients+=("claude:$HOME/.config/Claude/claude_desktop_config.json")
    elif [ -f "$HOME/Library/Application Support/Claude/claude_desktop_config.json" ]; then
        clients+=("claude:$HOME/Library/Application Support/Claude/claude_desktop_config.json")
    fi

    # VSCode (check settings.json for mcp configuration)
    if [ -f "$HOME/.config/Code/User/settings.json" ]; then
        if grep -q "mcp" "$HOME/.config/Code/User/settings.json" 2>/dev/null; then
            clients+=("vscode:$HOME/.config/Code/User/settings.json")
        fi
    fi

    printf '%s\n' "${clients[@]}"
}

# Backup configuration file
backup_config() {
    local config_file=$1
    local timestamp
    timestamp=$(date +%Y%m%d-%H%M%S)
    local backup_file="${config_file}.backup-${timestamp}"
    
    cp "$config_file" "$backup_file"
    echo "$backup_file"
}

# Merge Alejandria config into MCP config
merge_config() {
    local config_file=$1
    local client_name=$2
    local backup_file
    
    log_info "Configuring $(basename "$(dirname "$config_file")")..."
    
    # Backup existing config
    backup_file=$(backup_config "$config_file")
    log_info "Backup created: $backup_file"

    # Read existing config
    local existing_config
    existing_config=$(cat "$config_file")

    # Create Alejandria server config based on client type
    local alejandria_config
    if [ "$client_name" = "opencode" ]; then
        # OpenCode format: command array, enabled, type, environment
        alejandria_config=$(cat <<EOF
{
  "command": ["$INSTALL_DIR/alejandria", "serve"],
  "enabled": true,
  "type": "local",
  "environment": {
    "ALEJANDRIA_CONFIG": "$HOME/.config/alejandria/config.toml"
  }
}
EOF
)
    else
        # Claude Desktop format: command string, args array, env object
        alejandria_config=$(cat <<EOF
{
  "command": "$INSTALL_DIR/alejandria",
  "args": ["serve"],
  "env": {
    "ALEJANDRIA_CONFIG": "$HOME/.config/alejandria/config.toml"
  }
}
EOF
)
    fi

    # Determine the correct top-level key based on client
    local mcp_key
    if [ "$client_name" = "opencode" ]; then
        mcp_key="mcp"
    else
        mcp_key="mcpServers"
    fi

    # Merge configs using jq (or Python fallback)
    if command -v jq >/dev/null 2>&1; then
        echo "$existing_config" | jq \
            --argjson server "$alejandria_config" \
            --arg key "$mcp_key" \
            '.[$key].alejandria = $server' \
            > "$config_file.tmp"
    elif command -v python3 >/dev/null 2>&1; then
        python3 <<EOF
import json
import sys

existing = json.loads('''$existing_config''')
server = json.loads('''$alejandria_config''')
mcp_key = '''$mcp_key'''

if mcp_key not in existing:
    existing[mcp_key] = {}
existing[mcp_key]['alejandria'] = server

with open('$config_file.tmp', 'w') as f:
    json.dump(existing, f, indent=2)
EOF
    else
        log_error "Neither jq nor python3 found. Cannot merge configuration."
        return 1
    fi

    # Validate JSON syntax
    if command -v jq >/dev/null 2>&1; then
        if ! jq empty "$config_file.tmp" 2>/dev/null; then
            log_error "Generated invalid JSON. Rolling back..."
            rm "$config_file.tmp"
            cp "$backup_file" "$config_file"
            return 1
        fi
    fi

    # Apply new config
    mv "$config_file.tmp" "$config_file"
    log_success "Configuration updated"
    return 0
}

# Create initial Alejandria config if not exists
create_alejandria_config() {
    local config_dir="$HOME/.config/alejandria"
    local config_file="$config_dir/config.toml"

    if [ -f "$config_file" ]; then
        log_info "Alejandria config already exists: $config_file"
        return 0
    fi

    mkdir -p "$config_dir"
    
    cat > "$config_file" <<'EOF'
# Alejandria Configuration

[database]
path = "~/.local/share/alejandria/alejandria.db"

[embeddings]
enabled = true
# model = "BAAI/bge-small-en-v1.5"  # Default model

[decay]
# Decay profiles for memory importance
critical = 0.99
high = 0.95
medium = 0.90
low = 0.85

[server]
# MCP server settings (stdio transport)
# No additional configuration needed for stdio
EOF

    log_success "Created Alejandria config: $config_file"
}

# Main installation flow
main() {
    log_info "Alejandria Installer v4"
    echo

    # Detect platform
    local target
    target=$(detect_platform) || exit 1
    log_info "Detected platform: $target"

    # Get version
    if [ "$VERSION" = "latest" ]; then
        VERSION=$(get_latest_version) || {
            log_warn "Could not fetch latest version, will build from source"
            FORCE_BUILD=true
        }
        log_info "Latest version: $VERSION"
    fi

    # Check if we already have a recent binary installed
    if [ "$FORCE_BUILD" = "false" ] && [ -f "$INSTALL_DIR/alejandria" ]; then
        local existing_version
        existing_version=$("$INSTALL_DIR/alejandria" --version 2>/dev/null | awk '{print $2}' || echo "unknown")
        
        if [ "$existing_version" != "unknown" ]; then
            log_info "Found existing installation: v$existing_version"
            
            # If we don't have a specific version, ask if user wants to keep existing
            if [ "$VERSION" = "latest" ] || [ -z "$VERSION" ]; then
                log_info "Skipping reinstall (use FORCE_BUILD=true to reinstall)"
                VERSION="$existing_version"
            fi
        fi
    fi

    # Install binary (download or build)
    if [ "$FORCE_BUILD" = "true" ]; then
        build_from_source || exit 1
    elif [ -f "$INSTALL_DIR/alejandria" ] && [ "$VERSION" = "$existing_version" ]; then
        log_success "Using existing binary v$VERSION"
    else
        download_binary "$VERSION" "$target" || {
            log_warn "Download failed, falling back to build from source"
            build_from_source || exit 1
        }
    fi

    # Verify installation
    if ! "$INSTALL_DIR/alejandria" --version >/dev/null 2>&1; then
        log_error "Installation verification failed"
        exit 1
    fi
    
    local installed_version
    installed_version=$("$INSTALL_DIR/alejandria" --version | awk '{print $2}')
    log_success "Alejandria $installed_version installed successfully"

    # Add to PATH if not already there
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        log_warn "$INSTALL_DIR is not in your PATH"
        echo "Add this line to your ~/.bashrc or ~/.zshrc:"
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
    fi

    echo

    # Create Alejandria config
    create_alejandria_config

    # Detect and configure MCP clients
    log_info "Detecting MCP clients..."
    mapfile -t clients < <(detect_mcp_clients)
    
    if [ ${#clients[@]} -eq 0 ]; then
        log_warn "No MCP clients detected"
        log_info "Supported clients: OpenCode, Claude Desktop, VSCode"
        echo
        log_info "Manual configuration:"
        echo "  Add to your MCP client config:"
        echo '  "alejandria": {'
        echo "    \"command\": \"$INSTALL_DIR/alejandria\","
        echo '    "args": ["serve"],'
        echo '    "env": {'
        echo "      \"ALEJANDRIA_CONFIG\": \"$HOME/.config/alejandria/config.toml\""
        echo '    }'
        echo '  }'
    else
        log_success "Found ${#clients[@]} MCP client(s)"
        echo
        
        for client_info in "${clients[@]}"; do
            IFS=':' read -r client_name config_file <<< "$client_info"
            merge_config "$config_file" "$client_name" || {
                log_error "Failed to configure $client_name"
                continue
            }
        done

        echo
        log_success "All MCP clients configured!"
        log_warn "Please restart your MCP clients to apply changes"
    fi

    echo
    log_info "Quick verification:"
    echo "  alejandria --version"
    echo "  alejandria store --content \"Test memory\" --summary \"Test\""
    echo "  alejandria recall \"test\""
    
    echo
    log_success "Installation complete!"
}

main "$@"
