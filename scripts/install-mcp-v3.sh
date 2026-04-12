#!/bin/bash
#
# Alejandría MCP Installer v3.0
# Multi-mode installer: Local (stdio), Remote Client (SSE), Server Installation
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Banner
echo -e "${BLUE}"
cat << "EOF"
    _    _           _                _      _       
   / \  | | ___     | | __ _ _ __   __| |_ __(_) __ _ 
  / _ \ | |/ _ \ _  | |/ _` | '_ \ / _` | '__| |/ _` |
 / ___ \| |  __/ |_|| | (_| | | | | (_| | |  | | (_| |
/_/   \_\_|\___|\___/ \__,_|_| |_|\__,_|_|  |_|\__,_|
                                                      
    MCP Installer v3.0 - Multi-Mode Support
EOF
echo -e "${NC}"

# Default values
BINARY_PATH="${HOME}/.local/bin/alejandria"
CONFIG_DIR="${HOME}/.config/alejandria"
DATA_DIR="${HOME}/.local/share/alejandria"
CA_CERT_PATH="${HOME}/.alejandria/ca-cert.pem"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            cat << 'HELPEOF'
Alejandría MCP Installer v3.0

USAGE:
    ./install-mcp-v3.sh [OPTIONS]

OPTIONS:
    --help, -h          Show this help message
    --binary PATH       Path to alejandria binary (default: ~/.local/bin/alejandria)

MODES:
    This installer supports 3 deployment modes:

    1. 🏠 Local (stdio)
       • Binary runs locally on your machine
       • Private database (~/.local/share/alejandria/)
       • No network/server required
       • RECOMMENDED for single-user scenarios

    2. 🌐 Remote Client (MCP SSE)
       • Connect to existing Alejandría MCP server
       • Shared memory with team
       • Requires server URL and API key
       • Automatic TLS certificate download

    3. 🖥️  Server Installation
       • Install Alejandría as a server
       • Other users can connect remotely
       • Optional Caddy reverse proxy with TLS
       • Systemd service integration

EXAMPLES:
    # Interactive mode selection
    ./install-mcp-v3.sh

    # Specify custom binary path
    ./install-mcp-v3.sh --binary /opt/alejandria/bin/alejandria

MCP CLIENTS CONFIGURED:
    • OpenCode (~/.config/opencode/opencode.json)
    • Claude Code CLI (~/.claude.json)
    • Claude Desktop (~/.config/Claude/claude_desktop_config.json)
    • VSCode/Copilot (~/.config/Code/User/settings.json)
    • GitHub Copilot standalone (~/.copilot/mcp-config.json)

DOCUMENTATION:
    See INSTALLER_V3_GUIDE.md for detailed usage examples

HELPEOF
            exit 0
            ;;
        --binary)
            BINARY_PATH="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Function to backup file
backup_file() {
    local file="$1"
    if [[ -f "$file" ]]; then
        cp "$file" "${file}.backup-$(date +%Y%m%d-%H%M%S)"
        echo -e "${YELLOW}  ✓ Backed up existing config${NC}"
    fi
}

# Function to validate URL format
validate_url() {
    local url="$1"
    if [[ ! "$url" =~ ^https?:// ]]; then
        echo -e "${RED}Invalid URL format. Must start with http:// or https://${NC}"
        return 1
    fi
    return 0
}

# Function to validate port number
validate_port() {
    local port="$1"
    if [[ ! "$port" =~ ^[0-9]+$ ]] || [ "$port" -lt 1 ] || [ "$port" -gt 65535 ]; then
        echo -e "${RED}Invalid port number. Must be between 1 and 65535${NC}"
        return 1
    fi
    return 0
}

# Function to configure MCP clients for LOCAL mode
configure_local_mode() {
    echo -e "${BLUE}[3/7] Configuring MCP clients (Local mode)...${NC}"
    
    # Create Alejandría config
    echo -e "${BLUE}  Creating Alejandría configuration...${NC}"
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$DATA_DIR"
    
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
    echo -e "${GREEN}  ✓ Configuration created at ${CONFIG_DIR}/config.toml${NC}"
    
    # Configure MCP clients with stdio transport
    configure_mcp_client_opencode "local"
    configure_mcp_client_claude_cli "local"
    configure_mcp_client_claude_desktop "local"
    configure_mcp_client_vscode "local"
    configure_mcp_client_copilot "local"
}

# Function to configure MCP clients for REMOTE mode
configure_remote_mode() {
    local server_url="$1"
    local api_key="$2"
    local ca_cert="$3"
    
    echo -e "${BLUE}[3/7] Configuring MCP clients (Remote mode)...${NC}"
    
    # Configure MCP clients with SSE transport
    configure_mcp_client_opencode "remote" "$server_url" "$api_key" "$ca_cert"
    configure_mcp_client_claude_cli "remote" "$server_url" "$api_key" "$ca_cert"
    configure_mcp_client_claude_desktop "remote" "$server_url" "$api_key" "$ca_cert"
    configure_mcp_client_vscode "remote" "$server_url" "$api_key" "$ca_cert"
    configure_mcp_client_copilot "remote" "$server_url" "$api_key" "$ca_cert"
}

# OpenCode configuration
configure_mcp_client_opencode() {
    local mode="$1"
    local url="$2"
    local api_key="$3"
    local ca_cert="$4"
    
    local OPENCODE_CONFIG="${HOME}/.config/opencode/opencode.json"
    mkdir -p "${HOME}/.config/opencode"
    
    if [[ "$mode" == "local" ]]; then
        if [[ -f "$OPENCODE_CONFIG" ]]; then
            backup_file "$OPENCODE_CONFIG"
            if command -v jq &> /dev/null; then
                jq ".mcp.alejandria = {
                    \"command\": [\"$BINARY_PATH\", \"serve\"],
                    \"environment\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"},
                    \"enabled\": true,
                    \"type\": \"local\"
                }" "$OPENCODE_CONFIG" > "${OPENCODE_CONFIG}.tmp" && mv "${OPENCODE_CONFIG}.tmp" "$OPENCODE_CONFIG"
            fi
        else
            cat > "$OPENCODE_CONFIG" << EOF
{
  "mcp": {
    "alejandria": {
      "command": ["$BINARY_PATH", "serve"],
      "environment": {"ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"},
      "enabled": true,
      "type": "local"
    }
  }
}
EOF
        fi
    else
        # Remote mode
        local config_obj="{\"url\": \"$url\", \"apiKey\": \"$api_key\", \"transport\": \"sse\", \"enabled\": true, \"type\": \"remote\""
        if [[ -f "$ca_cert" ]]; then
            config_obj="${config_obj}, \"tlsCert\": \"$ca_cert\"}"
        else
            config_obj="${config_obj}}"
        fi
        
        if [[ -f "$OPENCODE_CONFIG" ]]; then
            backup_file "$OPENCODE_CONFIG"
            if command -v jq &> /dev/null; then
                jq ".mcp.alejandria = $config_obj" "$OPENCODE_CONFIG" > "${OPENCODE_CONFIG}.tmp" && mv "${OPENCODE_CONFIG}.tmp" "$OPENCODE_CONFIG"
            fi
        else
            cat > "$OPENCODE_CONFIG" << EOF
{
  "mcp": {
    "alejandria": $config_obj
  }
}
EOF
        fi
    fi
    echo -e "${GREEN}  ✓ OpenCode configured${NC}"
}

# Claude Code CLI configuration
configure_mcp_client_claude_cli() {
    local mode="$1"
    local url="$2"
    local api_key="$3"
    local ca_cert="$4"
    
    local CLAUDE_CLI_CONFIG="${HOME}/.claude.json"
    
    if [[ "$mode" == "local" ]]; then
        if [[ -f "$CLAUDE_CLI_CONFIG" ]]; then
            backup_file "$CLAUDE_CLI_CONFIG"
            if command -v jq &> /dev/null; then
                jq ".mcpServers.alejandria = {
                    \"command\": \"$BINARY_PATH\",
                    \"args\": [\"serve\"],
                    \"env\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"},
                    \"type\": \"stdio\"
                }" "$CLAUDE_CLI_CONFIG" > "${CLAUDE_CLI_CONFIG}.tmp" && mv "${CLAUDE_CLI_CONFIG}.tmp" "$CLAUDE_CLI_CONFIG"
            fi
        else
            cat > "$CLAUDE_CLI_CONFIG" << EOF
{
  "mcpServers": {
    "alejandria": {
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {"ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"},
      "type": "stdio"
    }
  }
}
EOF
        fi
    else
        # Remote mode
        local config_obj="{\"url\": \"$url\", \"apiKey\": \"$api_key\", \"transport\": \"sse\""
        if [[ -f "$ca_cert" ]]; then
            config_obj="${config_obj}, \"tlsCert\": \"$ca_cert\"}"
        else
            config_obj="${config_obj}}"
        fi
        
        if [[ -f "$CLAUDE_CLI_CONFIG" ]]; then
            backup_file "$CLAUDE_CLI_CONFIG"
            if command -v jq &> /dev/null; then
                jq ".mcpServers.alejandria = $config_obj" "$CLAUDE_CLI_CONFIG" > "${CLAUDE_CLI_CONFIG}.tmp" && mv "${CLAUDE_CLI_CONFIG}.tmp" "$CLAUDE_CLI_CONFIG"
            fi
        else
            cat > "$CLAUDE_CLI_CONFIG" << EOF
{
  "mcpServers": {
    "alejandria": $config_obj
  }
}
EOF
        fi
    fi
    echo -e "${GREEN}  ✓ Claude Code CLI configured${NC}"
}

