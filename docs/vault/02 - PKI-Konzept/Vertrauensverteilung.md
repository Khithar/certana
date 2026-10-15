---
type: reference
status: active
tags: [type/reference, area/pki, area/betrieb]
---

# Vertrauensverteilung (Root-Zertifikat auf Clients)

Zurück zum [[00 - Index]]. Diese Anleitungen werden im UI unter «Root exportieren» angezeigt.

| Plattform | Vorgehen |
|---|---|
| Debian/Ubuntu/Mint | `sudo cp root.crt.pem /usr/local/share/ca-certificates/certana-root.crt && sudo update-ca-certificates` |
| RHEL/Fedora | `sudo cp root.crt.pem /etc/pki/ca-trust/source/anchors/ && sudo update-ca-trust` |
| Windows | `certutil -addstore -f Root root.der` (als Administrator) oder GPO «Vertrauenswürdige Stammzertifizierungsstellen» |
| Firefox | eigener Store: Einstellungen → Zertifikate → Importieren, oder Enterprise Policy `Certificates.Install` / `security.enterprise_roots.enabled` |
| macOS | `sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain root.crt.pem` |
| Android | Einstellungen → Sicherheit → Zertifikat installieren (nur Nutzer-Store; Apps vertrauen ihm ab API 24 nur mit Network-Security-Config) |
| Java | `keytool -importcert -trustcacerts -alias certana-root -file root.der -cacerts` |

Nur das **Root**-Zertifikat verteilen, nie die Intermediate (die kommt per `fullchain.pem` vom Server).
