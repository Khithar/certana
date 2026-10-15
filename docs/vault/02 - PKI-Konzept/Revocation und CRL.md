---
type: concept
status: active
tags: [type/concept, area/pki]
---

# Revocation und CRL

Zurück zu [[02 - PKI-Konzept/Zertifikatsprofile]].

## Widerruf

- Gründe nach RFC 5280: `unspecified`, `keyCompromise`, `caCompromise`, `affiliationChanged`, `superseded`, `cessationOfOperation`, `certificateHold`, `privilegeWithdrawn`. v1 bietet alle ausser `certificateHold` an (kein `removeFromCRL`-Fluss).
- Zeitstempel, optionales `invalidityDate`, Kommentar. Widerruf ist irreversibel; `certificates.status = revoked`.
- Audit-Eintrag `revoke` mit Seriennummer und Grund.

## CRL der Intermediate

| Eigenschaft | Wert |
|---|---|
| Laufzeit (`nextUpdate`) | konfigurierbar, Default 7 Tage |
| CRL-Nummer | monoton steigend (`cas.crl_number`) |
| Signatur | Intermediate-Schlüssel (Sitzung muss entsperrt sein) |
| Inhalt | alle widerrufenen, noch nicht abgelaufenen Zertifikate (abgelaufene dürfen entfallen, bleiben in v1 aber bis Ablauf + 1 CRL-Laufzeit) |
| Export | DER (`.crl`) und PEM nach konfigurierbarem Pfad, z. B. `/var/www/pki/issuing.crl` |

Dashboard warnt, wenn die aktuelle CRL in < 2 Tagen abläuft; «CRL erneuern» erzeugt eine neue auch ohne neue Widerrufe.

## Root-CRL

Über die Root-Zeremonie, Laufzeit lang (Default 12 Monate), listet nur widerrufene Intermediates.

## OCSP

Nicht in v1 (siehe [[05 - Entscheidungen/ADR-Index]]). Vorbereitung: AIA kann bereits eine OCSP-URL tragen, sobald ein Responder existiert.

## Betrieb ohne CRL
Entscheid 2026-10-15: Im internen Betrieb wird keine CRL-URL in den CAs hinterlegt. Zertifikate erhalten dann keinen CRL Distribution Point, Clients prüfen nichts. Widerruf in Certana dient nur der eigenen Übersicht und dem Audit-Log; kompromittierte Zertifikate werden durch Ersatz auf dem Server und kurze Laufzeiten abgefangen. Falls doch eine CRL gewünscht wird: Laufzeit pro CA auf 90–365 Tage setzen und nach jedem Widerruf neu veröffentlichen. CRL-Upload (SFTP/FTP) und Post-Publish-Hook wurden besprochen und bewusst nicht gebaut.