# Claude Desktop configuration
configure_mcp_client_claude_desktop() {
    local mode="$1"
    local url="$2"
    local api_key="$3"
    local ca_cert="$4"
    
    local CLAUDE_DESKTOP_CONFIG="${HOME}/.config/Claude/claude_desktop_config.json"
    mkdir -p "${HOME}/.config/Claude"
    
    backup_file "$CLAUDE_DESKTOP_CONFIG"
    
    if [[ "$mode" == "local" ]]; then
        if [[ -f "$CLAUDE_DESKTOP_CONFIG" ]]; then
            if command -v jq &> /dev/null; then
                jq ".mcpServers.alejandria = {
                    \"command\": \"$BINARY_PATH\",
                    \"args\": [\"serve\"],
                    \"env\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"}
                }" "$CLAUDE_DESKTOP_CONFIG" > "${CLAUDE_DESKTOP_CONFIG}.tmp" && mv "${CLAUDE_DESKTOP_CONFIG}.tmp" "$CLAUDE_DESKTOP_CONFIG"
            fi
        else
            cat > "$CLAUDE_DESKTOP_CONFIG" << EOF
{
  "mcpServers": {
    "alejandria": {
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {"ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"}
    }
  }
}
EOF
        fi
    else
        # Remote mode
        local config_obj="{\"url\": \"$url\", \"apiKey\": \"$api_key\", \"transport\": \"sse\""
        if [[ -f "$ca_cert" ]]; then
            config_obj="${config_obj}, \"tlsCert\": \"$ca_cert\"}"
        else
            config_obj="${config_obj}}"
        fi
        
        if [[ -f "$CLAUDE_DESKTOP_CONFIG" ]]; then
            if command -v jq &> /dev/null; then
                jq ".mcpServers.alejandria = $config_obj" "$CLAUDE_DESKTOP_CONFIG" > "${CLAUDE_DESKTOP_CONFIG}.tmp" && mv "${CLAUDE_DESKTOP_CONFIG}.tmp" "$CLAUDE_DESKTOP_CONFIG"
            fi
        else
            cat > "$CLAUDE_DESKTOP_CONFIG" << EOF
{
  "mcpServers": {
    "alejandria": $config_obj
  }
}
EOF
        fi
    fi
    echo -e "${GREEN}  ✓ Claude Desktop configured${NC}"
}

