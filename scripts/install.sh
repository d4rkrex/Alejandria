#!/usr/bin/env bash
set -euo pipefail

# Alejandria Installer v5
# Intelligent installer with auto-download, MCP client detection, auto-configuration, AND skill installation
# Usage: curl -fsSL https://raw.githubusercontent.com/d4rkrex/Alejandria/main/scripts/install.sh | bash

VERSION="${ALEJANDRIA_VERSION:-latest}"
INSTALL_DIR="${ALEJANDRIA_INSTALL_DIR:-$HOME/.local/bin}"
SKILLS_DIR="${SKILLS_DIR:-$HOME/.config/opencode/skills}"
AGENT_INSTRUCTIONS="${AGENT_INSTRUCTIONS:-$HOME/.config/opencode/AGENT_INSTRUCTIONS.md}"
GITLAB_PROJECT="${GITLAB_PROJECT:-appsec/alejandria}"
GITLAB_HOST="${GITLAB_HOST:-}"
GITLAB_TOKEN="${GITLAB_TOKEN:-}"  # Optional: for private GitLab repos
GITHUB_REPO="${GITHUB_REPO:-d4rkrex/Alejandria}"  # Public GitHub repository
FORCE_BUILD="${FORCE_BUILD:-false}"
KEEP_BUILD_CACHE="${KEEP_BUILD_CACHE:-false}"  # Set to true to preserve build artifacts
INSTALL_SKILLS="${INSTALL_SKILLS:-true}"  # Set to false to skip skill installation
INSTALL_DEV_SKILLS="${INSTALL_DEV_SKILLS:-false}"  # Set to true to also install development skills

# Determine source (prefer GitHub public repo, fallback to GitLab for internal use)
if [ -n "$GITLAB_TOKEN" ] && [ -z "${FORCE_GITHUB:-}" ]; then
    SOURCE_TYPE="gitlab"
else
    SOURCE_TYPE="github"
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

# Get version from Cargo.toml workspace (most reliable source)
get_cargo_version() {
    local repo_dir="${1:-.}"
    local cargo_toml="$repo_dir/Cargo.toml"
    
    if [ -f "$cargo_toml" ]; then
        # Extract version from [workspace.package] section (portable sed, no \s)
        grep -A 10 '^\[workspace\.package\]' "$cargo_toml" | grep '^version' | head -1 | sed -E 's/version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/'
    fi
}

# Get latest release version from GitLab or GitHub
get_latest_version() {
    local api_url

    # First, try to get version from local Cargo.toml if we're in the repo
    if [ -f "Cargo.toml" ]; then
        local cargo_version
        cargo_version=$(get_cargo_version ".")
        if [ -n "$cargo_version" ]; then
            # Ensure v-prefix for consistency with release tags
            case "$cargo_version" in
                v*) echo "$cargo_version" ;;
                *)  echo "v${cargo_version}" ;;
            esac
            return 0
        fi
    fi
    
    if [ "$SOURCE_TYPE" = "gitlab" ]; then
        # GitLab API: get latest tag
        local project_path_encoded=$(echo "$GITLAB_PROJECT" | sed 's/\//%2F/g')
        api_url="https://${GITLAB_HOST}/api/v4/projects/${project_path_encoded}/repository/tags"
        
        local curl_opts=(-fsSL)
        if [ -n "$GITLAB_TOKEN" ]; then
            curl_opts+=(--header "PRIVATE-TOKEN: $GITLAB_TOKEN")
        fi
        
        if command -v curl >/dev/null 2>&1; then
            # Get first tag from array and extract name field
            curl "${curl_opts[@]}" "$api_url" | grep -m 1 '"name":' | sed -E 's/.*"name":\s*"([^"]+)".*/\1/'
        elif command -v wget >/dev/null 2>&1; then
            local wget_opts=(--quiet -O-)
            if [ -n "$GITLAB_TOKEN" ]; then
                wget_opts+=(--header="PRIVATE-TOKEN: $GITLAB_TOKEN")
            fi
            wget "${wget_opts[@]}" "$api_url" | grep -m 1 '"name":' | sed -E 's/.*"name":\s*"([^"]+)".*/\1/'
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

