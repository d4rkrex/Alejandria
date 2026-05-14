# Binaries

Pre-built binaries are distributed via **GitHub Releases** — not stored in this directory.

## Download

Visit the [Releases page](https://github.com/d4rkrex/Alejandria/releases/latest) to download the binary for your platform:

| Platform | File |
|----------|------|
| Linux x86_64 | `alejandria-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `alejandria-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `alejandria-vX.Y.Z-aarch64-apple-darwin.tar.gz` |

## One-line Installer (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/d4rkrex/Alejandria/main/scripts/install.sh | bash
```

## Manual Installation

```bash
# Download and extract (replace VERSION and PLATFORM)
curl -LO https://github.com/d4rkrex/Alejandria/releases/latest/download/alejandria-VERSION-PLATFORM.tar.gz
tar xzf alejandria-VERSION-PLATFORM.tar.gz
cp alejandria-VERSION-PLATFORM/alejandria ~/.local/bin/alejandria
chmod +x ~/.local/bin/alejandria

# Verify
alejandria --version
```
