#!/usr/bin/env bash
#
# Integration test script for Alejandria MCP client examples
# Tests all four language implementations against a local MCP server
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "============================================"
echo "Alejandria MCP Client Integration Tests"
echo "============================================"
echo

# 1. Build Alejandria MCP server
echo "📦 Building Alejandria MCP server..."
cargo build --release --bin alejandria
echo "✓ Server binary built successfully"
echo

# 2. Create temporary test database
TEST_DB=$(mktemp -d)/test_memories.db
export ALEJANDRIA_BIN="$PWD/target/release/alejandria"
export ALEJANDRIA_DB="$TEST_DB"

echo "🗄️  Using test database: $TEST_DB"
echo "🚀 Server binary: $ALEJANDRIA_BIN"
echo

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to run tests
run_test() {
    local lang=$1
    local test_cmd=$2
    local validation=$3
    
    echo "Testing $lang client..."
    if eval "$test_cmd" 2>&1 | grep -q "$validation"; then
        echo -e "${GREEN}✓${NC} $lang tests passed"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗${NC} $lang tests failed"
        ((TESTS_FAILED++))
    fi
    echo
}

# 3. Test Python client
if command -v python3 &>/dev/null; then
    cd examples/python
    echo "📝 Installing Python dependencies..."
    python3 -m pip install -q -r requirements.txt
    run_test "Python" "python3 example_memory.py" "Stored memory with ID"
    cd ../..
else
    echo -e "${YELLOW}⚠${NC}  Python 3 not found, skipping Python tests"
    echo
fi

# 4. Test Node.js client
if command -v node &>/dev/null; then
    cd examples/nodejs
    echo "📝 Installing Node.js dependencies..."
    npm install --silent
    npm run build --silent
    run_test "Node.js" "node dist/exampleMemory.js" "Stored memory with ID"
    cd ../..
else
    echo -e "${YELLOW}⚠${NC}  Node.js not found, skipping Node.js tests"
    echo
fi

# 5. Test Go client
if command -v go &>/dev/null; then
    cd examples/go
    run_test "Go" "go run example_memory.go" "Stored memory with ID"
    cd ../..
else
    echo -e "${YELLOW}⚠${NC}  Go not found, skipping Go tests"
    echo
fi

# 6. Test Rust client
if command -v cargo &>/dev/null; then
    cd examples/rust
    run_test "Rust" "cargo run --quiet --bin example_memory" "Stored memory with ID"
    cd ../..
else
    echo -e "${YELLOW}⚠${NC}  Rust not found, skipping Rust tests"
    echo
fi

# 7. Cleanup
echo "🧹 Cleaning up test database..."
rm -rf "$(dirname "$TEST_DB")"

# Summary
echo "============================================"
echo "Test Summary"
echo "============================================"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}✗ Some tests failed${NC}"
    exit 1
fi
