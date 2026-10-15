---
type: concept
status: active
tags: [type/concept, area/pki]
---

# Root-CA (offline)

Zurück zum [[00 - Index]]. Weiter: [[02 - PKI-Konzept/Intermediate-CA]], Formate: [[02 - PKI-Konzept/Schlüsselformate]].

## Grundsatz

Der Root-Schlüssel ist das wertvollste Geheimnis der PKI. Er wird **nie** in der (teilbaren) Datenbank gespeichert und nur für drei Operationen kurz geladen: Intermediate-Zertifikat signieren, Intermediate widerrufen, Root-CRL erneuern.

## Parameter

| Eigenschaft | Wert |
|---|---|
| Schlüssel | ECDSA P-384 (Default) oder RSA 4096 |
| Laufzeit | 10–20 Jahre (Default 15) |
| Subject | CN = «<Organisation> Root CA», O, C aus Settings |
| basicConstraints | `CA:TRUE, pathlen:1` (kritisch) |
| keyUsage | `keyCertSign, cRLSign` (kritisch) |
| SKI / AKI | gesetzt, AKI = SKI (selbstsigniert) |
| Seriennummer | 128 Bit zufällig, positiv |
| Signatur | SHA-384 (P-384) bzw. SHA-256 (RSA) |

## Zeremonie «Root erzeugen»

1. Wizard empfiehlt, Netzwerk zu trennen und auf einem vertrauenswürdigen Gerät zu arbeiten.
2. Parameter wählen, Passphrase (≥ 16 Zeichen, Stärkeanzeige) zweimal eingeben.
3. Erzeugung → sofortiges Schreiben des **Root-Bundles** `<name>.certana-root` an einen frei gewählten Ort (USB-Stick empfohlen).
4. Root-Zertifikat (PEM/DER) wird zusätzlich in der DB als `cas.kind = root` mit `encrypted_key_pkcs8 = NULL` registriert.
5. Hinweis auf Backup-Kopie des Bundles und sichere Aufbewahrung der Passphrase.

## Root-Bundle

Age-verschlüsselter (Passphrase, scrypt, ASCII-Armor) Container um ein JSON-Dokument (`vault::rootbundle`):
- `manifest` – Formatversion, Name, Schlüsselalgorithmus, SHA-256-Fingerprint des Zertifikats, Erstellungszeit, Certana-Version,
- `certificate_pem` – das Root-Zertifikat,
- `encrypted_key_pem` – PKCS#8, zusätzlich PBES2-verschlüsselt mit derselben Passphrase (doppelte Schicht: auch ein entpacktes Bundle liefert keinen Klartextschlüssel).

Beim Öffnen prüft Certana Fingerprint und Zuordnung Schlüssel ↔ Zertifikat; ein falsches Passwort liefert `CertanaError::Vault` ohne Details.

## Import bestehender Root

- PEM/PKCS#8-Schlüssel (verschlüsselt oder nicht) + Zertifikat → Certana packt daraus ein Root-Bundle.
- PKCS#12 → ebenso.
- **Nur Zertifikat** (Root bleibt extern, z. B. HSM oder Fremdsystem): Certana erzeugt die Intermediate-CSR zum externen Signieren und importiert das signierte Intermediate-Zertifikat.

## Zeremonie-Dialog (Signieren, Root-CRL)

Bundle wählen → Passphrase → Operation → Ergebnis in DB → Schlüsselspeicher mit `zeroize` leeren. Jeder Durchlauf erzeugt einen Audit-Eintrag `root_ceremony`.
