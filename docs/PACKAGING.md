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

## 6. AUR-Regeln, die wir einhalten

- `pkgname` = `fylr-bin`, `provides`/`conflicts` gesetzt, damit sich `-bin`, `fylr` und
  `fylr-git` nicht gleichzeitig installieren.
- Laufzeitabhängigkeiten explizit: `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`,
  `librsvg`; `ffmpeg` und `ntfs-3g` nur als `optdepends`.
- Keine Binärdateien im AUR-Repo — nur `PKGBUILD` und `.SRCINFO`.
- Lizenz MIT, `license=('MIT')`.
- `options=('!strip')`, weil das Tauri-Binary bereits gestrippt ist.
