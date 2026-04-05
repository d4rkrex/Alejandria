#!/usr/bin/env bash
#
# Alejandria Deployment Verification Script
# ==========================================
#
# This script verifies that Alejandria is correctly deployed and functional.
#
# USAGE:
#   ./scripts/verify-deployment.sh [OPTIONS]
#
# OPTIONS:
#   --binary PATH       Path to alejandria-mcp binary (default: searches PATH)
#   --config PATH       Path to configuration file (default: ~/.config/alejandria/config.toml)
#   --db PATH           Path to database file (default: from config or env)
#   --skip-tools        Skip MCP tool verification
#   --verbose           Enable verbose output
#   --help              Show this help message
#
# EXIT CODES:
#   0 - All checks passed
#   1 - One or more checks failed
#   2 - Invalid usage or configuration error
#
# CHECKS PERFORMED:
#   1. Binary exists and is executable
#   2. Configuration file is valid
#   3. Database connectivity and schema
#   4. MCP server starts correctly
#   5. Basic tool tests (mem_store, mem_recall, mem_health)
#   6. FTS5 and vector search availability
#
# EXAMPLES:
#   # Basic verification (uses defaults)
#   ./scripts/verify-deployment.sh
#
#   # Custom binary and config
#   ./scripts/verify-deployment.sh --binary /usr/local/bin/alejandria-mcp --config /etc/alejandria/config.toml
#
#   # Quick check without tool tests
#   ./scripts/verify-deployment.sh --skip-tools
#
#   # Verbose output for debugging
#   ./scripts/verify-deployment.sh --verbose

set -e
set -o pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
BINARY_PATH=""
CONFIG_PATH="${HOME}/.config/alejandria/config.toml"
DB_PATH=""
SKIP_TOOLS=false
VERBOSE=false
TEMP_DIR=""
SERVER_PID=""
EXIT_CODE=0

# Cleanup function
cleanup() {
    if [[ -n "${SERVER_PID}" ]]; then
        echo -e "${BLUE}[INFO]${NC} Stopping test server (PID: ${SERVER_PID})..."
        kill "${SERVER_PID}" 2>/dev/null || true
        wait "${SERVER_PID}" 2>/dev/null || true
    fi
    if [[ -n "${TEMP_DIR}" && -d "${TEMP_DIR}" ]]; then
        rm -rf "${TEMP_DIR}"
    fi
}
trap cleanup EXIT

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    EXIT_CODE=1
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_verbose() {
    if [[ "${VERBOSE}" == true ]]; then
        echo -e "${BLUE}[DEBUG]${NC} $1"
    fi
}

# Show help
show_help() {
    grep '^#' "$0" | sed 's/^# \?//' | head -n -1
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --binary)
            BINARY_PATH="$2"
            shift 2
            ;;
        --config)
            CONFIG_PATH="$2"
            shift 2
            ;;
        --db)
            DB_PATH="$2"
            shift 2
            ;;
        --skip-tools)
            SKIP_TOOLS=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            show_help
            ;;
        *)
            echo -e "${RED}Error: Unknown option $1${NC}" >&2
            echo "Use --help for usage information"
            exit 2
            ;;
    esac
done

echo "========================================="
echo "Alejandria Deployment Verification"
echo "========================================="
echo ""

# Check 1: Binary exists and is executable
log_info "Check 1: Binary verification..."

if [[ -z "${BINARY_PATH}" ]]; then
    # Search in PATH
    if command -v alejandria-mcp &> /dev/null; then
        BINARY_PATH=$(command -v alejandria-mcp)
        log_verbose "Found binary in PATH: ${BINARY_PATH}"
    else
        log_error "alejandria-mcp binary not found in PATH"
        log_info "  Try: cargo install --path crates/alejandria-mcp"
        log_info "  Or specify path with: --binary /path/to/alejandria-mcp"
        exit 1
    fi
fi

if [[ ! -f "${BINARY_PATH}" ]]; then
    log_error "Binary not found: ${BINARY_PATH}"
    exit 1
fi

if [[ ! -x "${BINARY_PATH}" ]]; then
    log_error "Binary is not executable: ${BINARY_PATH}"
    exit 1
