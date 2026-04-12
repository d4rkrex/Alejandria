#!/bin/bash
# P0-3 CORS Testing Commands
# Run these tests to verify CORS implementation

set -e

echo "=== P0-3 CORS Implementation Tests ==="
echo ""

# Test 1: Reject wildcard in production
echo "Test 1: Reject wildcard in production"
echo "--------------------------------------"
export ALEJANDRIA_ENV=production
export ALEJANDRIA_CORS_ENABLED=true
export ALEJANDRIA_CORS_ORIGINS="*"
export ALEJANDRIA_API_KEY="test-key"

echo "$ cargo run --features http-transport -- serve --http"
echo "Expected: Server refuses to start with error about wildcard"
echo ""
# Uncomment to run:
# cargo run --features http-transport -- serve --http 2>&1 | grep -i "wildcard"

# Test 2: Reject HTTP origins in production
echo "Test 2: Reject HTTP origins in production"
echo "------------------------------------------"
export ALEJANDRIA_CORS_ORIGINS="http://example.com"

echo "$ cargo run --features http-transport -- serve --http"
echo "Expected: Server refuses to start with error about HTTPS"
echo ""
# Uncomment to run:
# cargo run --features http-transport -- serve --http 2>&1 | grep -i "https"

# Test 3: Accept valid HTTPS origins
echo "Test 3: Accept valid HTTPS origins"
echo "-----------------------------------"
export ALEJANDRIA_CORS_ORIGINS="https://ar-appsec-01.veritran.net,https://admin.veritran.net"

echo "$ cargo run --features http-transport -- serve --http"
echo "Expected: Server starts successfully with CORS enabled"
echo ""
# Uncomment to run:
# cargo run --features http-transport -- serve --http &
# SERVER_PID=$!
# sleep 3
# kill $SERVER_PID

# Test 4: Development mode allows all origins
echo "Test 4: Development mode allows all origins"
echo "-------------------------------------------"
export ALEJANDRIA_ENV=development
export ALEJANDRIA_CORS_ORIGINS=""

echo "$ cargo run --features http-transport -- serve --http"
echo "Expected: Server starts with 'allow all origins (DEVELOPMENT MODE)'"
echo ""

# Test 5: CORS preflight request
echo "Test 5: CORS preflight request (requires running server)"
echo "--------------------------------------------------------"
echo "$ curl -i -X OPTIONS http://localhost:8080/health \\"
echo "    -H 'Origin: https://ar-appsec-01.veritran.net' \\"
echo "    -H 'Access-Control-Request-Method: POST'"
echo ""
echo "Expected headers:"
echo "  Access-Control-Allow-Origin: https://ar-appsec-01.veritran.net"
echo "  Access-Control-Allow-Methods: GET, POST, OPTIONS"
echo "  Access-Control-Allow-Credentials: true"
echo "  Access-Control-Max-Age: 3600"
echo ""

# Test 6: Unit tests
echo "Test 6: Unit tests"
echo "------------------"
echo "$ cargo test --package alejandria-mcp --lib transport::http::tests::test_cors --features http-transport"
echo ""
echo "Expected: All 6 CORS tests pass"
echo "  - test_cors_validation_rejects_wildcard_in_production"
echo "  - test_cors_validation_requires_origins_in_production"
echo "  - test_cors_validation_requires_https_in_production"
echo "  - test_cors_validation_allows_localhost_http"
echo "  - test_cors_validation_passes_with_valid_https_origins"
echo "  - test_cors_validation_disabled_always_passes"
echo ""

echo "=== End of Tests ==="
echo ""
echo "To run actual tests, uncomment the commands in this script"
echo "or run them manually from the project directory."
