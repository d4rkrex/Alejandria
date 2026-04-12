# Análisis de Seguridad: Distribución de Certificados CA

## 🤔 Pregunta del Usuario

> "¿Cada dev debe importar un certificado? ¿Ese certificado se puede colocar en el repo o es peligroso?"

---

## 🔐 Respuesta Detallada

### Parte 1: ¿Qué es un CA Certificate?

Un **CA (Certificate Authority) certificate** tiene **DOS componentes**:

```
┌─────────────────────────────────────┐
│ CA Certificate (Autoridad Raíz)    │
├─────────────────────────────────────┤
│ 1. Certificado Público (ca-cert.pem)│ ← PÚBLICO (compartible)
│    • Contiene: clave pública        │
│    • Usado para: VERIFICAR firmas   │
│    • Riesgo si expuesto: NINGUNO    │
├─────────────────────────────────────┤
│ 2. Clave Privada (ca-key.pem)      │ ← SECRETO (NUNCA compartir)
│    • Contiene: clave privada        │
│    • Usado para: FIRMAR certificados│
│    • Riesgo si expuesto: CRÍTICO    │
└─────────────────────────────────────┘
```

---

## ✅ LO QUE SÍ PUEDES PONER EN EL REPO

### 1. CA Certificate Público (`ca-cert.pem`)

**Archivo:**
```
~/repos/AppSec/Alejandria/certs/ca-cert.pem
```

**Contenido:**
```
-----BEGIN CERTIFICATE-----
MIIBozCCAUqgAwIBAgIRAPSeAlg7D/JZ3HMEcv+JCn8wCgYIKoZIzj0EAwIwMDEu
...
-----END CERTIFICATE-----
```

**¿Es seguro compartirlo?**
✅ **SÍ, 100% SEGURO**

**Razón:**
- Es información **PÚBLICA** por diseño
- Solo sirve para **VERIFICAR** certificados firmados por tu CA
- No permite **FIRMAR** nuevos certificados (para eso necesitas la clave privada)
- Equivalente a compartir una llave para **cerrar** pero no para **abrir**

**Comparación con otros certificados públicos:**
- **Let's Encrypt CA cert:** Público, descargable por cualquiera
- **DigiCert CA cert:** Público, incluido en navegadores
- **Tu CA cert:** Público, solo para tu organización

---

### 2. Server Certificate Público (`server-cert.pem`)

**Archivo:**
```
~/repos/AppSec/Alejandria/certs/server-cert.pem
```

**¿Es seguro compartirlo?**
✅ **SÍ, SEGURO**

**Razón:**
- Es el certificado que el servidor presenta a los clientes
- Ya es visible para cualquiera que se conecte al servidor (`openssl s_client`)
- No contiene información secreta

---

## ❌ LO QUE NUNCA DEBES PONER EN EL REPO

### 1. CA Private Key (`ca-key.pem`)

**Archivo:**
```
~/repos/AppSec/Alejandria/certs/ca-key.pem
```

**Contenido:**
```
-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC...
-----END PRIVATE KEY-----
```

**¿Es seguro compartirlo?**
❌ **NO, EXTREMADAMENTE PELIGROSO**

**Impacto si se compromete:**
🔴 **CRÍTICO** - Atacante puede:
1. Generar certificados válidos para cualquier dominio
2. Hacer MITM attacks impersonando tu servidor
3. Firmar malware como si fuera legítimo
4. Comprometer TODA la confianza de tu PKI

**Remediación si se filtra:**
1. Revocar TODOS los certificados firmados por ese CA
2. Generar nuevo CA desde cero
3. Re-distribuir nuevo CA cert a TODOS los clientes
4. Incident response (análisis de qué certificados maliciosos se generaron)

---

### 2. Server Private Key (`server-key.pem`)

**Archivo:**
```
~/repos/AppSec/Alejandria/certs/server-key.pem
```

**¿Es seguro compartirlo?**
❌ **NO, MUY PELIGROSO**

**Impacto si se compromete:**
🟠 **ALTO** - Atacante puede:
1. Impersonar el servidor específico (ar-appsec-01.veritran.net)
2. Hacer MITM para ese servidor
3. Descifrar tráfico histórico capturado (si no hay Perfect Forward Secrecy)

