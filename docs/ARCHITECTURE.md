# Architektur

## Überblick

```
┌────────────────────────────── Webview (Svelte 5) ─────────────────────────────┐
│  App.svelte ── Tabs (Overview · Duplicates · Rules · LogTab)                   │
│       │                                                                        │
│       └── lib/store.svelte.js  — einziger Zustand, kapselt alle invoke()-Calls  │
└───────────────┬───────────────────────────────────────────────▲───────────────┘
                │ invoke("command", args)                       │ emit("df://…")
┌───────────────▼───────────────────────────────────────────────┴───────────────┐
│ commands.rs  — AppState, Commands, Event-Emitter, Task-Guard                   │
│   scanner.rs → hasher.rs → mover.rs → categories.rs · metadata.rs · mounts.rs  │
└────────────────────────────────────────────────────────────────────────────────┘
```

**Grundsatz:** Das Frontend kennt keine Dateisystem-APIs. Jede Operation läuft in Rust
und wird über ein Command ausgelöst; Fortschritt kommt ausschließlich über Events zurück.

## Datenfluss eines Scans

1. `start_scan` prüft den Pfad, setzt das `cancel`-Flag zurück und ruft
   `scanner::scan_tree` in `spawn_blocking` auf (blockiert keinen Tokio-Worker).
2. `scan_tree` startet `ignore::WalkBuilder::build_parallel()` mit `num_cpus` Threads.
   Jeder Thread sammelt in einen lokalen Puffer (`Buf`), der alle 512 Einträge unter
   einem `Mutex` in den gemeinsamen `Acc` schreibt — das hält Lock-Zeit minimal.
3. Alle ~220 ms darf genau ein Thread ein `Progress`-Event senden (Gate über
   `try_lock` + Zeitstempel), das die aktuellen Zähler inklusive Pufferstand meldet.
4. Am Ende aggregiert `categories.rs` die Einträge zu `ScanSummary` und die Daten
   landen in `AppState { entries, summary, scan_root }`; das Frontend bekommt
   zusätzlich `df://scan-done`.
5. Abbrechen setzt `AtomicBool`; die Visitor-Closures geben `WalkState::Quit` zurück.

## Duplikaterkennung (`hasher.rs`)

1. `HashMap<u64 /*size*/, Vec<usize>>` über alle Einträge → nur Größen mit ≥ 2 Dateien.
2. Kandidaten mit `rayon` parallel hashen:
   - Datei ≤ 50 MB: BLAKE3 über den ganzen Inhalt.
   - Datei > 50 MB: zuerst Kopf 64 KB + Ende 64 KB + Länge; nur bei Gleichheit volle Datei.
3. Gruppen mit ≥ 2 identischen Hashes werden nach eingesparten Bytes sortiert.
4. Fortschritt über `doneBytes / totalBytes` → determinierter Fortschrittsbalken mit ETA.

## Sortieren (`mover.rs`)

```
plan  ──► für jede Datei: Zielpfad berechnen (Struktur + Kategorie + Endung)
      ──► Namenskonflikt? → unique_target() zählt hoch (Set der geplanten Pfade)
      ──► Duplikat-Richtlinie anwenden (skip | _Duplikate | delete)
      ──► optional: dry_run = true → nur LogBatch mit action="planned"
execute ─► rename()  ──EXDEV──► copy + BLAKE3-Verify + remove_file
        ─► mtime via filetime übernehmen
        ─► LogBatch in move_log.json (max. 30), Batch-ID = Zeit+PID+Sequenz
undo    ─► Ops rückwärts: moved → zurück-rennen, copied → Datei löschen + Original zurück,
           deleted → aus _Duplikate zurück (sofern vorhanden), Batch als undone markieren
```

Sicherheitsregeln: nie überschreiben, nie löschen ohne verifizierte Kopie,
Fehler pro Datei sammeln statt abzubrechen.

## Zustand und Persistenz

| Datei | Inhalt | Limit |
|---|---|---|
| `~/.config/diskforge/categories.json` | Kategorien + Endungen | – |
| `~/.config/diskforge/move_log.json` | Sortierprotokoll für Undo | 30 Batches |

`DISKFORGE_HOME` überschreibt das Konfigurationsverzeichnis — so laufen Tests
isoliert und schreiben nie ins echte Profil.

## Nebenläufigkeit

| Stelle | Mechanismus |
|---|---|
| Scan | `ignore`-Walker, N Threads, `Mutex<Acc>` mit Puffer-Flush |
| Hashing | `rayon` über Kandidaten, `blake3` mit `rayon`-Feature |
| Tasks | `tauri::async_runtime::spawn_blocking`, `AtomicBool` als Cancel-Flag |
| Busy-Schutz | `AppState.busy` verhindert parallele Scans/Sortierläufe |

## Tauri-Konfiguration

- `assetProtocol` mit Scope auf `$HOME/**`, `/home/**`, `/run/media/**`, `/media/**`, `/mnt/**`
  → Vorschauen funktionieren nur für lokale, eingebundene Pfade.
- CSP ohne Inline-JS, `img-src`/`media-src` erlauben `asset:` und `blob:`.
- Fenster 1520 × 940, Mindestgröße 1120 × 700, Theme `Dark`, Hintergrund `#0b0e14`.

## Bekannte Stolperfallen

- `class:`-Direktiven in Svelte vertragen keine Tailwind-Varianten → Hilfsklassen.
- `structuredClone` auf `$state`-Proxies wirft `DataCloneError` → `$state.snapshot()`.
- Byte-Slicing auf Dateinamen (`name[..n]`) bricht bei UTF-8 → `as_bytes()` verwenden.
- `rename` über Gerätegrenzen liefert `EXDEV` → Kopier-Fallback nötig.
- Bei Tests immer `DISKFORGE_HOME` setzen, sonst landet das Log im echten Profil.
