#!/bin/bash
#
# Alejandría MCP Installer v2.1
# Configures ALL MCP clients with correct formats:
# - OpenCode (opencode.json format)
# - Claude Code CLI (.claude.json format)  
# - Claude Desktop (claude_desktop_config.json format)
# - VSCode/Copilot (settings.json with github.copilot.chat.mcp.servers)
# - GitHub Copilot standalone (mcp-config.json format)
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Banner
echo -e "${BLUE}"
cat << "EOF"
    _    _           _                _      _       
   / \  | | ___     | | __ _ _ __   __| |_ __(_) __ _ 
  / _ \ | |/ _ \ _  | |/ _` | '_ \ / _` | '__| |/ _` |
 / ___ \| |  __/ |_|| | (_| | | | | (_| | |  | | (_| |
/_/   \_\_|\___|\___/ \__,_|_| |_|\__,_|_|  |_|\__,_|
                                                      
    MCP Installer v2.1 - All Clients Support
EOF
echo -e "${NC}"

# Default values
BINARY_PATH="${HOME}/.local/bin/alejandria"
CONFIG_DIR="${HOME}/.config/alejandria"
DATA_DIR="${HOME}/.local/share/alejandria"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --binary)
            BINARY_PATH="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --binary PATH       Path to alejandria binary (default: ~/.local/bin/alejandria)"
            echo "  --help              Show this help"
            echo ""
            echo "This installer configures Alejandría MCP for:"
            echo "  • OpenCode (~/.config/opencode/opencode.json)"
            echo "  • Claude Code CLI (~/.claude.json)"
            echo "  • Claude Desktop (~/.config/Claude/claude_desktop_config.json)"
            echo "  • VSCode/Copilot (~/.config/Code/User/settings.json)"
            echo "  • GitHub Copilot standalone (~/.copilot/mcp-config.json)"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Validate binary exists
if [[ ! -f "$BINARY_PATH" ]]; then
    echo -e "${RED}Error: Binary not found at $BINARY_PATH${NC}"
    echo "Please install Alejandría first or specify correct path with --binary"
    exit 1
fi

echo -e "${BLUE}[1/8] Creating directories...${NC}"
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR"
mkdir -p "${HOME}/.config/Claude"
mkdir -p "${HOME}/.config/opencode"
mkdir -p "${HOME}/.config/Code/User"
mkdir -p "${HOME}/.copilot"
echo -e "${GREEN}✓ Directories created${NC}"

# Create Alejandría config
echo -e "${BLUE}[2/8] Creating Alejandría configuration...${NC}"
cat > "$CONFIG_DIR/config.toml" << EOF
# Alejandría Configuration - Local Mode (stdio)
db_path = "${DATA_DIR}/alejandria.db"

[memory]
max_memories = 100000
default_decay_profile = "exponential"
access_dampening_factor = 0.5

[embeddings]
# Set to true for semantic search, false for keyword-only (faster)
enabled = false

[decay]
half_life_days = 90
score_threshold = 0.1

[stdio]
enabled = true
EOF
echo -e "${GREEN}✓ Configuration created at ${CONFIG_DIR}/config.toml${NC}"

# Function to backup file
backup_file() {
    local file="$1"
    if [[ -f "$file" ]]; then
        cp "$file" "${file}.backup-$(date +%Y%m%d-%H%M%S)"
        echo -e "${YELLOW}  Backed up existing config${NC}"
    fi
}

# Configure OpenCode
echo -e "${BLUE}[3/8] Configuring OpenCode...${NC}"
OPENCODE_CONFIG="${HOME}/.config/opencode/opencode.json"

if [[ -f "$OPENCODE_CONFIG" ]]; then
    backup_file "$OPENCODE_CONFIG"
    
    if command -v jq &> /dev/null; then
        # Use jq to add alejandria to mcp section
        jq ".mcp.alejandria = {
            \"command\": [\"$BINARY_PATH\", \"serve\"],
            \"environment\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"},
            \"enabled\": true,
            \"type\": \"local\"
        }" "$OPENCODE_CONFIG" > "${OPENCODE_CONFIG}.tmp" && mv "${OPENCODE_CONFIG}.tmp" "$OPENCODE_CONFIG"
        echo -e "${GREEN}✓ OpenCode configured${NC}"
    else
        echo -e "${YELLOW}⚠ jq not found, manually add to opencode.json:${NC}"
        cat << EOF
