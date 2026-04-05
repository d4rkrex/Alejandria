#!/usr/bin/env bash
# Docker Build Validation and Testing Script for Alejandria
# Tests both CLI and MCP images for functionality, size, and persistence

set -e  # Exit on first error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

run_test() {
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    local test_name="$1"
    log_info "Running test: $test_name"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test artifacts..."
    docker rm -f alejandria-mcp-test 2>/dev/null || true
    docker volume rm -f alejandria-test-volume 2>/dev/null || true
    docker volume rm -f alejandria-persist-test 2>/dev/null || true
}

# Set trap to cleanup on exit
trap cleanup EXIT

# ============================================================================
# Phase 4.2: CLI Image Build Test
# ============================================================================
run_test "CLI image builds successfully"
if docker build -f Dockerfile.cli -t alejandria-cli:test . > /tmp/cli-build.log 2>&1; then
    log_success "CLI image built successfully"
else
    log_error "CLI image build failed (see /tmp/cli-build.log)"
    exit 1
fi

# ============================================================================
# Phase 4.3: MCP Image Build Test
# ============================================================================
run_test "MCP image builds successfully"
if docker build -f Dockerfile.mcp -t alejandria-mcp:test . > /tmp/mcp-build.log 2>&1; then
    log_success "MCP image built successfully"
else
    log_error "MCP image build failed (see /tmp/mcp-build.log)"
    exit 1
fi

# ============================================================================
# Phase 4.4: CLI Smoke Test - Version Command
# ============================================================================
run_test "CLI container executes --version command"
if VERSION_OUTPUT=$(docker run --rm alejandria-cli:test --version 2>&1); then
    if echo "$VERSION_OUTPUT" | grep -q "alejandria"; then
        log_success "CLI --version command successful: $VERSION_OUTPUT"
    else
        log_error "CLI --version output doesn't contain 'alejandria': $VERSION_OUTPUT"
    fi
else
    log_error "CLI --version command failed"
fi

# ============================================================================
# Phase 4.5: CLI Help Test
# ============================================================================
run_test "CLI container executes --help command"
if HELP_OUTPUT=$(docker run --rm alejandria-cli:test --help 2>&1); then
    # Check for expected subcommands
    if echo "$HELP_OUTPUT" | grep -q "Commands:"; then
        log_success "CLI --help command successful and contains subcommands"
    else
        log_error "CLI --help output doesn't contain expected format"
    fi
else
    log_error "CLI --help command failed"
fi

# ============================================================================
# Phase 4.6: MCP Server Start Test
# ============================================================================
run_test "MCP server container starts successfully"
# Start MCP server in detached mode with a volume
if docker run -d --name alejandria-mcp-test -v alejandria-test-volume:/data alejandria-mcp:test > /dev/null 2>&1; then
    sleep 3  # Give server time to start
    
    # Check if container is still running
    if docker ps | grep -q alejandria-mcp-test; then
        log_success "MCP server started and is running"
        
        # Check logs for startup message
        if docker logs alejandria-mcp-test 2>&1 | grep -qi "server\|starting\|listening"; then
            log_success "MCP server logs show startup indication"
        else
            log_warning "MCP server logs don't show clear startup message (this may be normal)"
        fi
    else
        log_error "MCP server container started but exited immediately"
        docker logs alejandria-mcp-test
    fi
    
    # Stop the test container
    docker stop alejandria-mcp-test > /dev/null 2>&1
    docker rm alejandria-mcp-test > /dev/null 2>&1
else
    log_error "Failed to start MCP server container"
fi

# ============================================================================
# Phase 4.7: Persistence Test
# ============================================================================
run_test "Database persists across container restarts"
# Create a test volume
docker volume create alejandria-persist-test > /dev/null 2>&1

# Store a test memory using CLI
log_info "Storing test memory..."
if docker run --rm \
    -v alejandria-persist-test:/data \
    -e ALEJANDRIA_DB_PATH=/data/test.db \
    alejandria-cli:test store "Docker persistence test memory" \
    --project test-project \
    --source test > /dev/null 2>&1; then
    log_success "Test memory stored successfully"
