---
type: concept
status: active
tags: [type/concept, area/pki]
---

# Zertifikatsprofile

Zurück zu [[02 - PKI-Konzept/Intermediate-CA]]. UI-Fluss: [[04 - UI/Screens#Wizard «Neues Zertifikat»]].

| Profil | keyUsage | EKU | Laufzeit-Default | SAN |
|---|---|---|---|---|
| **Server** (Apache/nginx) | digitalSignature (+ keyEncipherment bei RSA) | serverAuth | 397 Tage, max. 825 | DNS/IP Pflicht, min. 1 |
| **Client-Auth** | digitalSignature | clientAuth | 1 Jahr | E-Mail/UPN optional |
| **S/MIME** | digitalSignature, keyEncipherment | emailProtection | 1–3 Jahre | rfc822Name Pflicht |
| **Code-Signing** | digitalSignature | codeSigning | 1–3 Jahre | – |

Gemeinsam: `CA:FALSE`, SKI/AKI, CRL-DP und AIA der Intermediate, Seriennummer 128 Bit zufällig, SHA-256. CN ist nur informativ (erster DNS-Name bzw. E-Mail); Browser werten ausschliesslich SANs.

## Schlüssel

- Default ECDSA P-256 (Server) – klein, schnell, von allen aktuellen Clients unterstützt.
- RSA 3072 wählbar (Kompatibilität mit alten Clients/Appliances).
- Ed25519 optional mit UI-Warnung (Browser-Support schwach).

## Zwei Wege

1. **Certana erzeugt den Schlüssel** – Export von `name.key.pem`, `name.crt.pem`, `chain.pem`, `fullchain.pem`, optional `.p12` und DER.
2. **CSR importieren** – Schlüssel bleibt auf dem Server. Certana zeigt die CSR-Felder, validiert gegen das Profil und übernimmt **nie** CA-Flags, KU/EKU oder Laufzeiten aus der CSR, nur Public Key, Subject und SANs (nach Bestätigung).

## Server-Snippets

```nginx
ssl_certificate     /etc/ssl/certana/host.fullchain.pem;
ssl_certificate_key /etc/ssl/certana/host.key.pem;
```

```apache
SSLCertificateFile    /etc/ssl/certana/host.fullchain.pem   # ab Apache 2.4.8 inkl. Kette
SSLCertificateKeyFile /etc/ssl/certana/host.key.pem
```
Hinweis im UI: Schlüsseldatei `chmod 0600`, Besitzer root.

## Erneuerung

«Renew» stellt ein neues Zertifikat mit identischen Namen aus, wahlweise mit demselben oder einem neuen Schlüssel; das alte bleibt gültig bis Ablauf oder wird auf Wunsch mit Grund `superseded` widerrufen.
