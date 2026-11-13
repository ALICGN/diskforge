export function formatBytes(bytes, digits = 1) {
  if (bytes === null || bytes === undefined || Number.isNaN(bytes)) return "–";
  const b = Number(bytes);
  if (b < 1000) return `${b} B`;
  const units = ["KB", "MB", "GB", "TB", "PB"];
  let v = b / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 ? 0 : digits)} ${units[i]}`;
}

export function formatCount(n) {
  if (n === null || n === undefined) return "–";
  return new Intl.NumberFormat("de-DE").format(n);
}

export function formatDuration(ms) {
  if (!ms || ms < 0) return "–";
  const s = ms / 1000;
  if (s < 1) return `${Math.round(ms)} ms`;
  if (s < 60) return `${s.toFixed(s < 10 ? 2 : 1)} s`;
  const m = Math.floor(s / 60);
  const rest = Math.round(s % 60);
  if (m < 60) return `${m} min ${rest} s`;
  const h = Math.floor(m / 60);
  return `${h} h ${m % 60} min`;
}

export function formatDate(ms) {
  if (!ms) return null;
  const d = new Date(Number(ms));
  if (Number.isNaN(d.getTime())) return null;
  return d.toLocaleString("de-DE", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function formatRate(v) {
  if (!v || v <= 0) return "–";
  if (v > 1024 * 1024 * 1024) return `${(v / 1024 / 1024 / 1024).toFixed(2)} GB/s`;
  if (v > 1024 * 1024) return `${(v / 1024 / 1024).toFixed(1)} MB/s`;
  if (v > 1024) return `${(v / 1024).toFixed(0)} KB/s`;
  return `${v.toFixed(0)} B/s`;
}

export function pct(part, total) {
  if (!total) return 0;
  return Math.min(100, Math.max(0, (Number(part) / Number(total)) * 100));
}

export function basename(path) {
  if (!path) return "";
  const idx = path.lastIndexOf("/");
  return idx >= 0 ? path.slice(idx + 1) : path;
}

export function dirname(path) {
  if (!path) return "";
  const idx = path.lastIndexOf("/");
  return idx > 0 ? path.slice(0, idx) : "/";
}

export function extLabel(ext) {
  if (!ext) return "Ohne Endung";
  return ext.toUpperCase();
}