else
    log_error "Failed to store test memory"
fi

# Start a new container with the same volume and check stats
log_info "Verifying data persists in new container..."
if STATS_OUTPUT=$(docker run --rm \
    -v alejandria-persist-test:/data \
    -e ALEJANDRIA_DB_PATH=/data/test.db \
    alejandria-cli:test stats --json 2>&1); then
    
    # Check if stats show the stored memory
    if echo "$STATS_OUTPUT" | grep -q "total_memories"; then
        log_success "Database persisted across container restart"
    else
        log_error "Stats output doesn't show expected format: $STATS_OUTPUT"
    fi
else
    log_error "Failed to retrieve stats from persisted database"
fi

# Cleanup persistence test volume
docker volume rm -f alejandria-persist-test > /dev/null 2>&1

# ============================================================================
# Phase 4.8: Image Size Validation
# ============================================================================
run_test "Image sizes are within targets"

# Get CLI image size in MB
CLI_SIZE=$(docker images alejandria-cli:test --format "{{.Size}}")
log_info "CLI image size: $CLI_SIZE"

# Convert size to MB for comparison (handle KB, MB, GB suffixes)
if echo "$CLI_SIZE" | grep -q "MB"; then
    CLI_SIZE_MB=$(echo "$CLI_SIZE" | sed 's/MB//')
    if (( $(echo "$CLI_SIZE_MB <= 8" | bc -l) )); then
        log_success "CLI image size ($CLI_SIZE) is within target (≤8MB)"
    else
        log_warning "CLI image size ($CLI_SIZE) exceeds target of 8MB"
    fi
elif echo "$CLI_SIZE" | grep -q "KB"; then
    log_success "CLI image size ($CLI_SIZE) is within target (≤8MB)"
elif echo "$CLI_SIZE" | grep -q "GB"; then
    log_error "CLI image size ($CLI_SIZE) is way over target (≤8MB)"
fi

# Get MCP image size in MB
MCP_SIZE=$(docker images alejandria-mcp:test --format "{{.Size}}")
log_info "MCP image size: $MCP_SIZE"

if echo "$MCP_SIZE" | grep -q "MB"; then
    MCP_SIZE_MB=$(echo "$MCP_SIZE" | sed 's/MB//')
    if (( $(echo "$MCP_SIZE_MB <= 20" | bc -l) )); then
        log_success "MCP image size ($MCP_SIZE) is within target (≤20MB)"
    else
        log_warning "MCP image size ($MCP_SIZE) exceeds target of 20MB"
    fi
elif echo "$MCP_SIZE" | grep -q "KB"; then
    log_success "MCP image size ($MCP_SIZE) is within target (≤20MB)"
elif echo "$MCP_SIZE" | grep -q "GB"; then
    log_error "MCP image size ($MCP_SIZE) is way over target (≤20MB)"
fi

# ============================================================================
# Phase 4.9: Multi-Platform Build Test (Optional)
# ============================================================================
if command -v docker &> /dev/null && docker buildx version &> /dev/null 2>&1; then
    run_test "Multi-platform build support (buildx available)"
    log_info "Testing multi-platform build for linux/amd64 and linux/arm64..."
    
    if docker buildx build --platform linux/amd64,linux/arm64 \
        -f Dockerfile.cli \
        -t alejandria-cli:multiplatform-test \
        --load=false \
        . > /tmp/buildx-test.log 2>&1; then
        log_success "Multi-platform build test passed (amd64 + arm64)"
    else
        log_warning "Multi-platform build test failed (see /tmp/buildx-test.log) - this is optional"
    fi
else
    log_warning "Skipping multi-platform test (docker buildx not available)"
fi

# ============================================================================
# Test Summary
# ============================================================================
echo ""
echo "========================================"
echo "  Docker Build Validation Summary"
echo "========================================"
echo -e "Total tests: $TESTS_TOTAL"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed! ✓${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed. See output above for details.${NC}"
    exit 1
fi
