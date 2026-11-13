import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

let toastId = 0;

class AppStore {
  /* ---------- global ---------- */
  tab = $state("overview");
  ready = $state(false);
  busy = $state(false);
  progress = $state(null);
  summary = $state(null);
  mounts = $state([]);
  categories = $state({ version: 1, categories: [] });
  dups = $state([]);
  logs = $state([]);
  sys = $state({ threads: 0, version: "", ffprobe: false, platform: "" });
  toasts = $state([]);

  /* ---------- scan form ---------- */
  root = $state("");
  scanOptions = $state({
    recursive: true,
    honorIgnoreList: true,
    followSymlinks: false,
    extraIgnores: [],
  });

  /* ---------- selection (ext / category filters for sorting) ---------- */
  selExts = $state([]);
  selCats = $state([]);
  viewMode = $state("categories"); // categories | extensions
  openCats = $state({});

  /* ---------- duplicates ---------- */
  dupMarks = $state({}); // path -> true  => wird aussortiert
  dupMinMb = $state(1);

  /* ---------- sorting ---------- */
  sort = $state({
    target: "",
    structure: "categoryExt",
    duplicatePolicy: "skip",
    dryRun: true,
    preserveMtime: true,
  });
  lastPlan = $state(null);
  lastSort = $state(null);
  sortOpen = $state(false);

  /* ---------- derived ---------- */
  get fileCount() {
    return this.summary?.files ?? 0;
  }
  get selectedExtCount() {
    return this.selExts.length;
  }
  get dupFileCount() {
    return Object.keys(this.dupMarks).length;
  }
  get dupWastedBytes() {
    let total = 0;
    for (const g of this.dups) {
      for (const f of g.files) if (this.dupMarks[f.path]) total += f.size;
    }
    return total;
  }
  get dupGroupWasted() {
    let total = 0;
    for (const g of this.dups) if (g.files.length > 1) total += g.size * (g.files.length - 1);
    return total;
  }

  /* ---------- infra ---------- */
  toast(message, kind = "info", timeout = 5200) {
    const id = ++toastId;
    this.toasts = [...this.toasts, { id, message, kind }];
    if (timeout) setTimeout(() => this.dismiss(id), timeout);
  }
  dismiss(id) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  assetUrl(path) {
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }

  async init() {
    if (this.ready) return;
    await listen("df://progress", (e) => {
      this.progress = e.payload;
      this.busy = e.payload.phase !== "done";
    });
    await listen("df://scan-done", (e) => {
      this.summary = e.payload;
      this.dupMarks = {};
      this.dups = [];
      this.lastPlan = null;
    });
    await listen("df://dups-done", (e) => {
      this.dups = e.payload;
    });
    await listen("df://sort-done", (e) => {
      this.lastSort = e.payload;
    });

    try {
      const [mounts, cats, logs, sys] = await Promise.all([
        invoke("list_mounts"),
        invoke("get_categories"),
        invoke("get_log"),
        invoke("system_info"),
      ]);
      this.mounts = mounts;
      this.categories = cats;
      this.logs = logs;
      this.sys = sys;
      const home = mounts.find((m) => m.path === "/home");
      if (home) this.root = "";
    } catch (err) {
      this.toast(String(err), "error");
    }
    this.ready = true;
  }

  /* ---------- scan ---------- */
  async pickDirectory(title = "Quellordner wählen") {
    try {
      const path = await invoke("pick_directory", { title });
      if (path) return path;
    } catch (err) {
      this.toast(String(err), "error");
    }
    return null;
  }

  async chooseSource() {
    const path = await this.pickDirectory("Quellordner wählen");
    if (path) this.root = path;
  }

  async startScan() {
    if (!this.root) {
      this.toast("Bitte zuerst einen Ordner oder ein Laufwerk wählen.", "warn");
      return null;
    }
    try {
      this.busy = true;
      this.progress = null;
      this.summary = null;
      const summary = await invoke("start_scan", {
        root: this.root,
        options: {
          recursive: this.scanOptions.recursive,
          honorIgnoreList: this.scanOptions.honorIgnoreList,
          followSymlinks: this.scanOptions.followSymlinks,
          extraIgnores: this.extraIgnoresList(),
        },
      });
      this.summary = summary;
      this.progress = null;
      this.openCats = {};
      this.selExts = [];
      this.selCats = [];
      this.toast(
        `${summary.files.toLocaleString("de-DE")} Dateien in ${(summary.durationMs / 1000).toFixed(2)} s erfasst`,
        summary.cancelled ? "warn" : "ok",
      );
      return summary;
    } catch (err) {
      this.toast(String(err), "error");
      return null;
    } finally {
      this.busy = false;
    }
  }

  extraIgnoresList() {
    const raw = this.scanOptions.extraIgnores;
    if (Array.isArray(raw)) return raw;
    return String(raw || "")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
  }

  async cancel() {
    await invoke("cancel_task").catch(() => {});
  }

  /* ---------- categories ---------- */
  async saveCategories(list) {
    try {
      this.categories = await invoke("save_categories", {
        config: { version: this.categories.version || 1, categories: list },
      });
      this.toast("Kategorie-Regeln gespeichert", "ok");
      return this.categories;
    } catch (err) {
      this.toast(String(err), "error");
      return null;
    }
  }

  async resetCategories() {
    try {
      this.categories = await invoke("reset_categories");
      this.toast("Standard-Kategorien wiederhergestellt", "ok");
      return this.categories;
    } catch (err) {
      this.toast(String(err), "error");
      return null;
    }
  }

