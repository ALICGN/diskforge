# Changelog

Alle nennenswerten Änderungen an diesem Projekt werden hier festgehalten.
Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/),
Versionierung nach [Semantic Versioning](https://semver.org/lang/de/).

## [Unreleased]

## [0.2.0] — 2026-11-13

### Geändert — Umbenennung in Fylr
- Projektname von **DiskForge** auf **Fylr** geändert: Binary `fylr`, Crate `fylr_lib`,
  Tauri-Identifier `com.alicgn.fylr`, Fenstertitel „Fylr — Disk Analyzer & Sorter".
- Konfigurationsverzeichnis: `~/.config/diskforge/` → `~/.config/fylr/`
  (`categories.json`, `move_log.json`). Bestehende Regeln müssen einmalig kopiert werden.
- Test-Isolation über `FYLR_HOME` statt `DISKFORGE_HOME`.
- Menüeintrag und Icons heißen `fylr`, Kategorien im `.desktop` korrigiert
  (`Utility;FileTools;Qt;KDE;`).
- Screenshots neu erzeugt, GitHub-Repo umbenannt: `ALICGN/fylr`.

## [0.1.0] — 2026-11-13

### Hinzugefügt
- Paralleler Scan beliebiger Verzeichnisse und Laufwerke (NTFS, btrfs, ext4) mit
  Live-Fortschritt, Abbrechen-Button und Abschlussstatistik.
- Kategorisierung nach Dateiendung und frei editierbaren Überkategorien
  (Bilder, Videos, Audio, Dokumente, Archive, Programme & Skripte, Sonstiges).
- Duplikaterkennung in zwei Stufen: Gruppierung nach Größe, danach BLAKE3-Hash
  (Teil-Hash der ersten/letzten 64 KB bei Dateien > 50 MB).
- Side-by-Side-Vergleich mit Bild-/Video-Vorschau, Auflösung, Dauer, Bitrate,
  Größe sowie mtime/btime/ctime und Markierung „ältestes"/„neuestes".
- Bulk-Helfer „immer älteste behalten" und „kürzester Pfad behalten".
- Sortieren nach Endung, Überkategorie oder hierarchisch, mit Dry-Run-Vorschau.
- Überschreibfreie Namensauflösung (`datei (1).jpg`), EXDEV-Fallback
  (kopieren → BLAKE3 prüfen → Quelle löschen), NTFS-Sonderfälle.
- Duplikat-Richtlinien: überspringen, `_Duplikate`-Ordner, endgültig löschen.
- Sortierprotokoll (`move_log.json`) mit vollständigem Undo des letzten Vorgangs.
- Deutsche Dark-UI (Svelte 5 + Tailwind 4) mit virtuellen Listen für > 50.000 Einträge.