"alejandria": {
  "command": ["$BINARY_PATH", "serve"],
  "environment": {"ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"},
  "enabled": true,
  "type": "local"
}
EOF
    fi
else
    echo -e "${YELLOW}⚠ OpenCode config not found at $OPENCODE_CONFIG${NC}"
fi

# Configure Claude Code CLI
echo -e "${BLUE}[4/8] Configuring Claude Code CLI...${NC}"
CLAUDE_CLI_CONFIG="${HOME}/.claude.json"

if [[ -f "$CLAUDE_CLI_CONFIG" ]]; then
    backup_file "$CLAUDE_CLI_CONFIG"
    
    if command -v jq &> /dev/null; then
        jq ".mcpServers.alejandria = {
            \"command\": \"$BINARY_PATH\",
            \"args\": [\"serve\"],
            \"env\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"},
            \"type\": \"stdio\"
        }" "$CLAUDE_CLI_CONFIG" > "${CLAUDE_CLI_CONFIG}.tmp" && mv "${CLAUDE_CLI_CONFIG}.tmp" "$CLAUDE_CLI_CONFIG"
        echo -e "${GREEN}✓ Claude Code CLI configured${NC}"
    else
        echo -e "${YELLOW}⚠ jq not found, manually add to .claude.json${NC}"
    fi
else
    echo -e "${YELLOW}⚠ Claude Code CLI config not found at $CLAUDE_CLI_CONFIG${NC}"
fi

# Configure Claude Desktop
echo -e "${BLUE}[5/8] Configuring Claude Desktop...${NC}"
CLAUDE_DESKTOP_CONFIG="${HOME}/.config/Claude/claude_desktop_config.json"
backup_file "$CLAUDE_DESKTOP_CONFIG"

if [[ -f "$CLAUDE_DESKTOP_CONFIG" ]]; then
    if command -v jq &> /dev/null; then
        jq ".mcpServers.alejandria = {
            \"command\": \"$BINARY_PATH\",
            \"args\": [\"serve\"],
            \"env\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"}
        }" "$CLAUDE_DESKTOP_CONFIG" > "${CLAUDE_DESKTOP_CONFIG}.tmp" && mv "${CLAUDE_DESKTOP_CONFIG}.tmp" "$CLAUDE_DESKTOP_CONFIG"
    else
        echo -e "${YELLOW}⚠ jq not found${NC}"
    fi
else
    # Create new file
    cat > "$CLAUDE_DESKTOP_CONFIG" << EOF
{
  "mcpServers": {
    "alejandria": {
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"
      }
    }
  }
}
EOF
fi
echo -e "${GREEN}✓ Claude Desktop configured${NC}"

# Configure VSCode/Copilot
echo -e "${BLUE}[6/8] Configuring VSCode/Copilot...${NC}"
VSCODE_SETTINGS="${HOME}/.config/Code/User/settings.json"
backup_file "$VSCODE_SETTINGS"

if [[ -f "$VSCODE_SETTINGS" ]]; then
    if command -v jq &> /dev/null; then
        # Add to both github.copilot.chat.mcp.servers AND mcp.servers
        jq ".[\"github.copilot.chat.mcp.servers\"].alejandria = {
            \"type\": \"stdio\",
            \"command\": \"$BINARY_PATH\",
            \"args\": [\"serve\"],
            \"env\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"}
        } | .[\"mcp.servers\"].alejandria = {
            \"command\": \"$BINARY_PATH\",
            \"args\": [\"serve\"],
            \"env\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"}
        }" "$VSCODE_SETTINGS" > "${VSCODE_SETTINGS}.tmp" && mv "${VSCODE_SETTINGS}.tmp" "$VSCODE_SETTINGS"
    else
        echo -e "${YELLOW}⚠ jq not found${NC}"
    fi
