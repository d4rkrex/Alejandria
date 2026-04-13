# Certificados TLS para Alejandría MCP

## 📋 Contenido de este Directorio

Este directorio contiene los **certificados públicos** para TLS de Alejandría.

### ✅ Archivos Versionados (Públicos - Seguros para compartir)

| Archivo | Descripción | Uso |
|---------|-------------|-----|
| `ca-cert.pem` | Certificado público del CA raíz | Instalar en clientes para confiar en TLS |
| `server-cert.pem` | Certificado público del servidor | Usado por Caddy reverse proxy |
| `README.md` | Este archivo | Documentación |

### ❌ Archivos NO Versionados (Privados - Git Ignore)

Estos archivos existen **solo en el servidor** `ar-appsec-01.veritran.net`:

| Archivo | Descripción | Ubicación |
|---------|-------------|-----------|
| `ca-key.pem` | Clave privada del CA | Servidor (NUNCA versionar) |
| `server-key.pem` | Clave privada del servidor | Caddy container |
| `*.csr` | Certificate Signing Requests | Temporal (no se versiona) |

---

## 🔐 Seguridad

### ¿Es seguro versionar estos certificados públicos?

**SÍ, 100% seguro.**

Los certificados públicos (`.pem`, `.crt`) contienen solo la **clave pública**, que por diseño es información pública. 

**Analogía:** Es como compartir un candado. Cualquiera puede usarlo para cerrar algo, pero solo quien tiene la llave (clave privada) puede abrirlo.

**Lo que NO puedes hacer con `ca-cert.pem`:**
- ❌ Descifrar tráfico TLS
- ❌ Generar nuevos certificados válidos
- ❌ Hacer MITM attacks
- ❌ Impersonar el servidor

**Lo que SÍ puedes hacer:**
- ✅ Verificar certificados firmados por este CA
- ✅ Confiar en conexiones TLS de Alejandría
- ✅ Saber que Alejandría usa TLS (info no sensible)

---

## 📥 Instalación para Desarrolladores

### Opción A: Instalar CA Cert en Sistema (Recomendado)

```bash
# Linux
sudo cp certs/ca-cert.pem /usr/local/share/ca-certificates/alejandria-veritran.crt
sudo update-ca-certificates

# macOS
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain certs/ca-cert.pem
```

**Ventaja:** Todos los programas confiarán automáticamente (curl, navegadores, etc.)

---

### Opción B: Usar CA Cert Directamente

```bash
# Copiar a directorio personal
mkdir -p ~/.alejandria
cp certs/ca-cert.pem ~/.alejandria/

# Test manual con curl
curl --cacert ~/.alejandria/ca-cert.pem \
     -H 'X-API-Key: alejandria-prod-initial-key-2026' \
     https://ar-appsec-01.veritran.net/alejandria/health
```

**Ventaja:** No requiere permisos sudo

---

## 🔄 Rotación de Certificados

### Validez Actual

```bash
# Ver expiración del CA
openssl x509 -in certs/ca-cert.pem -noout -enddate
# Expira: Apr 9 01:24:25 2036 GMT (10 años)

# Ver expiración del servidor
openssl x509 -in certs/server-cert.pem -noout -enddate
# Expira: Apr 11 01:24:27 2028 GMT (2 años)
```

### Cuándo Rotar

| Certificado | Validez | Rotación Recomendada |
|-------------|---------|---------------------|
| CA Root | 10 años | Cada 5 años o si se compromete |
| Server | 2 años | Cada 1 año (automatizable) |

### Proceso de Rotación

Ver `../docs/HTTP_SETUP.md` para el flujo actual de regeneración y despliegue de certificados.

---

## 🧪 Validación

### Verificar Integridad de Certificados

```bash
# Verificar que server-cert fue firmado por ca-cert
openssl verify -CAfile certs/ca-cert.pem certs/server-cert.pem
# Esperado: certs/server-cert.pem: OK

# Ver detalles del CA cert
openssl x509 -in certs/ca-cert.pem -text -noout | head -20

# Ver Subject Alternative Names del servidor
openssl x509 -in certs/server-cert.pem -text -noout | grep -A2 "Subject Alternative Name"
```

---

## 🚨 Incident Response

### Si `ca-key.pem` se compromete (CRÍTICO)

1. **PÁNICO CONTROLADO** - Esto invalida toda la PKI
2. Generar nuevo CA desde cero
3. Re-firmar TODOS los certificados de servidores
4. Distribuir nuevo `ca-cert.pem` a TODOS los clientes
5. Investigar qué certificados maliciosos se generaron

### Si `server-key.pem` se compromete (ALTO)

1. Generar nuevo par de claves del servidor
2. Firmar nuevo certificado con el CA existente
3. Desplegar en Caddy (reload, sin downtime)
4. Revocar certificado comprometido

---

## 📚 Referencias

- [Guía HTTP/TLS actual](../docs/HTTP_SETUP.md)
- [Guía de despliegue](../docs/DEPLOYMENT.md)
- [OpenSSL CA Tutorial](https://jamielinux.com/docs/openssl-certificate-authority/)
- [OWASP Transport Layer Protection](https://cheatsheetseries.owasp.org/cheatsheets/Transport_Layer_Protection_Cheat_Sheet.html)

---

## ❓ FAQ

**P: ¿Por qué usamos certificados autofirmados en vez de Let's Encrypt?**  
R: Alejandría corre en red interna (no Internet público). Let's Encrypt requiere validación DNS pública. Certificados autofirmados son perfectos para intranets corporativas.

**P: ¿Los navegadores mostrarán un warning?**  
R: Solo si accedes con navegador web SIN instalar el CA cert. Los clientes MCP usan el trust store del sistema, donde instalamos `ca-cert.pem`.

**P: ¿Podemos usar certificados de Veritran IT en vez de autofirmados?**  
R: ¡SÍ! Si Veritran tiene una PKI corporativa, es ideal usar esos certificados. Reemplaza `ca-cert.pem` con el CA corporativo.

**P: ¿Qué pasa si accidentalmente versioné una clave privada?**  
R: 1) NO hagas push, 2) Borra del working tree, 3) `git reset HEAD <file>`, 4) Avisa a @appsec para rotar certificados.

---

**Generado:** 11 Abril 2026  
**Team:** Veritran AppSec  
**Clasificación:** Pública (puede compartirse libremente)
