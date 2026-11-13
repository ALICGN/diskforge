# Veröffentlichung im AUR

Fylr wird als **`fylr-bin`** in den AUR gestellt (fertiges Paket aus dem GitHub-Release)
und optional zusätzlich als **`fylr`** (Quellbau mit mitgeliefertem Crate-Archiv).

## 1. Voraussetzungen (einmalig)

1. Account auf <https://aur.archlinux.org> anlegen und dort unter *My Account* einen
   SSH-Public-Key eintragen. Der Key für dieses Projekt liegt unter `~/.ssh/aur_fylr.pub`:

   ```bash
   cat ~/.ssh/aur_fylr.pub
   ```

2. `base-devel` und `git` müssen installiert sein:
   ```bash
   sudo pacman -S --needed base-devel git
   ```

## 2. Paket lokal bauen und prüfen

```bash
git clone ssh://aur@aur.archlinux.org/fylr-bin.git   # legt das AUR-Repo an
cd fylr-bin
makepkg -si            # baut und installiert
fylr                   # startet die App
```

Prüfen, was im Paket landet:

```bash
makepkg -f
bsdtar -tf fylr-bin-*.pkg.tar.zst | head
namcap fylr-bin-*.pkg.tar.zst
```

## 3. `.SRCINFO` erzeugen und hochladen

```bash
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "fylr-bin 0.2.0-1"
git push
```

Der AUR zeigt das Paket danach unter <https://aur.archlinux.org/packages/fylr-bin>.

**Bei jeder neuen Version:** `pkgver` erhöhen, `pkgrel=1` setzen, Release-Asset-URL prüfen,
dann erneut `makepkg --printsrcinfo > .SRCINFO` und pushen. Pakete werden nicht neu gebaut,
solange `pkgver-pkgrel` gleich bleibt.

## 4. Dateien in diesem Repo

| Pfad | Zweck |
|---|---|
| `packaging/aur/fylr-bin/PKGBUILD` | Fertiges Paket aus `Fylr_<ver>_amd64.deb` (GitHub-Release) |
| `packaging/aur/fylr/PKGBUILD` | Quellbau ab Tag-Tarball + `fylr-<ver>-vendor.tar.gz` |

## 4b. Stand der Veröffentlichung

- `fylr-bin` **0.2.0-1**: PKGBUILD lokal gebaut und geprüft (enthält
  `/usr/bin/fylr`, `fylr.desktop`, Icons; Quellen: Release-Deb, sha256 fixiert).
  Fertig zum Push ins AUR-Repo.
- `fylr` (Quellbau): PKGBUILD liegt bereit, **noch nicht gebaut** — benötigt erst das
  Vendor-Archiv als Release-Asset (Abschnitt 5). Bis dahin nur `fylr-bin` anbieten.

## 5. Vendor-Archiv für den Quellbau

Der Quellbau braucht alle Crates offline. Das Archiv wird einmal pro Version erzeugt
und als Release-Asset hochgeladen:

```bash
cd src-tauri && cargo vendor --versioned-dirs vendor
cd .. && tar -czf /tmp/fylr-0.2.0-vendor.tar.gz -C src-tauri vendor
gh release upload v0.2.0 /tmp/fylr-0.2.0-vendor.tar.gz
```

Ohne dieses Asset schlägt `fylr` fehl — dann ausschließlich `fylr-bin` anbieten.

## 5b. Eigenes pacman-Repository (sofort nutzbar, unabhängig vom AUR)

Solange die AUR-Registrierung geschlossen ist, liefert GitHub Pages das Paket direkt aus:

```
https://alicgn.github.io/fylr/x86_64
```

Client-Einrichtung:

```ini
# /etc/pacman.conf  – ganz unten anhängen
[fylr]
SigLevel = Optional TrustAll
Server = https://alicgn.github.io/fylr/x86_64
```

```bash
sudo pacman -Sy
sudo pacman -S fylr-bin
```

**Neue Version veröffentlichen:**

```bash
# 1. pkgver in packaging/aur/fylr-bin/PKGBUILD erhöhen, Release + Deb hochladen
# 2. sha256 des neuen Deb einsetzen
sha256sum Fylr_<ver>_amd64.deb
# 3. Paket bauen und Repo-DB neu erzeugen
./packaging/repo/build-repo.sh
# 4. auf den gh-pages-Zweig schieben
cd repo && cp -r x86_64 /tmp/pages-stage/ 2>/dev/null || true
```

Konkret für den Upload:

```bash
git worktree add /tmp/fylr-pages gh-pages
cp repo/x86_64/* /tmp/fylr-pages/x86_64/
cd /tmp/fylr-pages && git add -A && git commit -m "repo: fylr-bin <ver>" && git push
cd - && git worktree remove /tmp/fylr-pages
```

Das Repository ist **nicht signiert** (`SigLevel = Optional TrustAll` nötig). Wer das nicht
will, installiert die Pakete aus den GitHub-Releases oder baut selbst (`fylr`-PKGBUILD).

## 6. AUR-Regeln, die wir einhalten

- `pkgname` = `fylr-bin`, `provides`/`conflicts` gesetzt, damit sich `-bin`, `fylr` und
  `fylr-git` nicht gleichzeitig installieren.
- Laufzeitabhängigkeiten explizit: `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`,
  `librsvg`; `ffmpeg` und `ntfs-3g` nur als `optdepends`.
- Keine Binärdateien im AUR-Repo — nur `PKGBUILD` und `.SRCINFO`.
- Lizenz MIT, `license=('MIT')`.
- `options=('!strip')`, weil das Tauri-Binary bereits gestrippt ist.