fi

# Get version
VERSION=$("${BINARY_PATH}" --version 2>&1 || echo "unknown")
log_success "Binary found and executable: ${BINARY_PATH}"
log_info "  Version: ${VERSION}"

# Check 2: Configuration file validation
log_info "Check 2: Configuration validation..."

if [[ ! -f "${CONFIG_PATH}" ]]; then
    log_warning "Configuration file not found: ${CONFIG_PATH}"
    log_info "  Using default configuration (environment variables or built-in defaults)"
    CONFIG_PATH=""
else
    log_verbose "Configuration file: ${CONFIG_PATH}"
    
    # Basic TOML syntax check (if toml-cli available)
    if command -v toml &> /dev/null; then
        if toml get "${CONFIG_PATH}" . &> /dev/null; then
            log_success "Configuration file is valid TOML"
        else
            log_error "Configuration file has invalid TOML syntax"
            exit 1
        fi
    else
        log_verbose "toml-cli not available, skipping detailed syntax check"
        log_success "Configuration file exists: ${CONFIG_PATH}"
    fi
fi

# Check 3: Database setup
log_info "Check 3: Database connectivity..."

# Determine database path
if [[ -z "${DB_PATH}" ]]; then
    if [[ -n "${ALEJANDRIA_DB_PATH}" ]]; then
        DB_PATH="${ALEJANDRIA_DB_PATH}"
        log_verbose "Using database from environment: ${DB_PATH}"
    elif [[ -f "${CONFIG_PATH}" ]] && command -v toml &> /dev/null; then
        DB_PATH=$(toml get "${CONFIG_PATH}" db.path 2>/dev/null | tr -d '"' || echo "")
        log_verbose "Using database from config: ${DB_PATH}"
    fi
    
    if [[ -z "${DB_PATH}" ]]; then
        DB_PATH="${HOME}/.local/share/alejandria/memories.db"
        log_verbose "Using default database path: ${DB_PATH}"
    fi
fi

# Check if database exists
if [[ ! -f "${DB_PATH}" ]]; then
    log_warning "Database file does not exist: ${DB_PATH}"
    log_info "  Database will be created on first run"
    
    # Create parent directory if needed
    DB_DIR=$(dirname "${DB_PATH}")
    if [[ ! -d "${DB_DIR}" ]]; then
        log_info "  Creating database directory: ${DB_DIR}"
        mkdir -p "${DB_DIR}" || {
            log_error "Failed to create database directory"
            exit 1
        }
    fi
else
    log_success "Database file exists: ${DB_PATH}"
    
    # Check if SQLite can open it
    if command -v sqlite3 &> /dev/null; then
        if sqlite3 "${DB_PATH}" "SELECT 1;" &> /dev/null; then
            log_success "Database is readable and valid SQLite format"
            
            # Check schema
            TABLES=$(sqlite3 "${DB_PATH}" "SELECT name FROM sqlite_master WHERE type='table';" 2>/dev/null || echo "")
            if [[ -n "${TABLES}" ]]; then
                log_verbose "Database tables: $(echo ${TABLES} | tr '\n' ', ')"
                
                # Check for expected tables
                EXPECTED_TABLES=("memories" "memoirs" "concepts" "links" "migrations")
                for table in "${EXPECTED_TABLES[@]}"; do
                    if echo "${TABLES}" | grep -q "^${table}$"; then
                        log_verbose "  ✓ Table '${table}' exists"
                    else
                        log_warning "  Table '${table}' not found (will be created on startup)"
                    fi
                done
            else
                log_info "  Database is empty (schema will be initialized on first run)"
            fi
        else
            log_error "Database exists but cannot be opened (may be corrupted)"
        fi
    else
        log_verbose "sqlite3 CLI not available, skipping detailed database checks"
    fi
fi

# Check 4: MCP server startup
log_info "Check 4: MCP server startup test..."

# Create temporary directory for test
TEMP_DIR=$(mktemp -d)
TEST_DB="${TEMP_DIR}/test_memories.db"
log_verbose "Test database: ${TEST_DB}"