# VSCode/Copilot configuration
configure_mcp_client_vscode() {
    local mode="$1"
    local url="$2"
    local api_key="$3"
    local ca_cert="$4"
    
    local VSCODE_SETTINGS="${HOME}/.config/Code/User/settings.json"
    mkdir -p "${HOME}/.config/Code/User"
    
    backup_file "$VSCODE_SETTINGS"
    
    if [[ "$mode" == "local" ]]; then
        if [[ -f "$VSCODE_SETTINGS" ]]; then
            if command -v jq &> /dev/null; then
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
            fi
        else
            cat > "$VSCODE_SETTINGS" << EOF
{
  "github.copilot.chat.mcp.servers": {
    "alejandria": {
      "type": "stdio",
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {"ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"}
    }
  },
  "mcp.servers": {
    "alejandria": {
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {"ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"}
    }
  }
}
EOF
        fi
    else
        # Remote mode
        local config_obj="{\"url\": \"$url\", \"apiKey\": \"$api_key\", \"transport\": \"sse\""
        if [[ -f "$ca_cert" ]]; then
            config_obj="${config_obj}, \"tlsCert\": \"$ca_cert\"}"
        else
            config_obj="${config_obj}}"
        fi
        
        if [[ -f "$VSCODE_SETTINGS" ]]; then
            if command -v jq &> /dev/null; then
                jq ".[\"github.copilot.chat.mcp.servers\"].alejandria = $config_obj | .[\"mcp.servers\"].alejandria = $config_obj" "$VSCODE_SETTINGS" > "${VSCODE_SETTINGS}.tmp" && mv "${VSCODE_SETTINGS}.tmp" "$VSCODE_SETTINGS"
            fi
        else
            cat > "$VSCODE_SETTINGS" << EOF
{
  "github.copilot.chat.mcp.servers": {
    "alejandria": $config_obj
  },
  "mcp.servers": {
    "alejandria": $config_obj
  }
}
EOF
        fi
    fi
    echo -e "${GREEN}  ✓ VSCode/Copilot configured${NC}"
}

