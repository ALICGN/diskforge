#!/usr/bin/env bash
# Erzeugt einen neutralen Demo-Datensatz für Screenshots/Doku (keine echten Daten).
set -euo pipefail
ROOT="$HOME/diskforge-demo"
rm -rf "$ROOT"
mkdir -p "$ROOT"/{quell/{Bilder,Videos,Audio,Dokumente,Archive,Skripte,Diverses},ziel}
cd "$ROOT/quell"

# --- Bilder: Farbverläufe mit Beschriftung -----------------------------------
for i in $(seq -w 1 14); do
  case $((10#$i % 4)) in
    0) c1="#1e3a8a"; c2="#38bdf8" ;;
    1) c1="#312e81"; c2="#a78bfa" ;;
    2) c1="#0f766e"; c2="#4ade80" ;;
    *) c1="#7c2d12"; c2="#fbbf24" ;;
  esac
  magick -size 1600x1000 gradient:"$c1-$c2" -gravity center \
    -pointsize 90 -fill white -annotate 0 "Motiv $i" "Bilder/urlaub-$i.jpg"
done
magick -size 512x512 radial-gradient:'#38bdf8-#0b0e14' "Bilder/logo.png"
magick -size 800x800 xc:'#0b0e14' -fill '#4f8cff' -draw "circle 400,400 400,120" "Bilder/icon.png"
magick -size 1200x800 gradient:'#0b0e14-#4f8cff' -quality 92 "Bilder/panorama.jpg"
cp "Bilder/urlaub-03.jpg" "Bilder/urlaub-03-kopie.jpg"
cp "Bilder/logo.png" "Bilder/logo-kopie.png"

# --- Videos / Audio ----------------------------------------------------------
ffmpeg -y -f lavfi -i testsrc=size=1280x720:rate=25:duration=3 -pix_fmt yuv420p \
  -c:v libx264 -preset veryfast -crf 30 "Videos/clip-01.mp4" >/dev/null 2>&1
ffmpeg -y -f lavfi -i smptebars=size=1920x1080:rate=25:duration=2 -pix_fmt yuv420p \
  -c:v libx264 -preset veryfast -crf 30 "Videos/clip-02.mp4" >/dev/null 2>&1
ffmpeg -y -f lavfi -i "sine=frequency=440:duration=4" -c:a libmp3lame -q:a 6 "Audio/track-01.mp3" >/dev/null 2>&1
ffmpeg -y -f lavfi -i "sine=frequency=220:duration=3" -c:a flac "Audio/track-02.flac" >/dev/null 2>&1

# --- Dokumente ---------------------------------------------------------------
printf '%%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\ntrailer<</Root 1 0 R>>\n%%%%EOF\n' > "Dokumente/rechnung-2024.pdf"
cp "Dokumente/rechnung-2024.pdf" "Dokumente/rechnung-2024-kopie.pdf"
cat > "Dokumente/notizen.md" <<'MD'
# Notizen

- Scan laeuft parallel ueber alle Kerne
- Duplikate werden per BLAKE3 verglichen
- Sortieren nach Kategorie und Endung
MD
printf 'datum;kunde;betrag\n2024-01-02;Beispiel GmbH;199,00\n2024-02-11;Muster AG;49,90\n' > "Dokumente/tabelle.csv"
printf 'Hallo Welt\n' > "Dokumente/schreiben.txt"

# --- Archive -----------------------------------------------------------------
rm -f "Archive/bilder-export.zip"
(cd Bilder && python3 -c "
import zipfile, pathlib
with zipfile.ZipFile('../Archive/bilder-export.zip', 'w', zipfile.ZIP_DEFLATED) as z:
    for f in sorted(pathlib.Path('.').glob('*')):
        if f.is_file():
            z.write(f, f.name)
")
tar -czf "Archive/projekt-sicherung.tar.gz" -C "$ROOT/quell" Dokumente Skripte
tar --zstd -cf "Archive/projekt-sicherung.tar.zst" -C "$ROOT/quell" Dokumente

# --- Skripte / Diverses ------------------------------------------------------
printf '#!/usr/bin/env bash\nset -euo pipefail\necho "build"\n' > "Skripte/build.sh"
chmod +x "Skripte/build.sh"
printf 'fn main() {\n    println!("Hallo");\n}\n' > "Skripte/main.rs"
printf 'export const version = "0.1.0";\n' > "Skripte/app.js"
printf '{"name":"demo","version":"1.0.0"}\n' > "Skripte/paket.json"
printf 'Dies ist eine Datei ohne Endung.\n' > "Diverses/ohne_endung"
printf 'LIESMICH\n' > "Diverses/LIESMICH"
head -c 200000 /dev/urandom > "Diverses/rohdaten.bin"

echo "Demo-Datensatz:"
du -sh "$ROOT/quell"
find "$ROOT/quell" -type f | wc -l