# Check for pre-compiled binary in repo
use_prebuilt_binary() {
    local target=$1
    local repo_dir="${2:-.}"  # Current dir or specified repo path
    
    # Map target to binary filename
    local binary_name
    case "$target" in
        x86_64-unknown-linux-gnu)
            binary_name="alejandria-linux-x86_64"
            ;;
        x86_64-apple-darwin)
            binary_name="alejandria-macos-x86_64"
            ;;
        aarch64-apple-darwin)
            binary_name="alejandria-macos-aarch64"
            ;;
        *)
            log_warn "No pre-built binary for target: $target"
            return 1
            ;;
    esac
    
    local binary_path="$repo_dir/bin/$binary_name"
    local checksum_path="${binary_path}.sha256"
    
    # Check if binary exists
    if [ ! -f "$binary_path" ]; then
        return 1
    fi
    
    log_info "Found pre-built binary: $binary_path"
    
    # Verify checksum if available
    if [ -f "$checksum_path" ]; then
        log_info "Verifying checksum..."
        cd "$(dirname "$binary_path")"
        if command -v sha256sum >/dev/null 2>&1; then
            if ! sha256sum -c "$(basename "$checksum_path")" >/dev/null 2>&1; then
                log_error "Checksum verification failed"
                cd - >/dev/null
                return 1
            fi
        elif command -v shasum >/dev/null 2>&1; then
            if ! shasum -a 256 -c "$(basename "$checksum_path")" >/dev/null 2>&1; then
                log_error "Checksum verification failed"
                cd - >/dev/null
                return 1
            fi
        else
            log_warn "No checksum tool found, skipping verification"
        fi
        cd - >/dev/null
        log_success "Checksum verified"
    fi
    
    # Install binary
    mkdir -p "$INSTALL_DIR"
    
    if ! cp "$binary_path" "$INSTALL_DIR/alejandria" 2>/dev/null; then
        cp "$binary_path" "$INSTALL_DIR/alejandria.new"
        chmod +x "$INSTALL_DIR/alejandria.new"
        log_warn "Installed as 'alejandria.new' (current binary is running)"
        log_warn "After terminating any running instances, run:"
        log_warn "  mv $INSTALL_DIR/alejandria.new $INSTALL_DIR/alejandria"
    else
        chmod +x "$INSTALL_DIR/alejandria"
    fi
    
    log_success "Pre-built binary installed to $INSTALL_DIR/alejandria"
    return 0
}

# Download pre-built binary directly from GitLab API (workaround for clone cache issues)
download_from_gitlab_api() {
    local target=$1
    local token="${GITLAB_TOKEN:-}"
    
    # Map target to binary filename
    local binary_name
    case "$target" in
        x86_64-unknown-linux-gnu)
            binary_name="alejandria-linux-x86_64"
            ;;
        x86_64-apple-darwin)
            binary_name="alejandria-macos-x86_64"
            ;;
        aarch64-apple-darwin)
            binary_name="alejandria-macos-aarch64"
            ;;
        *)
            return 1
            ;;
    esac
    
    local project_path_encoded=$(echo "$GITLAB_PROJECT" | sed 's/\//%2F/g')
    local file_path_encoded="bin%2F${binary_name}"
    local checksum_file_path_encoded="bin%2F${binary_name}.sha256"
    local binary_url="https://${GITLAB_HOST}/api/v4/projects/${project_path_encoded}/repository/files/${file_path_encoded}/raw?ref=main"
    local checksum_url="https://${GITLAB_HOST}/api/v4/projects/${project_path_encoded}/repository/files/${checksum_file_path_encoded}/raw?ref=main"
    
    log_info "Attempting to download $binary_name from GitLab API..."
    
    local tmp_dir
    tmp_dir=$(mktemp -d)
    local binary_path="$tmp_dir/$binary_name"
    local checksum_path="$tmp_dir/${binary_name}.sha256"
    
    # Download binary
    local curl_opts=(-fsSL)
    if [ -n "$token" ]; then
        curl_opts+=(--header "PRIVATE-TOKEN: $token")
    fi
    
    if ! curl "${curl_opts[@]}" "$binary_url" -o "$binary_path" 2>/dev/null; then
        log_warn "Failed to download binary from GitLab API (may be private repo)"
        rm -rf "$tmp_dir"
        return 1
    fi
    
    # Download checksum (optional - may be stale due to GitLab cache issues)
    local checksum_verified=false
    if curl "${curl_opts[@]}" "$checksum_url" -o "$checksum_path" 2>/dev/null; then
        # Try to verify checksum (but don't fail if it doesn't match - cache issues)
        log_info "Attempting checksum verification..."
        cd "$tmp_dir"
        if command -v sha256sum >/dev/null 2>&1; then
            if sha256sum -c "$(basename "$checksum_path")" >/dev/null 2>&1; then
                checksum_verified=true
            fi
        elif command -v shasum >/dev/null 2>&1; then
            if shasum -a 256 -c "$(basename "$checksum_path")" >/dev/null 2>&1; then
                checksum_verified=true
            fi
        fi
        cd - >/dev/null
        
        if [ "$checksum_verified" = true ]; then
            log_success "Checksum verified"
        else
            log_warn "Checksum mismatch (possibly due to GitLab clone cache - continuing anyway)"
        fi
    else
        log_warn "Checksum file not found, skipping verification"
    fi
    
    # Install binary
    mkdir -p "$INSTALL_DIR"
    
    if ! cp "$binary_path" "$INSTALL_DIR/alejandria" 2>/dev/null; then
        cp "$binary_path" "$INSTALL_DIR/alejandria.new"
        chmod +x "$INSTALL_DIR/alejandria.new"
        log_warn "Installed as 'alejandria.new' (current binary is running)"
        log_warn "After terminating any running instances, run:"
        log_warn "  mv $INSTALL_DIR/alejandria.new $INSTALL_DIR/alejandria"
    else
        chmod +x "$INSTALL_DIR/alejandria"
    fi
    
    rm -rf "$tmp_dir"
    log_success "Binary installed from GitLab API"
    return 0
}

