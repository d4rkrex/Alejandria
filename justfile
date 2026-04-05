# Alejandria Build Automation
# Requires: just (cargo install just)

# Default recipe - show available commands
default:
    @just --list

# === Build Recipes ===

# Build all workspace crates in release mode
build:
    cargo build --release

# Build and run tests
test:
    cargo test

# Run clippy linter
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code with rustfmt
fmt:
    cargo fmt --all

# Check code without building
check:
    cargo check --all-targets --all-features

# === Docker Build Recipes ===

# Build CLI Docker image
docker-build-cli:
    docker build -f Dockerfile.cli -t alejandria-cli:latest .

# Build MCP Docker image
docker-build-mcp:
    docker build -f Dockerfile.mcp -t alejandria-mcp:latest .

# Build both CLI and MCP Docker images
docker-build: docker-build-cli docker-build-mcp

# Build multi-platform images for amd64 and arm64 with custom tag
docker-buildx TAG="latest":
    docker buildx build --platform linux/amd64,linux/arm64 -f Dockerfile.cli -t alejandria-cli:{{TAG}} --load .
    docker buildx build --platform linux/amd64,linux/arm64 -f Dockerfile.mcp -t alejandria-mcp:{{TAG}} --load .

# === Docker Run Recipes ===

# Run CLI container with custom arguments
docker-run-cli ARGS="--help":
    docker run --rm alejandria-cli:latest {{ARGS}}

# Run MCP server container with named volume
docker-run-mcp:
    docker run -d -v alejandria-data:/data --name alejandria-mcp alejandria-mcp:latest

# === Docker Cleanup Recipes ===

# Remove all Alejandria Docker images and prune dangling build cache
docker-clean:
    -docker rmi alejandria-cli:latest
    -docker rmi alejandria-mcp:latest
    docker image prune -f

# Stop and remove MCP container (if running)
docker-stop-mcp:
    -docker stop alejandria-mcp
    -docker rm alejandria-mcp

# === Combined Development Recipes ===

# Full clean build: format, lint, test, build
ci: fmt lint test build

# Quick development loop: format, check, test
dev: fmt check test
