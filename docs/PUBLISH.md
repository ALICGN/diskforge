# Veröffentlichung auf GitHub

Interne Checkliste für den ersten Upload und die Repo-Einstellungen.

## 1. Repo anlegen und pushen

```bash
gh auth login                     # GitHub.com → HTTPS → Login via Browser
cd ~/projekte/diskforge
gh repo create diskforge --public --source=. --remote=origin \
  --description "Schneller Disk-Analyzer, Duplikatfinder (BLAKE3) und Dateisortierer für Linux – Tauri 2, Rust, Svelte 5" \
  --push
```

Ohne `gh`:

```bash
git remote add origin git@github.com:OWNER/diskforge.git
git push -u origin main
```

**Vor dem Push prüfen:** `OWNER` in `README.md`, `CONTRIBUTING.md`,
`package.json`, `src-tauri/Cargo.toml` und `.github/ISSUE_TEMPLATE/config.yml`
durch den echten GitHub-Benutzernamen ersetzen:

```bash
git grep -l OWNER | xargs sed -i 's/OWNER/DEINNAME/g'
git commit -am "chore: Repository-URLs auf eigenen Account setzen"
```

## 2. Repo-Einstellungen (Web-UI)

**About-Bereich (rechts oben):**
- Description: *Schneller Disk-Analyzer, Duplikatfinder (BLAKE3) und Dateisortierer für Linux – Tauri 2, Rust, Svelte 5*
- Website: leer lassen
- Topics: `tauri` `rust` `svelte` `svelte5` `tailwindcss` `disk-analyzer` `duplicate-finder`
  `file-organizer` `blake3` `linux` `archlinux` `cachyos` `ntfs` `kde`
- ☑ Releases, ☑ Packages, ☑ Discussions

**Settings → General:**
- Features: Issues ☑, Discussions ☑, Projects optional, Wiki ☐ (Doku liegt im Repo)
- Pull Requests: ☑ Allow squash merging, ☑ Automatically delete head branches
- Merge button: „Allow merge commits" und „Allow rebase merging" können aus bleiben
- Default branch: `main`

**Settings → Rules → Branches:** Rule für `main`
- ☑ Require a pull request before merging (1 Approval)
- ☑ Require status checks: `Frontend (Svelte + Vite)`, `Backend (Rust)`
- ☑ Require conversation resolution
- ☑ Do not allow bypassing (auch für Admins sinnvoll, wenn du sauber arbeiten willst)

**Settings → Security:**
- ☑ Private vulnerability reporting (SECURITY.md verweist darauf)
- ☑ Dependabot alerts + security updates
- ☑ Dependabot version updates (`.github/dependabot.yml` ist vorbereitet)

## 3. Labels anlegen

```bash
gh label create "good first issue" -c 7057ff -d "Guter Einstieg für neue Beitragende"
gh label create "help wanted"      -c 008672 -d "Hier ist Unterstützung willkommen"
gh label create "triage"           -c ededed -d "Noch nicht eingeordnet"
gh label create "scan"             -c 1d76db -d "Scan / Analyse"
gh label create "duplicates"       -c 5319e7 -d "Duplikaterkennung"
gh label create "sorting"          -c 0e8a16 -d "Sortieren / Undo"
gh label create "ui"               -c d4c5f9 -d "Oberfläche"
gh label create "performance"      -c fbca04 -d "Geschwindigkeit, Speicher"
```

## 4. Erstes Release

```bash
git tag v0.1.0 -m "DiskForge 0.1.0"
git push origin v0.1.0        # löst .github/workflows/release.yml aus
```

Der Workflow baut `.deb`, `.rpm` und `.AppImage` und legt einen **Entwurf** an —
dort Release-Notes aus `CHANGELOG.md` einsetzen und veröffentlichen.

## 5. Danach

- Repository auf [GitHub Social Preview](https://github.com/OWNER/diskforge/settings)
  setzen: `docs/screenshots/overview.png` (1280 × 640 empfohlen).
- In `README.md` die Roadmap-Punkte pflegen, sobald Issues existieren.
- Screenshots neu erzeugen: `./scripts/make-demo-data.sh`, App starten,
  `magick <shot>.png -crop 1520x940+40+30 +repage -resize 1400x docs/screenshots/<name>.png`
