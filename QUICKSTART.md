# Alejandria Quick Start Guide

Get Alejandria running in under 2 minutes with pre-built binaries.

## One-Line Installation

```bash
curl -fsSL https://raw.githubusercontent.com/VeritranGH/Alejandria/main/scripts/install.sh | bash
```

That's it! The installer will:
- ✅ Detect your platform (Linux/macOS, Intel/ARM)
- ✅ Download the appropriate pre-built binary
- ✅ Detect your MCP clients (OpenCode, Claude Desktop, VSCode)
- ✅ Automatically configure them
- ✅ Create backup of existing configs

## What Gets Installed

- **Binary**: `~/.local/bin/alejandria` (~15MB)
- **Config**: `~/.config/alejandria/config.toml`
- **Database**: `~/.local/share/alejandria/alejandria.db` (created on first use)

## Verification

After installation, verify it works:

```bash
# Check version
alejandria --version

# Store a test memory
alejandria store "First memory" --summary "Test memory"

# Recall it
alejandria recall "test"
```

## MCP Client Integration

The installer automatically configures these clients:

### OpenCode
- Config: `~/.config/opencode/mcp_config.json`
- Restart: Close and reopen OpenCode

### Claude Desktop
- Config: `~/.config/Claude/claude_desktop_config.json` (Linux)
- Config: `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)
- Restart: Quit and relaunch Claude Desktop

### VSCode with MCP Extension
- Config: `~/.config/Code/User/settings.json`
- Restart: Reload window (Cmd/Ctrl+Shift+P → "Developer: Reload Window")

## Manual Configuration

If auto-detection fails, add this to your MCP client config:

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/YOUR_USER/.local/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_CONFIG": "/home/YOUR_USER/.config/alejandria/config.toml"
      }
    }
  }
}
```

Replace `/home/YOUR_USER` with your actual home directory path.

## Advanced Installation Options

### Custom Install Location

```bash
export ALEJANDRIA_INSTALL_DIR="$HOME/bin"
curl -fsSL https://raw.githubusercontent.com/VeritranGH/Alejandria/main/scripts/install.sh | bash
```

### Specific Version

```bash
export ALEJANDRIA_VERSION="v1.6.0"
curl -fsSL https://raw.githubusercontent.com/VeritranGH/Alejandria/main/scripts/install.sh | bash
```

### Force Build from Source

```bash
export FORCE_BUILD=true
curl -fsSL https://raw.githubusercontent.com/VeritranGH/Alejandria/main/scripts/install.sh | bash
```

## Testing MCP Integration

Once configured and restarted, test the MCP connection:

### In OpenCode

```
What memories do I have?
```

The agent should query Alejandria via MCP.

### In Claude Desktop

Create a custom instruction:
```
Always check Alejandria memory before starting new tasks.
```

### Via CLI (Bypass MCP)

```bash
# Store memories
alejandria store "Use TypeScript for new projects" --topic "preferences" --importance high

# Recall memories
alejandria recall "typescript"

# List all topics
alejandria list-topics

# Export memories
alejandria export --output memories.json
```

## Configuration

Edit `~/.config/alejandria/config.toml`:

```toml
[database]
path = "~/.local/share/alejandria/alejandria.db"

[embeddings]
enabled = true
# model = "BAAI/bge-small-en-v1.5"  # Default model

[decay]
critical = 0.99  # Slowest decay (security findings, critical decisions)
high = 0.95      # Important patterns, architecture decisions
medium = 0.90    # Regular development context
low = 0.85       # Temporary notes, experiments

[server]
# MCP server runs on stdio - no additional config needed
```

## Troubleshooting

### Binary Not Found

Add to your shell config (`~/.bashrc` or `~/.zshrc`):

```bash
export PATH="$PATH:$HOME/.local/bin"
```

Then restart your terminal.

### MCP Client Not Connecting

1. **Check config syntax**: Use `jq` to validate JSON
   ```bash
   jq empty ~/.config/opencode/mcp_config.json
   ```

2. **Check binary path**: Ensure it exists
   ```bash
   ls -lh ~/.local/bin/alejandria
   ```

3. **Test binary directly**:
   ```bash
   ~/.local/bin/alejandria --version
   ```

4. **Check logs**: Look for MCP errors in your client's logs
   - OpenCode: Developer Tools Console
   - Claude Desktop: Help → Show Logs

### Installer Fails

If auto-download fails, the installer automatically falls back to building from source. This requires:
- Rust toolchain: https://rustup.rs/
- Git
- 10-20 minutes for compilation

Or download manually from [GitHub Releases](https://github.com/VeritranGH/Alejandria/releases):

```bash
# Download for your platform (example for Linux x86_64)
curl -LO https://github.com/VeritranGH/Alejandria/releases/download/v1.9.6/alejandria-v1.9.6-x86_64-unknown-linux-gnu.tar.gz

# Verify checksum
curl -LO https://github.com/VeritranGH/Alejandria/releases/download/v1.9.6/alejandria-v1.9.6-x86_64-unknown-linux-gnu.tar.gz.sha256
sha256sum -c alejandria-v1.9.6-x86_64-unknown-linux-gnu.tar.gz.sha256

# Extract and install
tar xzf alejandria-v1.9.6-x86_64-unknown-linux-gnu.tar.gz
mkdir -p ~/.local/bin
cp alejandria-v1.9.6-x86_64-unknown-linux-gnu/alejandria ~/.local/bin/
chmod +x ~/.local/bin/alejandria
```

## Updating

Run the installer again - it will replace the old binary:

```bash
curl -fsSL https://raw.githubusercontent.com/VeritranGH/Alejandria/main/scripts/install.sh | bash
```

Your config and database are preserved.

## Uninstalling

```bash
# Remove binary
rm ~/.local/bin/alejandria

# Remove config (optional)
rm -rf ~/.config/alejandria

# Remove database (CAUTION: deletes all memories!)
rm -rf ~/.local/share/alejandria
```

Manually remove the `alejandria` entry from your MCP client configs.

## Next Steps

- Read the full [README](README.md) for architecture details
- Check [examples/](examples/) for language client examples
- Review [docs/](docs/) for API documentation
- Join discussions at GitHub Issues

## Security Notes

The installer:
- ✅ Downloads only via HTTPS
- ✅ Verifies SHA256 checksums before installation
- ✅ Creates timestamped backups of configs
- ✅ Validates JSON syntax before applying changes
- ✅ Supports rollback on any failure
- ✅ Never requires sudo/root privileges

Binary provenance: All releases are built via GitHub Actions from auditable source code.
