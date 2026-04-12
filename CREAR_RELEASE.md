# Crear Release en GitLab - Instrucciones

## 📦 Assets Preparados

Los archivos están listos en: `~/repos/AppSec/Alejandria/dist/`

- ✅ `alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz` (9.5 MB)
- ✅ `alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz.sha256` (130 bytes)

**SHA256**: `97d7f6edf6b38d85e8e66b3f019e945682bb5c4dc6ae6978c5b1ee0fd895d4c5`

---

## 🚀 Pasos para Crear el Release

### 1. Abrir GitLab Releases

Navega a: https://gitlab.veritran.net/appsec/alejandria/-/releases/new

### 2. Completar Formulario

**Tag name**: `v1.7.0-tui-dashboard` (selecciona del dropdown - ya existe)

**Release title**: `Alejandria v1.7.0 - TUI Dashboard`

**Release notes**: (copiar y pegar)

```markdown
## Alejandria v1.7.0 - TUI Dashboard

### 🎉 New Features

**TUI Admin Dashboard**
- Interactive terminal UI for API key management
- SSH-friendly, no Web UI needed
- 3 tabs: API Keys, Stats, Activity Log
- Vim-style keybindings (j/k, gg/G, Enter, r/R, f, /)
- Command: `alejandria tui` or `alejandria admin tui`

**Improved Installation**
- Pre-built binary for Linux x86_64
- One-line installer with auto-configuration
- Automatic MCP client detection (OpenCode, Claude Desktop, VSCode)
- Auto-cleanup of build cache (saves 11GB of disk space)

### 📦 Installation

\`\`\`bash
# One-liner install
curl -fsSL https://gitlab.veritran.net/appsec/alejandria/-/raw/main/scripts/install-mcp-v4.sh | bash
\`\`\`

Or download manually:

\`\`\`bash
wget https://gitlab.veritran.net/appsec/alejandria/-/raw/main/scripts/install-mcp-v4.sh
chmod +x install-mcp-v4.sh
./install-mcp-v4.sh
\`\`\`

### ✅ Checksum Verification

\`\`\`bash
# Download checksum file
wget https://gitlab.veritran.net/appsec/alejandria/-/releases/v1.7.0-tui-dashboard/downloads/alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz.sha256

# Verify
sha256sum -c alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz.sha256

# Expected: 97d7f6edf6b38d85e8e66b3f019e945682bb5c4dc6ae6978c5b1ee0fd895d4c5
\`\`\`

### 🔧 Manual Installation

\`\`\`bash
# Download binary
wget https://gitlab.veritran.net/appsec/alejandria/-/releases/v1.7.0-tui-dashboard/downloads/alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz

# Extract
tar xzf alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz
cd alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu

# Install
cp alejandria ~/.local/bin/
chmod +x ~/.local/bin/alejandria

# Verify
alejandria --version
alejandria tui
\`\`\`

### 📝 What's Changed

**v1.6.0 → v1.7.0**
- Added TUI admin dashboard (767 lines)
- Added uninstaller with `--keep-data` option
- Fixed OpenCode config format (mcpServers → mcp)
- Fixed installer edge cases (binary in use, missing repo)
- Auto-cleanup build cache (saves 11GB)
- Skip reinstall if version matches

**Full Changelog**: https://gitlab.veritran.net/appsec/alejandria/-/compare/v1.5.0-p0-2-complete...v1.7.0-tui-dashboard

### 📝 Supported Platforms

- ✅ **Linux x86_64** (tested on Ubuntu 22.04)
- ⏳ macOS Intel (waiting for CI runners)
- ⏳ macOS ARM (waiting for CI runners)

### 🔒 Security

This release includes:
- Multi-key API authentication (P0-2 complete)
- CORS whitelist protection (P0-3 complete)
- BOLA/IDOR protection (P0-5 complete)
- Input validation and sanitization
- Auto-generated API keys with SHA-256 hashing

See [SECURITY_REMEDIATION_PLAN.md](https://gitlab.veritran.net/appsec/alejandria/-/blob/main/SECURITY_REMEDIATION_PLAN.md) for full details.
```

### 3. Subir Assets

En la sección **"Release assets"**, sube los archivos:

**Método 1 - Drag & Drop:**
1. Arrastra los 2 archivos desde `~/repos/AppSec/Alejandria/dist/`
2. GitLab los subirá automáticamente

**Método 2 - Click Upload:**
1. Click en **"Upload file"** o **"Add another link"**
2. Navega a `~/repos/AppSec/Alejandria/dist/`
3. Selecciona:
   - `alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz`
   - `alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz.sha256`

### 4. Publicar

Click en **"Create release"** ✅

---

## ✅ Verificar el Release

Después de crear el release, prueba el instalador:

\`\`\`bash
# Debería descargar el binario pre-compilado (~30 segundos)
curl -fsSL https://gitlab.veritran.net/appsec/alejandria/-/raw/main/scripts/install-mcp-v4.sh | bash

# Verificar
alejandria --version  # alejandria 0.1.0
alejandria tui        # Abre TUI dashboard
\`\`\`

---

## 📍 Ubicación de los Archivos

\`\`\`
~/repos/AppSec/Alejandria/dist/
├── alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz (9.5 MB)
└── alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz.sha256 (130 bytes)
\`\`\`