# Download and verify binary
download_binary() {
    local version=$1
    local target=$2
    # Normalize version: ensure v-prefix for GitHub release tags
    case "$version" in
        v*) : ;;
        *)  version="v${version}" ;;
    esac
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
    
    # If binary is in use, try to replace it anyway (overwrite), fallback to .new
    if cp "${tmp_dir}/alejandria-${version}-${target}/alejandria" "$INSTALL_DIR/alejandria" 2>/dev/null; then
        chmod +x "$INSTALL_DIR/alejandria"
    else
        cp "${tmp_dir}/alejandria-${version}-${target}/alejandria" "$INSTALL_DIR/alejandria.new"
        chmod +x "$INSTALL_DIR/alejandria.new"
        # Try atomic replace via rename
        mv "$INSTALL_DIR/alejandria.new" "$INSTALL_DIR/alejandria" 2>/dev/null && \
            log_success "Binary replaced successfully" || {
            log_warn "Installed as 'alejandria.new' (current binary is running)"
            log_warn "After terminating any running instances, run:"
            log_warn "  mv $INSTALL_DIR/alejandria.new $INSTALL_DIR/alejandria"
        }
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

# Install skills globally for all agents
install_skills() {
    log_info "Installing Alejandría skills..."
    
    # Create skills directory
    mkdir -p "$SKILLS_DIR"
    
    # Try to find skills in multiple locations
    local skills_source=""
    
    # Location 1: Running from repo (development)
    if [ -d "./skills" ]; then
        skills_source="./skills"
    # Location 2: Extracted from downloaded tarball (build cache)
    elif [ -d "/tmp/alejandria-build/skills" ]; then
        skills_source="/tmp/alejandria-build/skills"
    # Location 3: Already installed
    elif [ -d "$HOME/.local/share/alejandria/skills" ]; then
        skills_source="$HOME/.local/share/alejandria/skills"
    fi
    
    if [ -z "$skills_source" ]; then
        log_warn "Skills directory not found, attempting to download..."
        
        # Try to download skills from source
        local temp_dir="/tmp/alejandria-skills-$$"
        mkdir -p "$temp_dir"
        
        if [ "$SOURCE_TYPE" = "gitlab" ]; then
            local project_path_encoded=$(echo "$GITLAB_PROJECT" | sed 's/\//%2F/g')
            local archive_url="https://${GITLAB_HOST}/api/v4/projects/${project_path_encoded}/repository/archive.tar.gz?sha=main"
            
            local curl_opts=(-fsSL)
            if [ -n "$GITLAB_TOKEN" ]; then
                curl_opts+=(--header "PRIVATE-TOKEN: $GITLAB_TOKEN")
            fi
            
            if curl "${curl_opts[@]}" "$archive_url" | tar -xz -C "$temp_dir" --strip-components=1 2>/dev/null; then
                skills_source="$temp_dir/skills"
            fi
        fi
        
        if [ -z "$skills_source" ] || [ ! -d "$skills_source" ]; then
            log_warn "Could not download skills. Skipping skill installation."
            log_info "You can manually install skills later from the repository"
            return 0
        fi
    fi
    
    log_info "Found skills in: $skills_source"
    
    # Install user skills (skills/* but not skills/dev/*)
    local user_installed_count=0
    local user_installed_skills=()
    local skill_name target_skill abs_skill_dir
    
    log_info "Installing user skills..."
    
    # Find all skill directories in skills/ (excluding dev/)
    for skill_dir in "$skills_source"/*/ ; do
        # Skip if not a directory
        if [ ! -d "$skill_dir" ]; then
            continue
        fi
        
        skill_name=$(basename "$skill_dir")
        
        # Skip special directories and dev directory
        if [ "$skill_name" = "_shared" ] || [ "$skill_name" = "dev" ]; then
            continue
        fi
        
        # Verify it's a valid skill (has SKILL.md)
        if [ ! -f "$skill_dir/SKILL.md" ]; then
            log_warn "Skipping $skill_name (no SKILL.md found)"
            continue
        fi
        
        target_skill="$SKILLS_DIR/$skill_name"
        
        # Remove existing installation
        if [ -e "$target_skill" ]; then
            rm -rf "$target_skill"
        fi
        
        # Get absolute path for symlink
        abs_skill_dir=$(cd "$skill_dir" && pwd) || continue
        
        # Try symlink first (better for development), fall back to copy
        if ln -s "$abs_skill_dir" "$target_skill" 2>/dev/null; then
            log_success "Linked user skill: $skill_name (symlink)"
            user_installed_skills+=("$skill_name")
            user_installed_count=$((user_installed_count + 1))
        else
            # Fall back to copy
            if cp -r "$skill_dir" "$target_skill" 2>/dev/null; then
                log_success "Installed user skill: $skill_name (copy)"
                user_installed_skills+=("$skill_name")
                user_installed_count=$((user_installed_count + 1))
            else
                log_error "Failed to install user skill: $skill_name"
            fi
        fi
    done
    
    if [ $user_installed_count -eq 0 ]; then
        log_warn "No user skills installed (no SKILL.md files found)"
    else
        log_success "Installed $user_installed_count user skill(s) to $SKILLS_DIR"
        log_info "User skills installed: ${user_installed_skills[*]}"
    fi
    
    # Install development skills if requested
    if [ "$INSTALL_DEV_SKILLS" = "true" ]; then
        local dev_installed_count=0
        local dev_installed_skills=()
        
        log_info "Installing development skills..."
        
        if [ -d "$skills_source/dev" ]; then
            for skill_dir in "$skills_source/dev"/*/ ; do
                # Skip if not a directory
                if [ ! -d "$skill_dir" ]; then
                    continue
                fi
                
                skill_name=$(basename "$skill_dir")
                
                # Verify it's a valid skill (has SKILL.md)
                if [ ! -f "$skill_dir/SKILL.md" ]; then
                    log_warn "Skipping dev skill $skill_name (no SKILL.md found)"
                    continue
                fi
                
                target_skill="$SKILLS_DIR/$skill_name"
                
                # Remove existing installation
                if [ -e "$target_skill" ]; then
                    rm -rf "$target_skill"
                fi
                
                # Get absolute path for symlink
                abs_skill_dir=$(cd "$skill_dir" && pwd) || continue
                
                # Try symlink first (better for development), fall back to copy
                if ln -s "$abs_skill_dir" "$target_skill" 2>/dev/null; then
                    log_success "Linked dev skill: $skill_name (symlink)"
                    dev_installed_skills+=("$skill_name")
                    dev_installed_count=$((dev_installed_count + 1))
                else
                    # Fall back to copy
                    if cp -r "$skill_dir" "$target_skill" 2>/dev/null; then
                        log_success "Installed dev skill: $skill_name (copy)"
                        dev_installed_skills+=("$skill_name")
                        dev_installed_count=$((dev_installed_count + 1))
                    else
                        log_error "Failed to install dev skill: $skill_name"
                    fi
                fi
            done
            
            if [ $dev_installed_count -eq 0 ]; then
                log_warn "No development skills installed (no SKILL.md files found)"
            else
                log_success "Installed $dev_installed_count development skill(s) to $SKILLS_DIR"
                log_info "Development skills installed: ${dev_installed_skills[*]}"
            fi
        else
            log_warn "No skills/dev/ directory found"
        fi
    else
        log_info "Skipping development skills (set INSTALL_DEV_SKILLS=true to include)"
    fi
    
    # Install global agent instructions
    install_agent_instructions
}

