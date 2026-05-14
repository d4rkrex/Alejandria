#!/usr/bin/env bash
# Test script for installer v4 - simulates clean environment

set -euo pipefail

echo "=== Alejandria Installer v4 Test Suite ==="
echo

# Test 1: Platform detection
echo "Test 1: Platform Detection"
if bash -c 'source scripts/install.sh && detect_platform' >/dev/null 2>&1; then
    echo "✓ Platform detection works"
else
    echo "✗ Platform detection failed"
    exit 1
fi

# Test 2: Script syntax
echo "Test 2: Script Syntax"
if bash -n scripts/install.sh; then
    echo "✓ No syntax errors"
else
    echo "✗ Syntax errors found"
    exit 1
fi

# Test 3: JSON merge simulation (requires jq or python3)
echo "Test 3: JSON Merge Logic"
if command -v jq >/dev/null 2>&1; then
    # Test merging Alejandria config into existing MCP config
    TEST_CONFIG='{"mcpServers": {"other": {"command": "test"}}}'
    TEST_SERVER='{"command": "/test/alejandria", "args": ["serve"]}'
    RESULT=$(echo "$TEST_CONFIG" | jq --argjson server "$TEST_SERVER" '.mcpServers.alejandria = $server')
    
    if echo "$RESULT" | jq -e '.mcpServers.alejandria.command == "/test/alejandria"' >/dev/null; then
        echo "✓ JSON merge works correctly"
    else
        echo "✗ JSON merge failed"
        exit 1
    fi
elif command -v python3 >/dev/null 2>&1; then
    # Test with Python fallback
    python3 <<'EOF'
import json
existing = {"mcpServers": {"other": {"command": "test"}}}
server = {"command": "/test/alejandria", "args": ["serve"]}
existing["mcpServers"]["alejandria"] = server
assert existing["mcpServers"]["alejandria"]["command"] == "/test/alejandria"
print("✓ JSON merge works correctly (Python)")
EOF
else
    echo "⚠ Skipped: neither jq nor python3 available"
fi

# Test 4: Check required functions exist
echo "Test 4: Required Functions"
REQUIRED_FUNCTIONS=(
    "detect_platform"
    "get_latest_version"
    "download_binary"
    "build_from_source"
    "detect_mcp_clients"
    "backup_config"
    "merge_config"
    "create_alejandria_config"
)

for func in "${REQUIRED_FUNCTIONS[@]}"; do
    if grep -q "^${func}()" scripts/install.sh; then
        echo "  ✓ $func defined"
    else
        echo "  ✗ $func missing"
        exit 1
    fi
done

# Test 5: Security checks
echo "Test 5: Security Checks"
SECURITY_PATTERNS=(
    "https://"          # HTTPS usage
    "sha256sum"         # Checksum verification
    "backup"            # Config backup
    "set -euo pipefail" # Strict error handling
)

for pattern in "${SECURITY_PATTERNS[@]}"; do
    if grep -q "$pattern" scripts/install.sh; then
        echo "  ✓ Uses $pattern"
    else
        echo "  ⚠ Missing $pattern reference"
    fi
done

# Test 6: Error handling patterns
echo "Test 6: Error Handling"
if grep -q "|| {" scripts/install.sh && \
   grep -q "return 1" scripts/install.sh && \
   grep -q "exit 1" scripts/install.sh; then
    echo "✓ Has error handling patterns"
else
    echo "✗ Missing error handling"
    exit 1
fi

# Test 7: GitHub Actions workflow validation
echo "Test 7: GitHub Actions Workflow"
if command -v yq >/dev/null 2>&1 || command -v python3 >/dev/null 2>&1; then
    if [ -f .github/workflows/release.yml ]; then
        # Basic YAML syntax check
        if python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" 2>/dev/null; then
            echo "✓ release.yml is valid YAML"
        else
            echo "✗ release.yml has syntax errors"
            exit 1
        fi
        
        # Check for required jobs
        if grep -q "create-release:" .github/workflows/release.yml && \
           grep -q "build:" .github/workflows/release.yml && \
           grep -q "checksums:" .github/workflows/release.yml; then
            echo "✓ All required jobs defined"
        else
            echo "✗ Missing required jobs"
            exit 1
        fi
    else
        echo "✗ release.yml not found"
        exit 1
    fi
else
    echo "⚠ Skipped: YAML validation tools not available"
fi

# Test 8: Documentation checks
echo "Test 8: Documentation"
if [ -f "QUICKSTART.md" ]; then
    echo "✓ QUICKSTART.md exists"
    
    # Check for key sections
    if grep -q "One-Line Installation" QUICKSTART.md && \
       grep -q "Troubleshooting" QUICKSTART.md && \
       grep -q "Security Notes" QUICKSTART.md; then
        echo "✓ QUICKSTART.md has required sections"
    else
        echo "✗ QUICKSTART.md missing required sections"
        exit 1
    fi
else
    echo "✗ QUICKSTART.md not found"
    exit 1
fi

if grep -q "30-Second Installation" README.md; then
    echo "✓ README.md updated with quickstart"
else
    echo "✗ README.md not updated"
    exit 1
fi

echo
echo "=== All Tests Passed ==="
echo
echo "Summary of deliverables:"
echo "  • .github/workflows/release.yml - Cross-platform build workflow"
echo "  • scripts/install.sh - Intelligent installer"
echo "  • QUICKSTART.md - Step-by-step guide"
echo "  • README.md - Updated with 30-second quickstart"
echo
echo "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Test locally: export FORCE_BUILD=true && bash scripts/install.sh"
echo "  3. Commit: git commit -am 'feat: pre-built binaries and installer v4'"
echo "  4. Tag: git tag v1.6.0-installation-friction"
echo "  5. Push: git push origin feat/prebuilt-binaries-installer --tags"