  /* ---------- duplicates ---------- */
  async findDuplicates() {
    try {
      this.busy = true;
      this.progress = null;
      const min = Math.max(1, Math.round(Number(this.dupMinMb || 0) * 1024 * 1024));
      const groups = await invoke("find_duplicates", { minSize: min });
      this.progress = null;
      this.dups = groups;
      const files = groups.reduce((a, g) => a + g.files.length, 0);
      this.toast(
        groups.length
          ? `${groups.length} Gruppen mit ${files} Dateien gefunden`
          : "Keine Duplikate gefunden",
        groups.length ? "ok" : "info",
      );
      return groups;
    } catch (err) {
      this.progress = null;
      this.toast(String(err), "error");
      return [];
    } finally {
      this.busy = false;
    }
  }

  markDuplicate(path, on = true) {
    const next = { ...this.dupMarks };
    if (on) next[path] = true;
    else delete next[path];
    this.dupMarks = next;
  }

  markGroup(fileList, keepPath = null) {
    const next = { ...this.dupMarks };
    for (const f of fileList) {
      if (keepPath && f.path === keepPath) delete next[f.path];
      else next[f.path] = true;
    }
    this.dupMarks = next;
  }

  clearGroup(fileList) {
    const next = { ...this.dupMarks };
    for (const f of fileList) delete next[f.path];
    this.dupMarks = next;
  }

  clearAllMarks() {
    this.dupMarks = {};
  }

  dupPaths() {
    return Object.keys(this.dupMarks);
  }

  /* ---------- sorting ---------- */
  async buildPlan() {
    try {
      this.busy = true;
      const batch = await invoke("plan_sort", { request: this.sortRequest(true) });
      this.lastPlan = batch;
      return batch;
    } catch (err) {
      this.toast(String(err), "error");
      return null;
    } finally {
      this.busy = false;
    }
  }

  async runSort() {
    try {
      this.busy = true;
      this.progress = null;
      const batch = await invoke("execute_sort", { request: this.sortRequest(false) });
      this.progress = null;
      this.lastSort = batch;
      // Bei einer Simulation bleibt der Dialog offen, damit die geplanten
      // Zielpfade sichtbar sind - geschlossen wird nur nach echtem Sortieren.
      this.sortOpen = !!batch.dryRun;
      if (!batch.dryRun) {
        this.logs = await invoke("get_log");
        const summary = await invoke("get_summary");
        if (summary) this.summary = summary;
      }
      this.toast(
        batch.dryRun
          ? `Simulation: ${batch.ops.length.toLocaleString("de-DE")} Aktionen geplant, ${batch.skipped} übersprungen, ${batch.errors} Fehler`
          : `${batch.moved + batch.copied} Dateien sortiert (${batch.errors} Fehler)`,
        batch.errors ? "warn" : "ok",
        7000,
      );
      return batch;
    } catch (err) {
      this.toast(String(err), "error");
      return null;
    } finally {
      this.busy = false;
    }
  }

  sortRequest(forceDry = false) {
    const exts = this.selExts.length ? this.selExts : null;
    const cats = this.selCats.length ? this.selCats : null;
    return {
      target: this.sort.target,
      structure: this.sort.structure,
      dryRun: forceDry ? true : this.sort.dryRun,
      duplicatePolicy: this.sort.duplicatePolicy,
      duplicatePaths: this.dupPaths(),
      selectedExts: exts,
      selectedCategories: cats,
      preserveMtime: this.sort.preserveMtime,
    };
  }

  async refreshLog() {
    try {
      this.logs = await invoke("get_log");
    } catch (err) {
      this.toast(String(err), "error");
    }
  }

  async undoLast() {
    try {
      this.busy = true;
      const res = await invoke("undo_last");
      await this.refreshLog();
      this.toast(
        `${res.restored} Dateien zurückverschoben${res.unrestorable ? `, ${res.unrestorable} gelöscht (nicht rückholbar)` : ""}`,
        res.failed.length ? "warn" : "ok",
        8000,
      );
      return res;
    } catch (err) {
      this.toast(String(err), "error");
      return null;
    } finally {
      this.busy = false;
    }
  }

  async undoBatch(id) {
    try {
      this.busy = true;
      const res = await invoke("undo_batch", { batchId: id });
      await this.refreshLog();
      this.toast(`${res.restored} Dateien zurückverschoben`, res.failed.length ? "warn" : "ok");
      return res;
    } catch (err) {
      this.toast(String(err), "error");
      return null;
    } finally {
      this.busy = false;
    }
  }

  async reveal(path) {
    await invoke("reveal_path", { path }).catch(() => {});
  }

  /* ---------- selection helpers ---------- */
  toggleExt(ext) {
    this.selExts = this.selExts.includes(ext)
      ? this.selExts.filter((e) => e !== ext)
      : [...this.selExts, ext];
  }
  toggleCat(name) {
    this.selCats = this.selCats.includes(name)
      ? this.selCats.filter((c) => c !== name)
      : [...this.selCats, name];
  }
  clearSelection() {
    this.selExts = [];
    this.selCats = [];
  }
  selectAllFrom(summary) {
    this.selExts = [];
    this.selCats = summary.categories.map((c) => c.name);
  }
  selectedStats() {
    if (!this.summary) return { files: 0, bytes: 0 };
    if (!this.selExts.length && !this.selCats.length) {
      return { files: this.summary.files, bytes: this.summary.bytes };
    }
    let files = 0;
    let bytes = 0;
    for (const cat of this.summary.categories) {
      if (this.selCats.includes(cat.name)) {
        files += cat.count;
        bytes += cat.size;
        continue;
      }
      for (const e of cat.exts) {
        const key = e.ext.toLowerCase();
        if (this.selExts.includes(key) || this.selExts.includes(e.ext)) {
          files += e.count;
          bytes += e.size;
        }
      }
    }
    return { files, bytes };
  }
}

export const app = new AppStore();
