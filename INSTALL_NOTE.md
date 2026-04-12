# Nota sobre Instalación de Binarios

## Estado Actual

El instalador **compila desde código fuente** por defecto porque el proyecto GitLab es privado y los binarios en Releases requieren autenticación.

### Por qué compila

1. El proyecto `appsec/alejandria` en GitLab Veritran es **privado**
2. La GitLab API requiere token de autenticación para acceder a releases
3. Los instaladores públicos no pueden incluir tokens (seguridad)
4. Por lo tanto: **fallback automático a compilación** (~5-10 min)

### Beneficios de la Compilación

- ✅ **Funciona siempre** (no depende de releases)
- ✅ **Verifica código fuente** (más seguro)
- ✅ **Usa última versión** del repositorio
- ✅ **Auto-limpia cache** (ahorra 11GB)

### Si Quieres Binarios Pre-compilados

**Opción 1: Hacer proyecto público**
```bash
# En GitLab: Settings → General → Visibility → Public
# Luego el instalador descargará binarios automáticamente
```

**Opción 2: Instalar manualmente desde Release**
```bash
# Descarga desde la UI de GitLab (requiere login)
1. Ir a: https://gitlab.veritran.net/appsec/alejandria/-/releases
2. Descargar: alejandria-v1.7.0-tui-dashboard-x86_64-unknown-linux-gnu.tar.gz
3. Extraer: tar xzf alejandria-*.tar.gz
4. Instalar: cp alejandria ~/.local/bin/
```

**Opción 3: Usar mirror público (futuro)**
Si eventualmente publicas en GitHub público:
```bash
GITHUB_REPO=user/alejandria ./scripts/install-mcp-v4.sh
```

### Tiempos de Instalación

| Método | Tiempo | Requiere |
|--------|--------|----------|
| **Binario público** | 30 seg | Proyecto público o mirror |
| **Compilación** | 5-10 min | Rust toolchain |
| **Download manual** | 1 min | Login en GitLab |

### Recomendación

**Para usuarios internos de Veritran**: La compilación automática funciona perfectamente. El tiempo extra (5-10 min) es aceptable para una instalación que se hace una vez.

**Para usuarios externos**: Si necesitas distribución pública, crea un mirror en GitHub público.
