#!/bin/bash
#
# Alejandría API Key Generator v1.0
# Genera API keys seguras para usuarios que se conectan al servidor SSE
#
# Uso:
#   ./scripts/generate-api-key.sh <username> [--expires <days>]
#
# Ejemplo:
#   ./scripts/generate-api-key.sh juan.perez --expires 90
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Default values
EXPIRES_DAYS=""
KEY_FILE="${HOME}/.alejandria/generated-keys.txt"

# Check for help first
if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
    echo "Usage: $0 <username> [--expires <days>]"
    echo ""
    echo "Arguments:"
    echo "  username           Username or identifier (e.g., juan.perez, mobile-app)"
    echo ""
    echo "Options:"
    echo "  --expires DAYS     Expiration in days (optional, default: no expiration)"
    echo "  --help, -h         Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 juan.perez"
    echo "  $0 maria.garcia --expires 90"
    echo "  $0 mobile-app --expires 30"
    exit 0
fi

# Parse arguments
USERNAME="$1"
shift || true

while [[ $# -gt 0 ]]; do
    case $1 in
        --expires)
            EXPIRES_DAYS="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Validate username
if [[ -z "$USERNAME" ]]; then
    echo -e "${RED}Error: Username required${NC}"
    echo "Usage: $0 <username> [--expires <days>]"
    echo "Example: $0 juan.perez --expires 90"
    exit 1
fi

# Validate username format (alphanumeric, dots, hyphens, underscores)
if ! [[ "$USERNAME" =~ ^[a-zA-Z0-9._-]+$ ]]; then
    echo -e "${RED}Error: Username can only contain letters, numbers, dots, hyphens, and underscores${NC}"
    exit 1
fi

# Generate secure random API key
echo -e "${BLUE}Generating API key for user: ${BOLD}${USERNAME}${NC}"
API_KEY="alejandria-$(openssl rand -hex 20)"

# Calculate expiration date if specified
EXPIRES_AT=""
EXPIRES_DISPLAY="Never"
if [[ -n "$EXPIRES_DAYS" ]]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        EXPIRES_AT=$(date -v+"${EXPIRES_DAYS}d" +%Y-%m-%d)
    else
        # Linux
        EXPIRES_AT=$(date -d "+${EXPIRES_DAYS} days" +%Y-%m-%d)
    fi
    EXPIRES_DISPLAY="$EXPIRES_AT ($EXPIRES_DAYS days)"
fi

# Create key registry directory
mkdir -p "$(dirname "$KEY_FILE")"

# Append to key registry
TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
echo "$TIMESTAMP | $USERNAME | $API_KEY | Expires: $EXPIRES_DISPLAY | Status: ACTIVE" >> "$KEY_FILE"

# Display results
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}${BOLD}              API Key Generated Successfully! 🔑${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}User:${NC}       ${BOLD}${USERNAME}${NC}"
echo -e "${CYAN}API Key:${NC}    ${BOLD}${API_KEY}${NC}"
echo -e "${CYAN}Created:${NC}    ${TIMESTAMP}"
echo -e "${CYAN}Expires:${NC}    ${EXPIRES_DISPLAY}"
echo ""
echo -e "${YELLOW}${BOLD}⚠️  SECURITY WARNINGS:${NC}"
echo ""
echo -e "${YELLOW}1. This key grants FULL ACCESS to Alejandría${NC}"
echo -e "   • Can read ALL memories in the database"
echo -e "   • Can create, update, and delete memories"
echo -e "   • Treat this as a PASSWORD"
echo ""
echo -e "${YELLOW}2. Share this key SECURELY with ${USERNAME}:${NC}"
echo -e "   ✓ Use 1Password, Bitwarden, or similar"
echo -e "   ✓ Use Signal, encrypted chat, or in-person"
echo -e "   ✗ DO NOT send via email or Slack"
echo -e "   ✗ DO NOT commit to Git"
echo ""
echo -e "${YELLOW}3. Key registry saved to:${NC}"
echo -e "   ${KEY_FILE}"
echo -e "   (Backup this file securely)"
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}${BOLD}              How to Use This Key${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}For the user (${USERNAME}):${NC}"
echo ""
echo "1. Save the API key to your MCP client config:"
echo ""
echo -e "${BOLD}   OpenCode (~/.config/opencode/opencode.json):${NC}"
echo '   {'
echo '     "mcp": {'
echo '       "alejandria": {'
echo '         "url": "https://ar-appsec-01.veritran.net/alejandria",'
echo "         \"apiKey\": \"${API_KEY}\","
echo '         "transport": "sse",'
echo '         "tlsCert": "~/.alejandria/ca-cert.pem"'
echo '       }'
echo '     }'
echo '   }'
echo ""
echo -e "${BOLD}   Claude Code CLI (~/.claude.json):${NC}"
echo '   {'
echo '     "mcpServers": {'
echo '       "alejandria": {'
echo '         "url": "https://ar-appsec-01.veritran.net/alejandria",'
echo "         \"apiKey\": \"${API_KEY}\","
echo '         "transport": "sse"'
echo '       }'
echo '     }'
echo '   }'
echo ""
echo "2. Download CA certificate:"
echo "   mkdir -p ~/.alejandria"
echo "   curl -o ~/.alejandria/ca-cert.pem \\"
echo "        https://ar-appsec-01.veritran.net/alejandria/ca-cert"
echo ""
echo "3. Restart MCP clients (OpenCode, Claude Desktop, etc.)"
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}${BOLD}              Admin Tasks${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}CURRENT LIMITATION:${NC}"
echo -e "  Alejandría v1.x supports ONLY ONE active API key at a time."
echo -e "  To activate this key, update the server:"
echo ""
echo -e "${BOLD}  1. SSH to server:${NC}"
echo "     ssh mroldan@ar-appsec-01.veritran.net"
echo ""
echo -e "${BOLD}  2. Update API key:${NC}"
echo "     sudo sed -i 's/^ALEJANDRIA_API_KEY=.*/ALEJANDRIA_API_KEY=${API_KEY}/' \\"
echo "          /etc/systemd/system/alejandria.service"
echo ""
echo -e "${BOLD}  3. Reload and restart:${NC}"
echo "     sudo systemctl daemon-reload"
echo "     sudo systemctl restart alejandria"
echo ""
echo -e "${BOLD}  4. Verify:${NC}"
echo "     curl -H 'X-API-Key: ${API_KEY}' \\"
echo "          https://ar-appsec-01.veritran.net/alejandria/health"
echo ""
echo -e "${YELLOW}⚠️  NOTE: Changing the API key will REVOKE all previous keys!${NC}"
echo -e "         Notify all users before rotating keys."
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}${BOLD}              Key Management Commands${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}List all generated keys:${NC}"
echo "  cat $KEY_FILE"
echo ""
echo -e "${CYAN}Search for user's key:${NC}"
echo "  grep '$USERNAME' $KEY_FILE"
echo ""
echo -e "${CYAN}Mark key as revoked:${NC}"
echo "  sed -i 's/| $USERNAME |.*ACTIVE/| $USERNAME |.*REVOKED/' $KEY_FILE"
echo ""
echo -e "${CYAN}Generate new key (rotate):${NC}"
echo "  $0 $USERNAME --expires 90"
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}${BOLD}              Future: Multi-Key Support (v2.0)${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo "Alejandría v2.0 will support multiple simultaneous API keys with:"
echo "  ✓ Per-user key management"
echo "  ✓ Individual key revocation"
echo "  ✓ Automatic expiration"
echo "  ✓ Usage auditing"
echo "  ✓ TUI admin console"
echo ""
echo "Planned for Sprint 1 (P0-2: API Key Rotation & Management)"
echo ""
echo -e "${GREEN}Happy memory sharing! 🧠${NC}"
echo ""