**Remediación si se filtra:**
1. Generar nuevo certificado del servidor
2. Revocar certificado comprometido
3. Rotar en Caddy (reload, no requiere reiniciar todo)

---

## 📁 Estructura de Repo Segura

### Recomendación: Separar Público vs Secreto

```bash
~/repos/AppSec/Alejandria/
├─ certs/                      # EN REPO (público)
│  ├─ ca-cert.pem              ✅ Seguro compartir
│  ├─ server-cert.pem          ✅ Seguro compartir
│  └─ README.md                ✅ Instrucciones de instalación
│
├─ certs-private/              # .gitignore (secreto)
│  ├─ ca-key.pem               ❌ NUNCA compartir
│  ├─ server-key.pem           ❌ NUNCA compartir
│  └─ .gitignore               ← Asegura que no se versione
│
└─ .gitignore                  # Raíz del repo
```

**Contenido de `.gitignore`:**
```gitignore
# Certificados - Claves Privadas (NUNCA versionar)
certs-private/
*.key
*.key.pem
ca-key.pem
server-key.pem
*-key.pem

# Certificados públicos pueden versionarse:
# certs/ca-cert.pem  ← Comentado, SÍ se versiona
# certs/server-cert.pem  ← Comentado, SÍ se versiona
```

---

## 🎯 Estrategias de Distribución del CA Cert

### Opción 1: **Via Repo Git (Recomendado para equipos pequeños)** ✅

**Pros:**
- ✅ Fácil de distribuir (todos tienen acceso al repo)
- ✅ Versionado (ves cuándo cambió el CA cert)
- ✅ Automático (cada dev hace `git pull` y ya tiene el cert)

**Cons:**
- ⚠️ Si repo es público en GitHub, el CA cert es público (pero eso es OK)

**Implementación:**
```bash
# En el repo
git add certs/ca-cert.pem
git commit -m "feat: add Caddy CA certificate for TLS verification"
git push

# Cada dev
git pull
cp ~/repos/AppSec/Alejandria/certs/ca-cert.pem ~/.alejandria/
# O instalar en sistema:
sudo cp ~/repos/AppSec/Alejandria/certs/ca-cert.pem /usr/local/share/ca-certificates/alejandria-veritran.crt
sudo update-ca-certificates
```

**Seguridad:** ✅ **SEGURO** (es información pública)

---

### Opción 2: **Via Script de Instalador (Automatizado)** ✅

**El instalador descarga el CA cert del servidor automáticamente:**

```bash
#!/bin/bash
# Parte del instalador
echo "Descargando CA certificate de Caddy..."
ssh mroldan@ar-appsec-01.veritran.net \
    "docker exec veriscan-proxy cat /data/caddy/pki/authorities/local/root.crt" \
    > ~/.alejandria/ca-cert.pem
```

**Pros:**
- ✅ Siempre obtiene la versión más reciente
- ✅ No requiere versionar en Git
- ✅ Funciona incluso si el CA se rota

**Cons:**
- ⚠️ Requiere acceso SSH al servidor
- ⚠️ Primera instalación más lenta

---

### Opción 3: **Via Internal Package Manager (Enterprise)** 🏆

Si Veritran tiene infraestructura avanzada:

```bash
# Via Ansible / Chef / Puppet
ansible-playbook install-alejandria-ca.yml

# Via internal apt repo
sudo apt install veritran-alejandria-ca-cert

# Via Vault / Secrets Manager
vault kv get secret/alejandria/ca-cert
```

**Pros:**
- ✅ Centralizado
- ✅ Auditable
- ✅ Rotación automatizada

**Cons:**
- ⚠️ Requiere infraestructura compleja

---

## 🛡️ Threat Model: ¿Qué pasa si el CA Cert es Público?

### Escenario: Atacante obtiene `ca-cert.pem`

**¿Puede el atacante...?**