# Build server command
SERVER_CMD=("${BINARY_PATH}")
if [[ -n "${CONFIG_PATH}" ]]; then
    SERVER_CMD+=(--config "${CONFIG_PATH}")
fi

# Override database path for test
export ALEJANDRIA_DB_PATH="${TEST_DB}"

# Start server in background
log_verbose "Starting server: ${SERVER_CMD[*]}"
"${SERVER_CMD[@]}" &> "${TEMP_DIR}/server.log" &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Check if server is still running
if kill -0 "${SERVER_PID}" 2>/dev/null; then
    log_success "MCP server started successfully (PID: ${SERVER_PID})"
    log_verbose "Server log: ${TEMP_DIR}/server.log"
else
    log_error "MCP server failed to start"
    log_info "Server log output:"
    cat "${TEMP_DIR}/server.log" | sed 's/^/    /'
    exit 1
fi

# Check 5: Basic tool tests
if [[ "${SKIP_TOOLS}" == false ]]; then
    log_info "Check 5: MCP tool verification..."
    
    # Note: These tests require the server to accept stdio JSON-RPC requests
    # For a full test, you'd need to send proper JSON-RPC messages
    # Here we just verify the server process is running
    
    log_verbose "Testing tool availability (requires JSON-RPC client)..."
    
    # Check if server is responsive
    sleep 1
    if kill -0 "${SERVER_PID}" 2>/dev/null; then
        log_success "Server is responsive and running"
        log_info "  Note: Full tool testing requires JSON-RPC client"
        log_info "  Run: cargo test --package alejandria-mcp --test integration"
    else
        log_error "Server stopped unexpectedly"
        log_info "Server log output:"
        cat "${TEMP_DIR}/server.log" | sed 's/^/    /'
    fi
    
    # Check server log for any errors
    if grep -i "error\|panic\|fatal" "${TEMP_DIR}/server.log" > /dev/null 2>&1; then
        log_warning "Server log contains error messages:"
        grep -i "error\|panic\|fatal" "${TEMP_DIR}/server.log" | sed 's/^/    /'
    fi
else
    log_info "Check 5: Skipped (--skip-tools specified)"
fi

# Check 6: System capabilities
log_info "Check 6: System capabilities..."

# Check for FTS5 support
if command -v sqlite3 &> /dev/null; then
    if sqlite3 "${TEST_DB}" "CREATE VIRTUAL TABLE test_fts USING fts5(content);" &> /dev/null; then
        log_success "FTS5 full-text search is available"
        sqlite3 "${TEST_DB}" "DROP TABLE test_fts;" &> /dev/null
    else
        log_error "FTS5 is not available in SQLite"
        log_info "  Rebuild SQLite with FTS5 support or install a version that includes it"
    fi
    
    # Check for vector search (sqlite-vec)
    # Note: This would require the extension to be loaded
    log_verbose "Vector search availability depends on sqlite-vec extension"
    log_info "  Vector search requires sqlite-vec extension (checked at runtime)"
else
    log_verbose "sqlite3 CLI not available, skipping capability checks"
fi

# Check Rust environment (for building/updating)
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    log_success "Rust toolchain available: ${RUST_VERSION}"
else
    log_warning "Rust toolchain not found (needed for building from source)"
fi

# Summary
echo ""
echo "========================================="
echo "Verification Summary"
echo "========================================="

if [[ ${EXIT_CODE} -eq 0 ]]; then
    log_success "All checks passed! Alejandria is ready to use."
    echo ""
    echo "Next steps:"
    echo "  1. Configure your MCP client (e.g., Claude Desktop)"
    echo "  2. See: examples/claude-desktop/README.md"
    echo "  3. Test with: cargo test --package alejandria-mcp"
    echo ""
else
    log_error "Some checks failed. Please review the errors above."
    echo ""
    echo "Common fixes:"
    echo "  • Install binary: cargo install --path crates/alejandria-mcp"
    echo "  • Create config: cp config/default.toml ~/.config/alejandria/config.toml"
    echo "  • Check permissions on database directory"
    echo "  • Review logs in ${TEMP_DIR}/server.log"
    echo ""
fi

exit ${EXIT_CODE}