# Install global agent instructions file
install_agent_instructions() {
    log_info "Installing global agent instructions..."
    
    mkdir -p "$(dirname "$AGENT_INSTRUCTIONS")"
    
    # Create AGENT_INSTRUCTIONS.md
    cat > "$AGENT_INSTRUCTIONS" << 'EOF'
# Global Agent Instructions

These instructions apply to ALL code agents using Alejandría memory system.

---

## Memory System Available

You have access to Alejandría persistent memory via MCP server.

**Commands available**:
- `mem_store`: Save new memory
- `mem_recall`: Search memories
- `mem_update`: Update existing memory
- `mem_forget`: Delete memory
- `mem_list_topics`: List all topics

---

## MANDATORY: Memory Discipline

**Before saving ANY memory**, you MUST read:
`~/.config/opencode/skills/memory-discipline/SKILL.md`

**Quick Rules**:
1. Save IMMEDIATELY after completing tasks (not at session end)
2. Include WHY, not just WHAT
3. Use structured format: What/Why/Where/Learned/Impact
4. Save to appropriate topic (search existing topics first)
5. Don't save obvious code or duplicates

**When to save**:
- ✅ Bug fixes (symptoms + root cause)
- ✅ Architecture decisions (alternatives + trade-offs)
- ✅ Non-obvious discoveries (surprises + gotchas)
- ✅ Implementation patterns (reusable templates)
- ✅ Workarounds (problem + future ideal solution)

**When NOT to save**:
- ❌ Code that can be read from files
- ❌ Obvious facts without context
- ❌ Incomplete thoughts
- ❌ Duplicates (search first!)

---

## Memory Structure Template

```markdown
## [Type]: [Short Title] ([Version/Context])

### What
[1-2 sentences: What was done]

### Why
[Context that motivated this]

### Where
[Files/locations affected with line numbers]

### Learned
[Insights, gotchas, surprises - MOST VALUABLE]

### Impact
[How this affects future work]
```

---

## Pre-Commit Checklist

- [ ] Did I make a non-obvious decision? → Save it
- [ ] Did I fix a bug? → Save symptoms + root cause
- [ ] Did I discover a gotcha? → Save the surprise
- [ ] Would this help me 6 months from now? → Save it

---

Last updated: 2026-04-12
Installed by: Alejandría installer v5
EOF

    log_success "Created global agent instructions: $AGENT_INSTRUCTIONS"
    
    # Show summary
    echo
    log_info "Skills and instructions installed for:"
    log_info "  • OpenCode (MCP client)"
    log_info "  • Claude Desktop (MCP client)"
    log_info "  • VSCode with MCP extension"
    log_info "  • GitHub Copilot (via instructions file)"
    log_info "  • Any agent configured to read ~/.config/opencode/"
}

