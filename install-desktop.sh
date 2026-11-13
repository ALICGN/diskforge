#!/usr/bin/env bash
# Bindet DiskForge ins KDE-/CachyOS-Anwendungsmenü ein (nur für den aktuellen Nutzer).
set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$SRC/src-tauri/target/release/diskforge"
ICONS="$SRC/src-tauri/icons"

[[ -x "$BIN" ]] || { echo "Release-Binary fehlt: $BIN (erst: npx tauri build --no-bundle)"; exit 1; }

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_ROOT="$HOME/.local/share/icons/hicolor"

mkdir -p "$BIN_DIR" "$APP_DIR"

install -m 755 "$BIN" "$BIN_DIR/diskforge"

for size in 32 64 128 256 512; do
  dir="$ICON_ROOT/${size}x${size}/apps"
  mkdir -p "$dir"
  case "$size" in
    32)  src="$ICONS/32x32.png" ;;
    64)  src="$ICONS/64x64.png" ;;
    128) src="$ICONS/128x128.png" ;;
    256) src="$ICONS/128x128@2x.png" ;;
    512) src="$ICONS/icon.png" ;;
  esac
  if [[ -f "$src" ]]; then
    magick "$src" -resize "${size}x${size}" "$dir/diskforge.png"
  fi
done
# SVG für Skalierung / HiDPI
mkdir -p "$ICON_ROOT/scalable/apps"
[[ -f "$ICONS/icon.svg" ]] && install -m 644 "$ICONS/icon.svg" "$ICON_ROOT/scalable/apps/diskforge.svg"

cat > "$APP_DIR/diskforge.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=DiskForge
GenericName=Festplatten-Analyse & Sortierung
Comment=Laufwerke scannen, Duplikate per BLAKE3 finden und Dateien sortieren
Comment[de]=Laufwerke scannen, Duplikate per BLAKE3 finden und Dateien sortieren
Exec=${BIN_DIR}/diskforge
Icon=diskforge
Terminal=false
Categories=Utility;FileTools;System;KDE;
Keywords=Disk;Duplikate;Duplicates;Sortieren;Scanner;NTFS;Festplatte;Aufräumen;
Keywords[de]=Festplatte;Duplikate;Sortieren;Scanner;NTFS;Aufräumen;Speicherplatz;
StartupNotify=true
StartupWMClass=diskforge
X-GNOME-UsesNotifications=true
EOF

chmod 644 "$APP_DIR/diskforge.desktop"

command -v update-desktop-database >/dev/null && update-desktop-database "$APP_DIR" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -f -t "$ICON_ROOT" >/dev/null 2>&1 || true
# KDE-Menü-Cache neu aufbauen (KDE 5/6)
for kb in kbuildsycoca6 kbuildsycoca5; do
  command -v "$kb" >/dev/null && "$kb" --noincremental >/dev/null 2>&1 && break
done

if command -v desktop-file-validate >/dev/null; then
  desktop-file-validate "$APP_DIR/diskforge.desktop" && echo "desktop-file-validate: ok"
fi

echo "Installiert:"
echo "  $BIN_DIR/diskforge"
echo "  $APP_DIR/diskforge.desktop"
echo "  Icons in $ICON_ROOT"
