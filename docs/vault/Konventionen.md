---
type: reference
status: active
tags: [type/reference]
---

# Vault-Konventionen

- **Sprache:** Deutsch (Schweizer Schreibweise, «ss» statt «ß»). Code-Bezeichner, Crate-Namen, Befehle bleiben englisch.
- **Frontmatter:** `type` (moc | concept | reference | adr | session), `status` (draft | active | superseded), `tags`.
- **Wikilinks** statt Pfade; jede Seite verlinkt zurück auf ihre MOC bzw. den [[00 - Index]].
- **ADRs** unter `05 - Entscheidungen/` als `YYYY-MM-DD-<slug>.md` mit Kontext → Entscheidung → Konsequenzen. Eine getroffene Entscheidung wird nie gelöscht, sondern mit `status: superseded` und Link auf die Nachfolgerin markiert.
- **Sessions** unter `99 - Sessions/` als `YYYY-MM-DD <Titel>.md`: was wurde getan, was ist offen, welche Vault-Seiten wurden geändert.
- **Was nicht hinein gehört:** Passphrasen, private Schlüssel, Testzertifikate mit echten Hostnamen, Secrets jeder Art. Ebenso keine Firmen-, Host-, Share- oder Benutzernamen aus der realen Umgebung (der Vault ist öffentlich auf GitHub); Beispiele mit `example`-Namen schreiben.
- `.obsidian/` wird bis auf `app.json` nicht versioniert.

Zurück zum [[00 - Index]].
