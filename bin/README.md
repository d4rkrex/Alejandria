# Pre-compiled Binaries

This directory contains pre-compiled binaries for quick installation without compilation.

## Available Binaries

| Platform | Binary | SHA256 |
|----------|--------|--------|
| Linux x86_64 | `alejandria-linux-x86_64` | See `.sha256` file |
| macOS Intel | Coming soon | - |
| macOS ARM | Coming soon | - |

## Verification

```bash
# Verify checksum
sha256sum -c alejandria-linux-x86_64.sha256

# Expected output:
# alejandria-linux-x86_64: OK
```

## Manual Installation

```bash
# Copy to your PATH
cp bin/alejandria-linux-x86_64 ~/.local/bin/alejandria
chmod +x ~/.local/bin/alejandria

# Verify
alejandria --version
```

## Automatic Installation

The installer script automatically detects and uses these binaries:

```bash
# From repo root
./scripts/install.sh

# From anywhere (one-line install)
curl -fsSL https://gitlab.veritran.net/appsec/alejandria/-/raw/main/scripts/install.sh | bash
```

## Update Binaries

When releasing a new version:

```bash
# Build release
cargo build --release --package alejandria-cli --bin alejandria

# Copy to bin/
cp target/release/alejandria bin/alejandria-linux-x86_64

# Generate checksum
sha256sum bin/alejandria-linux-x86_64 > bin/alejandria-linux-x86_64.sha256

# Commit
git add bin/
git commit -m "chore: update pre-compiled binaries to vX.Y.Z"
git push
```

## Why Pre-compiled Binaries?

- ⚡ **Fast installation**: 30 seconds vs 10 minutes compilation
- 🔒 **Works offline**: No external dependencies
- ✅ **Version controlled**: Binaries tracked with code
- 🚀 **No build tools needed**: Just git clone and install

## Size Note

Binaries are ~32MB each. Git LFS is not used to keep setup simple.