| Acción | Posible | Impacto |
|--------|---------|---------|
| Descifrar tráfico TLS pasado | ❌ NO | Ninguno (necesita clave privada) |
| Descifrar tráfico TLS futuro | ❌ NO | Ninguno (necesita clave privada) |
| Generar certificados válidos | ❌ NO | Ninguno (necesita clave privada) |
| Hacer MITM impersonando servidor | ❌ NO | Ninguno (necesita clave privada) |
| Verificar certificados legítimos | ✅ SÍ | 🟢 **BAJO** (eso es su función) |
| Saber que Alejandría usa TLS | ✅ SÍ | 🟢 **BAJO** (info no sensible) |

**Conclusión:** Exponer `ca-cert.pem` públicamente tiene **RIESGO MÍNIMO**.

---

### Escenario: Atacante obtiene `ca-key.pem` (clave privada)

**¿Puede el atacante...?**

| Acción | Posible | Impacto |
|--------|---------|---------|
| Generar certificados para `*.veritran.net` | ✅ SÍ | 🔴 **CRÍTICO** |
| Hacer MITM impersonando Alejandría | ✅ SÍ | 🔴 **CRÍTICO** |
| Firmar malware como "trusted" | ✅ SÍ | 🔴 **CRÍTICO** |
| Comprometer toda la PKI | ✅ SÍ | 🔴 **CRÍTICO** |

**Conclusión:** Exponer `ca-key.pem` es **CATASTRÓFICO**.

---

## 📋 Checklist de Seguridad para Certificados

### Antes de Versionar Certificados en Git

- [ ] Verificar que es `ca-cert.pem` (público), NO `ca-key.pem` (privado)
- [ ] Verificar que es `server-cert.pem` (público), NO `server-key.pem` (privado)
- [ ] Agregar `*.key` y `*.key.pem` a `.gitignore`
- [ ] Agregar `ca-key.pem` explícitamente a `.gitignore`
- [ ] Revisar historial de Git por si se versionó por error:
  ```bash
  git log --all --full-history -- "**/ca-key.pem"
  git log --all --full-history -- "**/*.key"
  ```
- [ ] Si se versionó clave privada por error: **ROTAR INMEDIATAMENTE**

---

### Después de Comprometer una Clave Privada (Incident Response)

**Si `server-key.pem` se filtró:**
1. [ ] Generar nuevo par de claves para el servidor
2. [ ] Firmar nuevo certificado con el CA
3. [ ] Desplegar en Caddy (reload sin downtime)
4. [ ] Revocar certificado comprometido (CRL/OCSP)
5. [ ] Analizar logs para detectar uso malicioso

**Si `ca-key.pem` se filtró:**
1. [ ] **PÁNICO CONTROLADO** - Esto es serio
2. [ ] Generar nuevo CA completo desde cero
3. [ ] Re-firmar TODOS los certificados de servidores
4. [ ] Distribuir nuevo CA cert a TODOS los clientes
5. [ ] Revocar CA antiguo completamente
6. [ ] Forensics: revisar qué certificados maliciosos se generaron
7. [ ] Post-mortem: cómo se filtró y cómo prevenir

---

## ✅ Recomendación para Veritran AppSec Team

### Estrategia Recomendada (Balance Seguridad vs Usabilidad)

**1. Versionar CA Cert Público en Repo**

```bash
cd ~/repos/AppSec/Alejandria
git add certs/ca-cert.pem
git commit -m "feat: add Alejandría CA certificate for TLS trust"
git push
```

**Justificación:**
- ✅ Fácil distribución a todo el equipo
- ✅ Versionado (tracking de rotaciones)
- ✅ Seguro (es información pública)
- ✅ Automatizable en instalador

---

**2. NUNCA Versionar Claves Privadas**

```bash
# Asegurar .gitignore
cat >> ~/repos/AppSec/Alejandria/.gitignore <<EOF

# Certificados - Claves Privadas (CRÍTICO: NUNCA versionar)
ca-key.pem
server-key.pem
*.key
*.key.pem
certs-private/
EOF

git add .gitignore
git commit -m "security: add gitignore for private keys"
git push
```

---

**3. Claves Privadas Solo en Servidor Producción**

```
ar-appsec-01.veritran.net:
  /etc/alejandria/tls/
    server-key.pem  (permisos 600, owner: root)
    server-cert.pem (permisos 644, owner: root)

Caddy container (veriscan-proxy):
  /data/caddy/pki/authorities/local/
    root.crt  (CA cert público)
    root.key  (CA key privado - DENTRO del container)
```