else
    # Create new file
    cat > "$VSCODE_SETTINGS" << EOF
{
  "github.copilot.chat.mcp.servers": {
    "alejandria": {
      "type": "stdio",
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"
      }
    }
  },
  "mcp.servers": {
    "alejandria": {
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"
      }
    }
  }
}
EOF
fi
echo -e "${GREEN}✓ VSCode/Copilot configured${NC}"

# Configure GitHub Copilot standalone
echo -e "${BLUE}[7/8] Configuring GitHub Copilot standalone...${NC}"
COPILOT_CONFIG="${HOME}/.copilot/mcp-config.json"
backup_file "$COPILOT_CONFIG"

if [[ -f "$COPILOT_CONFIG" ]]; then
    if command -v jq &> /dev/null; then
        jq ".mcpServers.alejandria = {
            \"command\": \"$BINARY_PATH\",
            \"args\": [\"serve\"],
            \"env\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"}
        }" "$COPILOT_CONFIG" > "${COPILOT_CONFIG}.tmp" && mv "${COPILOT_CONFIG}.tmp" "$COPILOT_CONFIG"
    else
        echo -e "${YELLOW}⚠ jq not found${NC}"
    fi
else
    # Create new file
    cat > "$COPILOT_CONFIG" << EOF
{
  "mcpServers": {
    "alejandria": {
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"
      }
    }
  }
}
EOF
fi
echo -e "${GREEN}✓ GitHub Copilot standalone configured${NC}"

# Create test script
echo -e "${BLUE}[8/8] Creating test script...${NC}"
cat > "${HOME}/.local/bin/test-alejandria" << 'TESTEOF'
#!/bin/bash
echo "Testing Alejandría installation..."
echo ""
echo "1. Testing binary..."
alejandria --version
echo ""
echo "2. Storing test memory..."
alejandria store "Test memory from installation script" \
    --topic "installation" \
    --importance "medium"
echo ""
echo "3. Recalling test memory..."
alejandria recall "test installation" 2>&1 | head -20
echo ""
echo "4. Showing stats..."
alejandria stats
echo ""
echo "✓ Installation test complete!"
TESTEOF
chmod +x "${HOME}/.local/bin/test-alejandria"
echo -e "${GREEN}✓ Test script created${NC}"

# Summary
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}           Installation Complete! 🎉${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BLUE}Binary:${NC} $BINARY_PATH"
echo -e "${BLUE}Config:${NC} $CONFIG_DIR/config.toml"
echo -e "${BLUE}Database:${NC} $DATA_DIR/alejandria.db"
echo ""
echo -e "${BLUE}Configured clients:${NC}"
echo "  • OpenCode: ~/.config/opencode/opencode.json"
echo "  • Claude Code CLI: ~/.claude.json"
echo "  • Claude Desktop: ~/.config/Claude/claude_desktop_config.json"
echo "  • VSCode/Copilot: ~/.config/Code/User/settings.json"
echo "  • GitHub Copilot standalone: ~/.copilot/mcp-config.json"
echo ""
echo -e "${YELLOW}⚠ IMPORTANT: Restart ALL clients to detect Alejandría${NC}"
echo ""
echo -e "${BLUE}Restart commands:${NC}"
echo "  • OpenCode: pkill -9 opencode && opencode"
echo "  • Claude Code: /exit (then reopen)"
echo "  • Claude Desktop: Close app and reopen"
echo "  • VSCode: Ctrl+Shift+P → 'Developer: Reload Window'"
echo "  • GitHub Copilot standalone: Restart CLI session"
echo ""
echo -e "${BLUE}Test installation:${NC}"
echo "  test-alejandria"
echo ""
echo -e "${GREEN}Happy memory building! 🧠${NC}"
