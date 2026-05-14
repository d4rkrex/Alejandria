#!/usr/bin/env bash
set -euo pipefail

# Alejandria Uninstaller
# Removes Alejandria binary and MCP client configurations
# Usage: ./scripts/uninstall-mcp.sh [--keep-data]

INSTALL_DIR="${ALEJANDRIA_INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="$HOME/.config/alejandria"
DATA_DIR="$HOME/.local/share/alejandria"
KEEP_DATA=false

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ${NC} $*"; }
log_success() { echo -e "${GREEN}✓${NC} $*"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $*"; }
log_error() { echo -e "${RED}✗${NC} $*"; }

# Parse arguments
for arg in "$@"; do
    case $arg in
        --keep-data)
            KEEP_DATA=true
            shift
            ;;
        --help|-h)
            cat <<EOF
Alejandria Uninstaller

Usage: $0 [OPTIONS]

Options:
  --keep-data    Keep database and configuration files
  --help, -h     Show this help message

Examples:
  $0                # Complete removal (binary + configs + data)
  $0 --keep-data    # Remove only binary and MCP configs, keep data
EOF
            exit 0
            ;;
    esac
done

echo -e "${BLUE}Alejandria Uninstaller${NC}\n"

# Confirm uninstallation
if [ "$KEEP_DATA" = false ]; then
    log_warn "This will remove:"
    echo "  - Binary: $INSTALL_DIR/alejandria"
    echo "  - Config: $CONFIG_DIR"
    echo "  - Data: $DATA_DIR (including all memories)"
    echo "  - MCP client configurations (backups will be kept)"
else
    log_warn "This will remove:"
    echo "  - Binary: $INSTALL_DIR/alejandria"
    echo "  - MCP client configurations (backups will be kept)"
    echo ""
    log_info "Data will be KEPT:"
    echo "  - Config: $CONFIG_DIR"
    echo "  - Data: $DATA_DIR"
fi

echo ""
read -p "Continue with uninstallation? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    log_info "Uninstallation cancelled"
    exit 0
fi

# Remove binary
if [ -f "$INSTALL_DIR/alejandria" ]; then
    rm -f "$INSTALL_DIR/alejandria"
    log_success "Binary removed: $INSTALL_DIR/alejandria"
else
    log_info "Binary not found: $INSTALL_DIR/alejandria"
fi

# Remove from MCP clients
removed_count=0

# OpenCode
OPENCODE_CONFIG="$HOME/.config/opencode/opencode.json"
if [ -f "$OPENCODE_CONFIG" ]; then
    if command -v jq >/dev/null 2>&1; then
        if jq -e '.mcp.alejandria' "$OPENCODE_CONFIG" >/dev/null 2>&1; then
            # Backup
            backup_file="${OPENCODE_CONFIG}.backup-uninstall-$(date +%Y%m%d-%H%M%S)"
            cp "$OPENCODE_CONFIG" "$backup_file"
            
            # Remove alejandria entry
            jq 'del(.mcp.alejandria)' "$OPENCODE_CONFIG" > "${OPENCODE_CONFIG}.tmp"
            mv "${OPENCODE_CONFIG}.tmp" "$OPENCODE_CONFIG"
            
            log_success "Removed from OpenCode (backup: $(basename "$backup_file"))"
            ((removed_count++))
        fi
    elif command -v python3 >/dev/null 2>&1; then
        if python3 -c "import json; config=json.load(open('$OPENCODE_CONFIG')); exit(0 if 'alejandria' in config.get('mcp', {}) else 1)" 2>/dev/null; then
            backup_file="${OPENCODE_CONFIG}.backup-uninstall-$(date +%Y%m%d-%H%M%S)"
            cp "$OPENCODE_CONFIG" "$backup_file"
            
            python3 <<EOF
import json
with open('$OPENCODE_CONFIG', 'r') as f:
    config = json.load(f)
if 'mcp' in config and 'alejandria' in config['mcp']:
    del config['mcp']['alejandria']
with open('$OPENCODE_CONFIG', 'w') as f:
    json.dump(config, f, indent=2)
EOF
            log_success "Removed from OpenCode (backup: $(basename "$backup_file"))"
            ((removed_count++))
        fi
    fi
fi

# Claude Desktop
CLAUDE_CONFIG="$HOME/.config/Claude/claude_desktop_config.json"
if [ ! -f "$CLAUDE_CONFIG" ]; then
    CLAUDE_CONFIG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
fi