**Ventaja:** Claves privadas nunca salen del servidor.

---

**4. Actualizar Instalador para Distribuir CA Cert**

```bash
# En install-mcp.sh
echo "Instalando CA certificate de Alejandría..."

# Opción A: Desde repo (recomendado)
if [[ -f "certs/ca-cert.pem" ]]; then
    mkdir -p ~/.alejandria
    cp certs/ca-cert.pem ~/.alejandria/
    echo "✓ CA cert instalado desde repo"
fi

# Opción B: Desde servidor (fallback)
if [[ ! -f ~/.alejandria/ca-cert.pem ]]; then
    ssh mroldan@ar-appsec-01.veritran.net \
        "docker exec veriscan-proxy cat /data/caddy/pki/authorities/local/root.crt" \
        > ~/.alejandria/ca-cert.pem 2>/dev/null || true
fi
```

---

## 🎓 Educación al Equipo

### Email Template para el Team

```
Subject: [AppSec] Certificados TLS de Alejandría - Qué versionar y qué NO

Equipo,

Implementamos TLS para Alejandría vía Caddy reverse proxy. 

IMPORTANTE - Certificados:

✅ SEGURO compartir (ya está en el repo):
  • certs/ca-cert.pem (certificado público del CA)
  
❌ NUNCA compartir (ni versionar):
  • ca-key.pem (clave privada del CA)
  • server-key.pem (clave privada del servidor)
  • Cualquier archivo *.key o *.key.pem

Si necesitas el CA cert:
  git pull
  cp certs/ca-cert.pem ~/.alejandria/

Si accidentalmente versionas una clave privada:
  1. NO hagas push
  2. Avisa a @appsec inmediatamente
  3. Rotaremos el certificado

¿Dudas? Ping en #appsec-team

Saludos,
AppSec Team
```

---

## 📊 Comparación con Otras Empresas

### ¿Qué hacen otras empresas con CA Certs internos?

| Empresa | CA Cert | Clave Privada |
|---------|---------|---------------|
| **Google** | Público (Chrome trust store) | HSM (Hardware Security Module) |
| **Netflix** | Distribuido via Spinnaker | Vault (HashiCorp) |
| **Amazon** | Pre-instalado en AMIs | AWS KMS |
| **Tu Caso (Veritran)** | Repo Git (recomendado) ✅ | Servidor (archivo con permisos 600) ⚠️ |

**Mejora futura:** Migrar claves privadas a HSM o Vault.

---

## ✅ Acción Inmediata

```bash
# 1. Versionar CA cert público (SEGURO)
cd ~/repos/AppSec/Alejandria
git add certs/ca-cert.pem
git commit -m "feat: add Caddy CA certificate for TLS verification

This is the PUBLIC certificate (ca-cert.pem) used to verify
Alejandría's TLS connections via Caddy reverse proxy.

Safe to share - does not contain private key material."
git push

# 2. Asegurar que claves privadas NO se versionen
cat >> .gitignore <<EOF

# Certificados - Claves Privadas
ca-key.pem
server-key.pem
*.key
*.key.pem
certs-private/
EOF
git add .gitignore
git commit -m "security: prevent private keys from being committed"
git push

# 3. Verificar historial limpio
git log --all --full-history --oneline -- "**/*.key*" | wc -l
# Debe ser 0 (ningún commit con claves privadas)
```

---

## 🎯 Respuesta Final a tu Pregunta

> "¿Ese certificado se puede colocar en el repo o es peligroso?"

**Respuesta:**

✅ **SÍ, el CA certificate (público) es SEGURO ponerlo en el repo**

❌ **NO, la clave privada del CA es PELIGROSO ponerla en el repo**

**Analogía:**
- CA cert público = Candado (cualquiera puede usarlo para cerrar, pero solo quien tenga la llave puede abrir)
- CA key privada = Llave maestra (quien la tenga puede abrir TODO)

**Acción:** Versiona `ca-cert.pem` en Git, distribúyelo al equipo, y asegura que `ca-key.pem` NUNCA se versione.

---

**Preparado por:** AppSec Team - Veritran  
**Fecha:** 11 Abril 2026  
**Clasificación:** Pública (puede compartirse)
