# Sicherheitshinweise

## Meldung von Schwachstellen

Bitte **kein** öffentliches Issue für Sicherheitslücken anlegen.
Nutze die GitHub-Funktion *Security → Advisories → Report a vulnerability*
oder schreibe eine E-Mail an den Maintainer (siehe `Cargo.toml`/Profil).

Wir bestätigen den Eingang innerhalb von 72 Stunden und stimmen das weitere
Vorgehen ab. Bitte keine Details veröffentlichen, bevor ein Fix verfügbar ist.

## Relevante Angriffsflächen

Fylr läuft mit den Rechten des angemeldeten Nutzers und darf bewusst
Dateien verschieben und löschen. Deshalb gilt:

- **Löschen** ist nur über die Duplikat-Richtlinie „endgültig löschen" möglich
  und erfordert eine Sicherheitsabfrage im UI.
- Beim Verschieben über Dateisystemgrenzen wird **nie** die Quelle gelöscht,
  bevor die Kopie per BLAKE3 verifiziert wurde.
- Vorhandene Zieldateien werden **nie** überschrieben, sondern hochgezählt.
- Das Tauri-`assetProtocol` ist auf `$HOME/**`, `/home/**`, `/run/media/**`,
  `/media/**` und `/mnt/**` beschränkt; die CSP erlaubt kein Inline-JavaScript.
- Es werden keine Telemetriedaten gesendet; alle Zustände liegen lokal in
  `~/.config/fylr/`.

## Unterstützte Versionen

Sicherheitsfixes erscheinen für die jeweils aktuelle Minor-Version.