if [ -f "$CLAUDE_CONFIG" ]; then
    if command -v jq >/dev/null 2>&1; then
        if jq -e '.mcpServers.alejandria' "$CLAUDE_CONFIG" >/dev/null 2>&1; then
            backup_file="${CLAUDE_CONFIG}.backup-uninstall-$(date +%Y%m%d-%H%M%S)"
            cp "$CLAUDE_CONFIG" "$backup_file"
            
            jq 'del(.mcpServers.alejandria)' "$CLAUDE_CONFIG" > "${CLAUDE_CONFIG}.tmp"
            mv "${CLAUDE_CONFIG}.tmp" "$CLAUDE_CONFIG"
            
            log_success "Removed from Claude Desktop (backup: $(basename "$backup_file"))"
            ((removed_count++))
        fi
    elif command -v python3 >/dev/null 2>&1; then
        if python3 -c "import json; config=json.load(open('$CLAUDE_CONFIG')); exit(0 if 'alejandria' in config.get('mcpServers', {}) else 1)" 2>/dev/null; then
            backup_file="${CLAUDE_CONFIG}.backup-uninstall-$(date +%Y%m%d-%H%M%S)"
            cp "$CLAUDE_CONFIG" "$backup_file"
            
            python3 <<EOF
import json
with open('$CLAUDE_CONFIG', 'r') as f:
    config = json.load(f)
if 'mcpServers' in config and 'alejandria' in config['mcpServers']:
    del config['mcpServers']['alejandria']
with open('$CLAUDE_CONFIG', 'w') as f:
    json.dump(config, f, indent=2)
EOF
            log_success "Removed from Claude Desktop (backup: $(basename "$backup_file"))"
            ((removed_count++))
        fi
    fi
fi

# VSCode
VSCODE_CONFIG="$HOME/.config/Code/User/settings.json"
if [ -f "$VSCODE_CONFIG" ]; then
    if command -v jq >/dev/null 2>&1; then
        if jq -e '.mcp.servers.alejandria' "$VSCODE_CONFIG" >/dev/null 2>&1; then
            backup_file="${VSCODE_CONFIG}.backup-uninstall-$(date +%Y%m%d-%H%M%S)"
            cp "$VSCODE_CONFIG" "$backup_file"
            
            jq 'del(.mcp.servers.alejandria)' "$VSCODE_CONFIG" > "${VSCODE_CONFIG}.tmp"
            mv "${VSCODE_CONFIG}.tmp" "$VSCODE_CONFIG"
            
            log_success "Removed from VSCode (backup: $(basename "$backup_file"))"
            ((removed_count++))
        fi
    fi
fi

if [ $removed_count -eq 0 ]; then
    log_info "No MCP client configurations found"
else
    log_success "Removed from $removed_count MCP client(s)"
fi

# Remove config and data (unless --keep-data)
if [ "$KEEP_DATA" = false ]; then
    if [ -d "$CONFIG_DIR" ]; then
        rm -rf "$CONFIG_DIR"
        log_success "Config removed: $CONFIG_DIR"
    fi
    
    if [ -d "$DATA_DIR" ]; then
        # Show database stats before removal
        if [ -f "$DATA_DIR/alejandria.db" ]; then
            if command -v sqlite3 >/dev/null 2>&1; then
                memory_count=$(sqlite3 "$DATA_DIR/alejandria.db" "SELECT COUNT(*) FROM memories WHERE deleted_at IS NULL" 2>/dev/null || echo "unknown")
                log_warn "Removing database with $memory_count memories"
            fi
        fi
        
        rm -rf "$DATA_DIR"
        log_success "Data removed: $DATA_DIR"
    fi
else
    log_info "Data preserved:"
    if [ -d "$CONFIG_DIR" ]; then
        echo "  - Config: $CONFIG_DIR"
    fi
    if [ -d "$DATA_DIR" ]; then
        if [ -f "$DATA_DIR/alejandria.db" ]; then
            if command -v sqlite3 >/dev/null 2>&1; then
                memory_count=$(sqlite3 "$DATA_DIR/alejandria.db" "SELECT COUNT(*) FROM memories WHERE deleted_at IS NULL" 2>/dev/null || echo "unknown")
                echo "  - Database: $DATA_DIR/alejandria.db ($memory_count memories)"
            else
                echo "  - Database: $DATA_DIR/alejandria.db"
            fi
        fi
    fi
fi

echo ""
log_success "Uninstallation complete!"
echo ""

if [ $removed_count -gt 0 ]; then
    log_warn "Please restart your MCP clients to apply changes"
fi

if [ "$KEEP_DATA" = true ]; then
    echo ""
    log_info "To reinstall Alejandria with preserved data:"
    echo "  curl -fsSL https://raw.githubusercontent.com/VeritranGH/Alejandria/main/scripts/install.sh | bash"
fi