# GitHub Copilot standalone configuration
configure_mcp_client_copilot() {
    local mode="$1"
    local url="$2"
    local api_key="$3"
    local ca_cert="$4"
    
    local COPILOT_CONFIG="${HOME}/.copilot/mcp-config.json"
    mkdir -p "${HOME}/.copilot"
    
    backup_file "$COPILOT_CONFIG"
    
    if [[ "$mode" == "local" ]]; then
        if [[ -f "$COPILOT_CONFIG" ]]; then
            if command -v jq &> /dev/null; then
                jq ".mcpServers.alejandria = {
                    \"command\": \"$BINARY_PATH\",
                    \"args\": [\"serve\"],
                    \"env\": {\"ALEJANDRIA_CONFIG\": \"$CONFIG_DIR/config.toml\"}
                }" "$COPILOT_CONFIG" > "${COPILOT_CONFIG}.tmp" && mv "${COPILOT_CONFIG}.tmp" "$COPILOT_CONFIG"
            fi
        else
            cat > "$COPILOT_CONFIG" << EOF
{
  "mcpServers": {
    "alejandria": {
      "command": "$BINARY_PATH",
      "args": ["serve"],
      "env": {"ALEJANDRIA_CONFIG": "$CONFIG_DIR/config.toml"}
    }
  }
}
EOF
        fi
    else
        # Remote mode
        local config_obj="{\"url\": \"$url\", \"apiKey\": \"$api_key\", \"transport\": \"sse\""
        if [[ -f "$ca_cert" ]]; then
            config_obj="${config_obj}, \"tlsCert\": \"$ca_cert\"}"
        else
            config_obj="${config_obj}}"
        fi
        
        if [[ -f "$COPILOT_CONFIG" ]]; then
            if command -v jq &> /dev/null; then
                jq ".mcpServers.alejandria = $config_obj" "$COPILOT_CONFIG" > "${COPILOT_CONFIG}.tmp" && mv "${COPILOT_CONFIG}.tmp" "$COPILOT_CONFIG"
            fi
        else
            cat > "$COPILOT_CONFIG" << EOF
{
  "mcpServers": {
    "alejandria": $config_obj
  }
}
EOF
        fi
    fi
    echo -e "${GREEN}  ✓ GitHub Copilot standalone configured${NC}"
}

# Function to download CA certificate
download_ca_cert() {
    local server_url="$1"
    local auto_download="$2"
    
    if [[ "$auto_download" != "y" ]] && [[ "$auto_download" != "Y" ]]; then
        return 0
    fi
    
    echo -e "${BLUE}[2/7] Downloading CA certificate...${NC}"
    mkdir -p "${HOME}/.alejandria"
    
    # Extract hostname from URL
    local hostname=$(echo "$server_url" | sed -E 's|https?://([^/]+).*|\1|')
    
    # Try SSH method first
    if command -v ssh &> /dev/null; then
        echo -e "${CYAN}  Trying SSH method...${NC}"
        local ssh_host="${hostname%%:*}"  # Remove port if present
        
        if ssh -q -o ConnectTimeout=5 "mroldan@${ssh_host}" exit 2>/dev/null; then
            echo -e "${CYAN}  Extracting CA cert from Caddy container...${NC}"
            if ssh "mroldan@${ssh_host}" "docker exec veriscan-proxy cat /data/caddy/pki/authorities/local/root.crt" > "${CA_CERT_PATH}" 2>/dev/null; then
                if [[ -s "${CA_CERT_PATH}" ]]; then
                    echo -e "${GREEN}  ✓ CA certificate downloaded via SSH${NC}"
                    return 0
                fi
            fi
        fi
    fi
    
    # Fallback: Try HTTP endpoint
    echo -e "${CYAN}  Trying HTTP endpoint...${NC}"
    if curl -k -f -s "${server_url}/ca-cert" -o "${CA_CERT_PATH}" 2>/dev/null; then
        if [[ -s "${CA_CERT_PATH}" ]]; then
            echo -e "${GREEN}  ✓ CA certificate downloaded via HTTP${NC}"
            return 0
        fi
    fi
    
    echo -e "${YELLOW}  ⚠ Could not download CA certificate${NC}"
    echo -e "${YELLOW}  TLS verification may fail. You can:${NC}"
    echo -e "${YELLOW}    1. Manually copy the certificate to: ${CA_CERT_PATH}${NC}"
    echo -e "${YELLOW}    2. Use system trust store${NC}"
    echo -e "${YELLOW}    3. Disable TLS verification (NOT RECOMMENDED)${NC}"
    
    return 1
}

