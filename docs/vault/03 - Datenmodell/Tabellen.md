---
type: reference
status: active
tags: [type/reference, area/datenmodell]
---

# Tabellen (SQLite)

Zurück zum [[00 - Index]]. Betrieb: [[03 - Datenmodell/Shared-SQLite-Betrieb]]. Migrationen via `rusqlite_migration` (`PRAGMA user_version`), SQL unter `certana-core/src/db/sql/`. Zeilen-Typen in `db/models.rs`, Repositories als `impl Database` in `db/repo_*.rs`.

```sql
settings      (key TEXT PK, value TEXT)
cas           (id, kind 'root'|'intermediate', name, cert_pem, cert_sha256, subject,
               not_before, not_after, key_algo, encrypted_key_pkcs8 NULL bei Offline-Root,
               parent_ca_id → cas, crl_dp_url, aia_url, crl_number, crl_days, created_at)
certificates  (id, ca_id → cas, serial_hex UNIQUE, profile, subject, sans_json,
               not_before, not_after, cert_pem, encrypted_key_pkcs8 NULL bei CSR-Import,
               csr_pem, status 'valid'|'revoked'|'expired', notes, created_at, created_by)
revocations   (cert_id PK → certificates, revoked_at, reason, invalidity_date, comment,
               first_crl_id → crls)
crls          (id, ca_id → cas, crl_number, this_update, next_update, crl_der, exported_path)
audit_log     (id, ts, actor, host, action, target_type, target_id, details_json)
```

- `status = expired` wird über `Database::refresh_expired()` aus `not_after` abgeleitet (App-Start, Dashboard-Refresh).
- `revocations.first_crl_id` wird beim ersten `insert_crl` der betreffenden CA gesetzt.
- `cas.crl_number` wird mit `next_crl_number()` atomar hochgezählt.
- Verschlüsselte Schlüsselspalten tragen `#[serde(skip_serializing)]`; die UI erhält nur `has_key`.
- Schreibende Methoden prüfen `ensure_writable()` (Read-only-Modus bei fremdem Lock).
- `PRAGMA foreign_keys = ON` bei jeder Verbindung.
- Zeiten als ISO-8601 UTC-Text.
