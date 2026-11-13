# Mitmachen bei Fylr

Danke, dass du mithelfen willst! Dieses Dokument beschreibt, wie du das Projekt
lokal aufsetzt, welche Konventionen gelten und wie ein Pull Request abläuft.

## 1. Voraussetzungen

- Linux (getestet auf CachyOS/Arch mit KDE Plasma 6, Wayland **und** X11)
- Rust ≥ 1.77, Node ≥ 20, npm
- Systempakete siehe [README → Installation](README.md#aus-dem-quellcode-bauen)

## 2. Lokales Setup

```bash
git clone https://github.com/ALICGN/fylr.git
cd fylr
npm install
npm run tauri dev          # startet Vite + Rust-Backend mit Hot-Reload
```

Nützliche Befehle:

| Befehl | Zweck |
|---|---|
| `npm run tauri dev` | App im Entwicklungsmodus |
| `npm run build` | Frontend-Bundle erzeugen |
| `npm run check` | `svelte-check` (Typen/A11y) |
| `cd src-tauri && cargo test` | Rust-Unit-Tests |
| `cd src-tauri && cargo fmt --all` | Formatierung |
| `cd src-tauri && cargo clippy --all-targets -- -D warnings` | Lints |
| `./scripts/make-demo-data.sh` | neutraler Demo-Datensatz für Tests/Screenshots |

## 3. Wo was liegt

```
src/lib/tabs/*.svelte      die vier Hauptansichten
src/lib/components/*       wiederverwendbare Bausteine
src/lib/store.svelte.js    einziger Zustandsspeicher (Svelte-5-Runes) + IPC
src-tauri/src/*.rs         Backend-Module, ein Thema pro Datei
```

Faustregel: **Logik ins Backend, Darstellung ins Frontend.** Dateisystem-Zugriffe
gehören ausschließlich nach Rust — das Frontend ruft nur Tauri-Commands auf.

## 4. Code-Konventionen

**Allgemein**
- Sprache im Code: Englische Bezeichner, **deutsche UI-Texte**.
- Keine Secrets, keine Pfade mit persönlichen Daten in Commits oder Screenshots.
- Neue Features brauchen Tests (Rust) bzw. zumindest eine manuelle Prüfanleitung im PR.

**Rust**
- `cargo fmt` und `cargo clippy -D warnings` müssen sauber sein.
- Fehler niemals `unwrap()` auf Dateisystem-Operationen — sammeln und als
  `errors: Vec<String>` zurückgeben, damit ein einzelner Fehler den Lauf nicht abbricht.
- Kein `panic!` in Code, der mit Nutzerdaten läuft.

**Svelte**
- Svelte 5 mit Runes (`$state`, `$derived`, `$effect`), keine Stores mehr.
- Keine `class:`-Direktiven mit Tailwind-Varianten (`bg-accent/12`, `hover:…`) —
  stattdessen Hilfsklassen in `src/app.css` oder konditionale Strings.
- `$state`-Proxies dürfen nicht in `structuredClone` landen → `$state.snapshot()`.

## 5. Commit-Nachrichten

[Conventional Commits](https://www.conventionalcommits.org/de/):

```
feat(scanner): Ignorier-Liste um .venv erweitern
fix(mover): EXDEV-Fallback bei Netzlaufwerken korrigieren
docs: README-Screenshots aktualisieren
chore(deps): blake3 auf 1.6 anheben
```

Erlaubte Typen: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`.

## 6. Pull Requests

1. Issue vorhanden? Wenn nicht, kurz eines anlegen (Bug/Feature) — dann diskutieren wir das Vorgehen.
2. Branch von `main`: `feat/kurzbeschreibung` oder `fix/kurzbeschreibung`.
3. Klein und thematisch: ein PR = ein Thema. Große Umbauten vorher absprechen.
4. PR-Text ausfüllen (Template): Was, Warum, Wie getestet.
5. CI muss grün sein (`cargo test`, `npm run build`, `svelte-check`).
6. Review: mindestens eine Zusage. Danach Squash-Merge.

## 7. Bugs melden

Bitte mit:
- Fylr-Version (`v0.1.0` in der Seitenleiste) und Linux-Distro/Desktop
- Dateisystem des betroffenen Laufwerks (ntfs3, fuseblk, btrfs, ext4 …)
- Schritten zum Nachstellen und **anonymisierten** Pfaden
- Relevantem Log: App aus dem Terminal starten und die Ausgabe mitschicken

## 8. Übersetzungen

Die Oberfläche ist aktuell deutsch. Für weitere Sprachen bitte zuerst ein Issue
aufmachen — wir bauen dann gemeinsam eine i18n-Schicht, statt Texte zu duplizieren.