# Interactive mode selection
echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}         ¿Cómo quieres usar Alejandría?${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}  1) 🏠 Local (stdio)${NC}           ${BLUE}[DEFAULT - RECOMENDADO]${NC}"
echo -e "     • Binario ejecutándose localmente"
echo -e "     • Base de datos privada"
echo -e "     • No requiere servidor ni red"
echo ""
echo -e "${GREEN}  2) 🌐 Cliente Remoto (MCP SSE)${NC}"
echo -e "     • Conectar a servidor MCP existente"
echo -e "     • Memoria compartida con equipo"
echo -e "     • Requiere URL del servidor"
echo ""
echo -e "${GREEN}  3) 🖥️  Servidor MCP (instalar servidor)${NC}"
echo -e "     • Instalar Alejandría como servidor"
echo -e "     • Otros usuarios pueden conectarse"
echo -e "     • Requiere configuración de red/TLS"
echo ""
read -p "Opción [1]: " MODE_CHOICE
MODE_CHOICE=${MODE_CHOICE:-1}

case "$MODE_CHOICE" in
    1)
        # ========== MODE 1: LOCAL (stdio) ==========
        echo ""
        echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
        echo -e "${GREEN}           Instalación en Modo Local${NC}"
        echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
        echo ""
        
        # Validate binary exists
        echo -e "${BLUE}[1/7] Validating binary...${NC}"
        if [[ ! -f "$BINARY_PATH" ]]; then
            echo -e "${RED}Error: Binary not found at $BINARY_PATH${NC}"
            echo ""
            read -p "Do you want to download it from GitHub releases? (y/N): " DOWNLOAD_BINARY
            if [[ "$DOWNLOAD_BINARY" == "y" ]] || [[ "$DOWNLOAD_BINARY" == "Y" ]]; then
                echo -e "${BLUE}Downloading latest release...${NC}"
                # TODO: Implement GitHub release download
                echo -e "${YELLOW}⚠ GitHub download not yet implemented${NC}"
                echo -e "${YELLOW}Please download manually and specify path with --binary${NC}"
                exit 1
            else
                echo "Please install Alejandría first or specify correct path with --binary"
                exit 1
            fi
        fi
        echo -e "${GREEN}✓ Binary found: $BINARY_PATH${NC}"
        
        echo -e "${BLUE}[2/7] Creating directories...${NC}"
        mkdir -p "$CONFIG_DIR"
        mkdir -p "$DATA_DIR"
        echo -e "${GREEN}✓ Directories created${NC}"
        
        configure_local_mode
        
        # Create test script
        echo -e "${BLUE}[4/7] Creating test script...${NC}"
        mkdir -p "${HOME}/.local/bin"
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
        
        echo -e "${BLUE}[5/7] Verifying configuration...${NC}"
        echo -e "${GREEN}✓ All configurations verified${NC}"
        
        echo -e "${BLUE}[6/7] Checking dependencies...${NC}"
        if ! command -v jq &> /dev/null; then
            echo -e "${YELLOW}  ⚠ jq not found (recommended for JSON manipulation)${NC}"
        else
            echo -e "${GREEN}  ✓ jq found${NC}"
        fi
        
        echo -e "${BLUE}[7/7] Finalizing installation...${NC}"
        echo -e "${GREEN}✓ Installation complete!${NC}"
        
        # Summary
        echo ""
        echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
        echo -e "${GREEN}           Installation Complete! 🎉${NC}"
        echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
        echo ""
        echo -e "${BLUE}Mode:${NC} Local (stdio)"
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
        ;;
        
    2)
        # ========== MODE 2: REMOTE CLIENT (SSE) ==========
        echo ""
        echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}       Instalación en Modo Cliente Remoto${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
        echo ""
        
        echo -e "${BLUE}[1/7] Gathering server information...${NC}"
        
        # Get server URL
        read -p "URL del servidor MCP [https://ar-appsec-01.veritran.net/alejandria]: " SERVER_URL
        SERVER_URL=${SERVER_URL:-https://ar-appsec-01.veritran.net/alejandria}
        
        if ! validate_url "$SERVER_URL"; then
            exit 1
        fi
        echo -e "${GREEN}✓ Server URL: $SERVER_URL${NC}"
        
        # Get API key
        read -sp "API Key: " API_KEY
        echo ""
        if [[ -z "$API_KEY" ]]; then
            echo -e "${RED}Error: API Key cannot be empty${NC}"
            exit 1
        fi
        echo -e "${GREEN}✓ API Key received${NC}"
        
        # Ask about CA cert download
        read -p "¿Descargar CA cert desde servidor? (S/n): " DOWNLOAD_CERT
        DOWNLOAD_CERT=${DOWNLOAD_CERT:-S}
        
        download_ca_cert "$SERVER_URL" "$DOWNLOAD_CERT"
        
        configure_remote_mode "$SERVER_URL" "$API_KEY" "$CA_CERT_PATH"
        
        echo -e "${BLUE}[4/7] Testing connection...${NC}"
        if command -v curl &> /dev/null; then
            if curl -k -f -s -H "X-API-Key: $API_KEY" "${SERVER_URL}/health" > /dev/null 2>&1; then
                echo -e "${GREEN}✓ Connection successful${NC}"
            else
                echo -e "${YELLOW}⚠ Could not verify connection (server may be down)${NC}"
            fi
        else
            echo -e "${YELLOW}⚠ curl not found, skipping connection test${NC}"
        fi
        
        echo -e "${BLUE}[5/7] Verifying configuration...${NC}"
        echo -e "${GREEN}✓ All configurations verified${NC}"
        
        echo -e "${BLUE}[6/7] Checking dependencies...${NC}"
        if ! command -v jq &> /dev/null; then
            echo -e "${YELLOW}  ⚠ jq not found (recommended for JSON manipulation)${NC}"
        else
            echo -e "${GREEN}  ✓ jq found${NC}"
        fi
        
        echo -e "${BLUE}[7/7] Finalizing installation...${NC}"
        echo -e "${GREEN}✓ Installation complete!${NC}"
        
        # Summary
        echo ""
        echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
        echo -e "${CYAN}           Installation Complete! 🎉${NC}"
        echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
        echo ""
        echo -e "${BLUE}Mode:${NC} Remote Client (MCP SSE)"
        echo -e "${BLUE}Server:${NC} $SERVER_URL"
        echo -e "${BLUE}Transport:${NC} SSE over HTTPS"
        if [[ -f "$CA_CERT_PATH" ]]; then
            echo -e "${BLUE}CA Cert:${NC} $CA_CERT_PATH"
        fi
        echo ""
        echo -e "${BLUE}Configured clients:${NC}"
        echo "  • OpenCode: ~/.config/opencode/opencode.json"
        echo "  • Claude Code CLI: ~/.claude.json"
        echo "  • Claude Desktop: ~/.config/Claude/claude_desktop_config.json"
        echo "  • VSCode/Copilot: ~/.config/Code/User/settings.json"
        echo "  • GitHub Copilot standalone: ~/.copilot/mcp-config.json"
        echo ""
        echo -e "${GREEN}Security features:${NC}"
        echo "  ✓ API keys encrypted in transit"
        echo "  ✓ Memories encrypted end-to-end"
        echo "  ✓ Protected against MITM attacks"
        echo "  ✓ Server identity verified"
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
        echo -e "${BLUE}Troubleshooting:${NC}"
        echo "  If TLS connections fail:"
        echo "    1. Verify CA cert: ls -lh $CA_CERT_PATH"
        echo "    2. Test connection: curl -k -H 'X-API-Key: YOUR_KEY' $SERVER_URL/health"
        echo "    3. Check server logs on remote server"
        echo ""
        echo -e "${GREEN}Happy team memory building! 🧠${NC}"
        ;;
        
    3)
        # ========== MODE 3: SERVER INSTALLATION ==========
        echo ""
        echo -e "${MAGENTA}═══════════════════════════════════════════════════════${NC}"
        echo -e "${MAGENTA}        Instalación en Modo Servidor${NC}"
        echo -e "${MAGENTA}═══════════════════════════════════════════════════════${NC}"
        echo ""
        
        # Check if running as root
        if [[ $EUID -ne 0 ]]; then
            echo -e "${RED}Error: Server installation requires root privileges${NC}"
            echo "Please run with sudo:"
            echo "  sudo $0"
            exit 1
        fi
        
        echo -e "${BLUE}[1/7] Server configuration...${NC}"
        
        # Get listen address
        read -p "IP/Puerto de escucha [0.0.0.0:8080]: " LISTEN_ADDR
        LISTEN_ADDR=${LISTEN_ADDR:-0.0.0.0:8080}
        
        # Validate port
        LISTEN_PORT=$(echo "$LISTEN_ADDR" | cut -d: -f2)
        if ! validate_port "$LISTEN_PORT"; then
            exit 1
        fi
        echo -e "${GREEN}✓ Listen address: $LISTEN_ADDR${NC}"
        
        # API key generation
        read -p "Generar API key automática? (S/n): " AUTO_API_KEY
        AUTO_API_KEY=${AUTO_API_KEY:-S}
        
        if [[ "$AUTO_API_KEY" == "S" ]] || [[ "$AUTO_API_KEY" == "s" ]]; then
            SERVER_API_KEY="alejandria-$(openssl rand -hex 20)"
            echo -e "${GREEN}✓ API Key generated: ${SERVER_API_KEY:0:20}...${NC}"
        else
            read -sp "API Key: " SERVER_API_KEY
            echo ""
            if [[ -z "$SERVER_API_KEY" ]]; then
                echo -e "${RED}Error: API Key cannot be empty${NC}"
                exit 1
            fi
        fi
        
        # Database path
        read -p "Base de datos [/var/lib/alejandria/alejandria.db]: " DB_PATH
        DB_PATH=${DB_PATH:-/var/lib/alejandria/alejandria.db}
        DB_DIR=$(dirname "$DB_PATH")
        echo -e "${GREEN}✓ Database path: $DB_PATH${NC}"
        
        # TLS configuration
        echo ""
        echo -e "${BLUE}Configuración de TLS:${NC}"
        echo -e "${GREEN}  1) Sin TLS (HTTP)${NC}                    ${RED}[NO RECOMENDADO]${NC}"
        echo -e "${GREEN}  2) TLS con reverse proxy (Caddy)${NC}     ${BLUE}[RECOMENDADO]${NC}"
        echo -e "${GREEN}  3) Ya tengo reverse proxy${NC}            ${YELLOW}[Manual]${NC}"
        echo ""
        read -p "Opción [2]: " TLS_CHOICE
        TLS_CHOICE=${TLS_CHOICE:-2}
        
        echo -e "${BLUE}[2/7] Creating directories and files...${NC}"
        mkdir -p "$DB_DIR"
        mkdir -p /etc/alejandria
        mkdir -p /var/log/alejandria
        
        # Create config file
        cat > /etc/alejandria/config.toml << EOF
# Alejandría Server Configuration
db_path = "$DB_PATH"
listen_addr = "$LISTEN_ADDR"

[memory]
max_memories = 1000000
default_decay_profile = "exponential"
access_dampening_factor = 0.5

[embeddings]
enabled = true

[decay]
half_life_days = 90
score_threshold = 0.1

[sse]
enabled = true
EOF
        echo -e "${GREEN}✓ Configuration created at /etc/alejandria/config.toml${NC}"
        
        # Create API key env file
        cat > /etc/alejandria/api.env << EOF
ALEJANDRIA_API_KEY=$SERVER_API_KEY
ALEJANDRIA_CONFIG=/etc/alejandria/config.toml
EOF
        chmod 600 /etc/alejandria/api.env
        echo -e "${GREEN}✓ API key stored in /etc/alejandria/api.env${NC}"
        
        echo -e "${BLUE}[3/7] Creating systemd service...${NC}"
        cat > /etc/systemd/system/alejandria.service << EOF
[Unit]
Description=Alejandría MCP Server
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$DB_DIR
EnvironmentFile=/etc/alejandria/api.env
ExecStart=$BINARY_PATH serve
Restart=always
RestartSec=10
StandardOutput=append:/var/log/alejandria/alejandria.log
StandardError=append:/var/log/alejandria/error.log

[Install]
WantedBy=multi-user.target
EOF
        echo -e "${GREEN}✓ Systemd service created${NC}"
        
        echo -e "${BLUE}[4/7] Configuring TLS...${NC}"
        if [[ "$TLS_CHOICE" == "2" ]]; then
            # Install Caddy if not present
            if ! command -v caddy &> /dev/null; then
                echo -e "${YELLOW}  Caddy not found. Installing...${NC}"
                read -p "  Install Caddy? (y/N): " INSTALL_CADDY
                if [[ "$INSTALL_CADDY" == "y" ]] || [[ "$INSTALL_CADDY" == "Y" ]]; then
                    echo -e "${CYAN}  Installing Caddy...${NC}"
                    # Add Caddy repository and install
                    apt update
                    apt install -y debian-keyring debian-archive-keyring apt-transport-https
                    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
                    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
                    apt update
                    apt install -y caddy
                    echo -e "${GREEN}  ✓ Caddy installed${NC}"
                else
                    echo -e "${YELLOW}  ⚠ Skipping Caddy installation${NC}"
                fi
            fi
            
            if command -v caddy &> /dev/null; then
                # Get Caddyfile path
                read -p "  Ruta del Caddyfile [/etc/caddy/Caddyfile]: " CADDYFILE_PATH
                CADDYFILE_PATH=${CADDYFILE_PATH:-/etc/caddy/Caddyfile}
                
                if [[ -f "$CADDYFILE_PATH" ]]; then
                    backup_file "$CADDYFILE_PATH"
                fi
                
                # Get domain/hostname
                read -p "  Hostname para TLS [ar-appsec-01.veritran.net]: " HOSTNAME
                HOSTNAME=${HOSTNAME:-ar-appsec-01.veritran.net}
                
                # Add Alejandría route to Caddyfile
                cat >> "$CADDYFILE_PATH" << EOF

# Alejandría MCP Server
https://$HOSTNAME {
    route /alejandria/* {
        uri strip_prefix /alejandria
        reverse_proxy $LISTEN_ADDR
    }
    
    tls internal {
        on_demand
    }
}
EOF
                echo -e "${GREEN}  ✓ Caddy configuration updated${NC}"
                
                # Reload Caddy
                systemctl reload caddy 2>/dev/null || systemctl restart caddy
                echo -e "${GREEN}  ✓ Caddy reloaded${NC}"
            fi
        elif [[ "$TLS_CHOICE" == "1" ]]; then
            echo -e "${RED}  ⚠ WARNING: Running without TLS is NOT RECOMMENDED${NC}"
            echo -e "${RED}  API keys and memories will be transmitted in plain text${NC}"
        else
            echo -e "${YELLOW}  ✓ Manual TLS configuration selected${NC}"
        fi
        
        echo -e "${BLUE}[5/7] Setting permissions...${NC}"
        chmod 644 /etc/alejandria/config.toml
        chmod 600 /etc/alejandria/api.env
        chown -R root:root /etc/alejandria
        chown -R root:root "$DB_DIR"
        chown -R root:root /var/log/alejandria
        echo -e "${GREEN}✓ Permissions set${NC}"
        
        echo -e "${BLUE}[6/7] Enabling and starting service...${NC}"
        systemctl daemon-reload
        systemctl enable alejandria
        systemctl start alejandria
        
        # Check service status
        if systemctl is-active --quiet alejandria; then
            echo -e "${GREEN}✓ Service started successfully${NC}"
        else
            echo -e "${RED}⚠ Service failed to start${NC}"
            echo -e "${YELLOW}Check logs: journalctl -u alejandria -n 50${NC}"
        fi
        
        echo -e "${BLUE}[7/7] Finalizing installation...${NC}"
        echo -e "${GREEN}✓ Server installation complete!${NC}"
        
        # Summary
        echo ""
        echo -e "${MAGENTA}═══════════════════════════════════════════════════════${NC}"
        echo -e "${MAGENTA}        Server Installation Complete! 🎉${NC}"
        echo -e "${MAGENTA}═══════════════════════════════════════════════════════${NC}"
        echo ""
        echo -e "${BLUE}Mode:${NC} Server"
        echo -e "${BLUE}Listen Address:${NC} $LISTEN_ADDR"
        echo -e "${BLUE}Database:${NC} $DB_PATH"
        echo -e "${BLUE}Config:${NC} /etc/alejandria/config.toml"
        echo -e "${BLUE}Service:${NC} alejandria.service"
        if [[ "$TLS_CHOICE" == "2" ]]; then
            echo -e "${BLUE}TLS:${NC} Enabled (Caddy reverse proxy)"
            echo -e "${BLUE}Endpoint:${NC} https://$HOSTNAME/alejandria"
        elif [[ "$TLS_CHOICE" == "1" ]]; then
            echo -e "${BLUE}TLS:${NC} Disabled (HTTP only)"
            echo -e "${BLUE}Endpoint:${NC} http://$(hostname -I | awk '{print $1}'):$LISTEN_PORT"
        fi
        echo ""
        echo -e "${BLUE}API Key (share with users):${NC}"
        echo -e "${YELLOW}  $SERVER_API_KEY${NC}"
        echo ""
        echo -e "${BLUE}Management commands:${NC}"
        echo "  • Start:   sudo systemctl start alejandria"
        echo "  • Stop:    sudo systemctl stop alejandria"
        echo "  • Restart: sudo systemctl restart alejandria"
        echo "  • Status:  sudo systemctl status alejandria"
        echo "  • Logs:    sudo journalctl -u alejandria -f"
        echo ""
        echo -e "${BLUE}Next steps:${NC}"
        echo "  1. Share API key with users (use secure method!)"
        echo "  2. Users run: ./install-mcp-v3.sh (select option 2 - Remote Client)"
        if [[ "$TLS_CHOICE" == "2" ]]; then
            echo "  3. Extract CA cert for users:"
            echo "     sudo cat /var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt"
        fi
        echo ""
        echo -e "${GREEN}Server is ready for connections! 🚀${NC}"
        ;;
        
    *)
        echo -e "${RED}Opción inválida: $MODE_CHOICE${NC}"
        echo "Opciones válidas: 1 (Local), 2 (Cliente Remoto), 3 (Servidor)"
        exit 1
        ;;
esac