# Main installation flow
main() {
    log_info "Alejandría Installer v5"
    echo

    # Detect platform
    local target
    target=$(detect_platform) || exit 1
    log_info "Detected platform: $target"

    # Get version
    if [ "$VERSION" = "latest" ]; then
        VERSION=$(get_latest_version) || {
            log_warn "Could not fetch latest version, will try pre-built binaries"
            VERSION="main"  # Use main branch binaries as fallback
        }
        log_info "Target version: $VERSION"
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

    # Install binary (priority order)
    if [ "$FORCE_BUILD" = "true" ]; then
        build_from_source || exit 1
    elif [ -f "$INSTALL_DIR/alejandria" ] && [ "$VERSION" = "$existing_version" ]; then
        log_success "Using existing binary v$VERSION"
    else
        # Try 1: Pre-built binary in repo (if we're in the repo)
        if use_prebuilt_binary "$target" "." 2>/dev/null; then
            log_success "Installed from pre-built binary"
        # Try 2: Download from GitLab API (workaround for clone cache issues)
        elif [ "$SOURCE_TYPE" = "gitlab" ] && download_from_gitlab_api "$target" 2>/dev/null; then
            log_success "Downloaded binary from GitLab API"
        # Try 3: Download from release/package registry
        elif download_binary "$VERSION" "$target" 2>/dev/null; then
            log_success "Downloaded and installed binary"
        # Try 4: Build from source
        else
            log_warn "No pre-built binary or download available, building from source"
            build_from_source || exit 1
        fi
    fi

    # Verify installation (also accept .new if mv didn't complete)
    local verify_bin="$INSTALL_DIR/alejandria"
    if ! "$verify_bin" --version >/dev/null 2>&1; then
        if [ -x "$INSTALL_DIR/alejandria.new" ]; then
            verify_bin="$INSTALL_DIR/alejandria.new"
            log_warn "Verification using alejandria.new — run: mv $INSTALL_DIR/alejandria.new $INSTALL_DIR/alejandria"
        else
            log_error "Installation verification failed"
            exit 1
        fi
    fi

    local installed_version
    installed_version=$("$verify_bin" --version | awk '{print $2}')
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

    # Install skills globally (NEW in v5)
    if [ "$INSTALL_SKILLS" = "true" ]; then
        echo
        install_skills
    fi

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
    echo "  alejandria store \"Test memory\" --summary \"Test\""
    echo "  alejandria recall \"test\""
    
    echo
    log_success "Installation complete!"
}

main "$@"
