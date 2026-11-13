<script>
  import { app } from "../store.svelte.js";
  import { formatBytes, formatCount, formatDuration } from "../format.js";
  import CategoryTree from "../components/CategoryTree.svelte";
  import ProgressPanel from "../components/ProgressPanel.svelte";
  import {
    HardDrive,
    FolderOpen,
    Play,
    Search,
    List,
    FolderTree,
    Zap,
    Clock,
    Files,
    AlertTriangle,
    ShieldAlert,
    Gauge,
    ArrowDownUp,
    ChevronDown,
    Lock,
    RotateCcw,
    CircleCheck,
  } from "lucide-svelte";

  let filter = $state("");
  let showMounts = $state(true);

  let summary = $derived(app.summary);
  let sel = $derived(app.selectedStats());
  let scanned = $derived(!!summary);

  function useMount(m) {
    app.root = m.path;
  }

  let sortedMounts = $derived(
    [...app.mounts].sort((a, b) => {
      const ra = a.path.startsWith("/run/media") ? 0 : a.path === "/" ? 2 : 1;
      const rb = b.path.startsWith("/run/media") ? 0 : b.path === "/" ? 2 : 1;
      return ra - rb;
    }),
  );
</script>

<div class="flex h-full flex-col overflow-hidden">
  <div class="flex-1 overflow-y-auto">
    <div class="mx-auto flex max-w-[1500px] flex-col gap-3 p-4">
      <!-- Quelle -->
      <div class="card p-3.5">
        <div class="mb-2.5 flex items-center gap-2">
          <HardDrive class="h-4 w-4 text-accent" />
          <h2 class="text-[13px] font-semibold">Quelle</h2>
          <span class="text-[11px] text-muted">
            Laufwerk oder Ordner — auch NTFS/externe Datenträger
          </span>
          <button class="btn btn-ghost btn-sm ml-auto" onclick={() => (showMounts = !showMounts)}>
            Laufwerke {showMounts ? "ausblenden" : "anzeigen"}
            <ChevronDown
              class="h-3 w-3 transition-transform"
              style="transform: rotate({showMounts ? 180 : 0}deg)"
            />
          </button>
        </div>

        {#if showMounts}
          <div class="mb-3 grid grid-cols-2 gap-2 xl:grid-cols-3">
            {#each sortedMounts as m (m.path)}
              <button
                class="group flex flex-col gap-1.5 rounded-lg border px-3 py-2 text-left transition-colors"
                class:border-accent={app.root === m.path}
                class:sel-card={app.root === m.path}
                class:border-line={app.root !== m.path}
                class:bg-panel2={app.root !== m.path}
                onclick={() => useMount(m)}
                title={m.source}
              >
                <div class="flex w-full items-center gap-2">
                  <HardDrive class="h-3.5 w-3.5 text-muted group-hover:text-accent" />
                  <span class="truncate text-[12px] font-medium">{m.label || m.path}</span>
                  {#if m.readonly}
                    <span class="badge badge-warn"><Lock class="h-2.5 w-2.5" /> read-only</span>
                  {/if}
                  <span class="ml-auto mono text-[10px] text-muted">{m.fs}</span>
                </div>
                <div class="mono truncate text-[10px] text-[#5f6d84]">{m.path}</div>
                <div class="h-1 w-full overflow-hidden rounded-full bg-panel3">
                  <div
                    class="h-full rounded-full bg-gradient-to-r from-accent to-cyan"
                    style="width:{m.total ? Math.min(100, (m.used / m.total) * 100) : 0}%"
                  ></div>
                </div>
                <div class="flex justify-between text-[10px] text-muted num">
                  <span>{formatBytes(m.free)} frei</span>
                  <span>{formatBytes(m.total)}</span>
                </div>
              </button>
            {/each}
          </div>
        {/if}

        <div class="flex flex-wrap items-center gap-2">
          <div class="relative min-w-[280px] flex-1">
            <FolderOpen class="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted" />
            <input
              class="input mono pl-8"
              placeholder="/home/ali/Bilder oder /run/media/ali/ExterneHD"
              bind:value={app.root}
              onkeydown={(e) => e.key === "Enter" && !app.busy && app.startScan()}
            />
          </div>
          <button class="btn" onclick={() => app.chooseSource()} disabled={app.busy}>
            <FolderOpen class="h-3.5 w-3.5" /> Ordner wählen
          </button>
          <button class="btn btn-ghost" onclick={() => (app.root = "/")} disabled={app.busy}>
            /
          </button>
          <div class="mx-1 h-6 w-px bg-line"></div>
          <button
            class="flex items-center gap-1.5 text-[11.5px] text-muted hover:text-ink"
            onclick={() => (app.scanOptions.recursive = !app.scanOptions.recursive)}
          >
            <span class="switch" class:on={app.scanOptions.recursive}></span> rekursiv
          </button>
          <button
            class="flex items-center gap-1.5 text-[11.5px] text-muted hover:text-ink"
            onclick={() => (app.scanOptions.honorIgnoreList = !app.scanOptions.honorIgnoreList)}
            title="$RECYCLE.BIN, System Volume Information, .Trash-*, .git, node_modules"
          >
            <span class="switch" class:on={app.scanOptions.honorIgnoreList}></span> Ignorier-Liste
          </button>
          <button
            class="flex items-center gap-1.5 text-[11.5px] text-muted hover:text-ink"
            onclick={() => (app.scanOptions.followSymlinks = !app.scanOptions.followSymlinks)}
          >
            <span class="switch" class:on={app.scanOptions.followSymlinks}></span> Symlinks folgen
          </button>
          <button class="btn btn-primary" onclick={() => app.startScan()} disabled={app.busy}>
            {#if app.busy}
              <Zap class="h-3.5 w-3.5 animate-pulse" /> läuft…
            {:else}
              <Play class="h-3.5 w-3.5" /> Scannen
            {/if}
          </button>
        </div>

        <div class="mt-2 flex items-center gap-2">
          <span class="text-[10.5px] text-muted">Zusätzliche Ausschlüsse (Ordnernamen, Komma):</span>
          <input
            class="input mono h-7 flex-1 text-[11px]"
            placeholder="z.B. Backups, Temp, .venv"
            value={Array.isArray(app.scanOptions.extraIgnores)
              ? app.scanOptions.extraIgnores.join(', ')
              : app.scanOptions.extraIgnores}
            oninput={(e) =>
              (app.scanOptions.extraIgnores = e.currentTarget.value
                .split(',')
                .map((s) => s.trim())
                .filter(Boolean))}
          />
        </div>
      </div>

      <ProgressPanel />

      {#if summary}
        <!-- Statistik -->
        <div class="grid grid-cols-2 gap-2 lg:grid-cols-6">
          {#each [
            { icon: Files, label: "Dateien", value: formatCount(summary.files), sub: "erfasst" },
            { icon: FolderTree, label: "Ordner", value: formatCount(summary.dirs), sub: "durchlaufen" },
            { icon: HardDrive, label: "Größe", value: formatBytes(summary.bytes), sub: "gesamt" },
            {
              icon: Clock,
              label: "Dauer",
              value: summary.durationMs < 1000
                ? `${summary.durationMs} ms`
                : `${(summary.durationMs / 1000).toFixed(2)} s`,
              sub: `${Math.round(summary.files / Math.max(1, summary.durationMs / 1000)).toLocaleString("de-DE")} Dateien/s`,
            },
            {
              icon: Gauge,
              label: "Threads",
              value: String(summary.threads),
              sub: summary.truncated ? "Limit erreicht" : "parallel",
            },
            {
              icon: summary.errors.length ? AlertTriangle : ShieldAlert,
              label: "Fehler",
              value: formatCount(summary.errors.length),
              sub: `${formatCount(summary.skipped)} übersprungen`,
              tone: summary.errors.length ? "text-warn" : "text-muted",
            },
          ] as st}
            <div class="card flex items-center gap-3 px-3 py-2.5">
              <st.icon class="h-4 w-4 flex-none {st.tone ?? 'text-muted'}" />
              <div class="min-w-0">
                <div class="text-[10px] uppercase tracking-wider text-muted">{st.label}</div>
                <div class="num text-[15px] font-semibold leading-tight">{st.value}</div>
                <div class="truncate text-[10px] text-[#5f6d84]">{st.sub}</div>
              </div>
            </div>
          {/each}
        </div>

        {#if summary.cancelled}
          <div class="card flex items-center gap-2 border-warn/40 bg-warn/5 px-3 py-2 text-[11.5px] text-warn">
            <AlertTriangle class="h-3.5 w-3.5" /> Scan wurde abgebrochen — Ergebnisse sind
            unvollständig.
          </div>
        {/if}

        {#if summary.errors.length}
          <details class="card px-3 py-2">
            <summary class="cursor-pointer text-[11.5px] text-muted">
              {summary.errors.length} Fehler beim Lesen (z. B. Rechte) — Details
            </summary>
            <div class="mono mt-2 max-h-32 overflow-y-auto text-[10.5px] text-[#7d8ba1]">
              {#each summary.errors as err}
                <div class="truncate">{err}</div>
              {/each}
            </div>
          </details>
        {/if}

        <!-- Kategorien -->
        <div class="card flex min-h-[320px] flex-1 flex-col overflow-hidden">
          <div class="flex items-center gap-2 border-b border-line px-3 py-2">
            <h2 class="text-[13px] font-semibold">Analyse</h2>
            <div class="flex overflow-hidden rounded-lg border border-line">
              <button
                class="btn btn-ghost btn-sm rounded-none border-0"
                class:on={app.viewMode === "categories"}
                onclick={() => (app.viewMode = "categories")}
              >
                <FolderTree class="h-3 w-3" /> Kategorien
              </button>
              <button
                class="btn btn-ghost btn-sm rounded-none border-0"
                class:on={app.viewMode === "extensions"}
                onclick={() => (app.viewMode = "extensions")}
              >
                <List class="h-3 w-3" /> Endungen
              </button>
            </div>
            <div class="relative w-56">
              <Search class="absolute left-2.5 top-2 h-3.5 w-3.5 text-muted" />
              <input class="input h-7 pl-8 text-[11.5px]" placeholder="filtern…" bind:value={filter} />
            </div>
            <div class="ml-auto flex items-center gap-1.5">
              <button
                class="btn btn-ghost btn-sm"
                onclick={() => {
                  for (const c of summary.categories) app.openCats = { ...app.openCats, [c.name]: true };
                }}
                disabled={app.viewMode === "extensions"}>alles auf</button
              >
              <button
                class="btn btn-ghost btn-sm"
                onclick={() => (app.openCats = {})}
                disabled={app.viewMode === "extensions"}>zu</button
              >
              <button class="btn btn-ghost btn-sm" onclick={() => app.selectAllFrom(summary)}
                >alle wählen</button
              >
              <button class="btn btn-ghost btn-sm" onclick={() => app.clearSelection()}>Auswahl weg</button
              >
            </div>
          </div>
          <div class="min-h-0 flex-1">
            <CategoryTree {summary} mode={app.viewMode} {filter} />
          </div>
        </div>
      {:else}
        <div class="card flex flex-1 flex-col items-center justify-center gap-3 p-10 text-center">
          <div class="rounded-2xl border border-line bg-panel2 p-4">
            <Search class="h-7 w-7 text-accent" />
          </div>
          <div class="text-[14px] font-semibold">Bereit zum Scannen</div>
          <p class="max-w-md text-[12px] leading-relaxed text-muted">
            Laufwerk oben wählen oder einen Pfad eingeben. DiskForge traversiert parallel über alle
            Kerne ({app.sys.threads}), fasst Endungen case-insensitiv zusammen und ordnet sie deinen
            Überkategorien zu.
          </p>
          <div class="mt-1 flex flex-wrap justify-center gap-1.5">
            {#each ["Bilder", "Videos", "Audio", "Dokumente", "Archive", "Programme & Skripte", "Sonstiges"] as c}
              <span class="chip">{c}</span>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>

  {#if summary}
    <!-- Auswahl-Leiste -->
    <div class="flex items-center gap-3 border-t border-line bg-panel px-4 py-2">
      <CircleCheck class="h-3.5 w-3.5 text-accent" />
      <span class="text-[11.5px] text-muted">
        Auswahl:
        <span class="text-ink">{formatCount(sel.files)}</span> Dateien ·
        <span class="text-ink">{formatBytes(sel.bytes)}</span>
      </span>
      <span class="text-[10.5px] text-[#5f6d84]">
        {app.selCats.length ? `${app.selCats.length} Kategorien` : ""}
        {app.selExts.length ? ` ${app.selExts.length} Endungen` : ""}
        {!app.selCats.length && !app.selExts.length ? "alles" : ""}
        {#if app.dupFileCount} · {app.dupFileCount} Duplikate markiert{/if}
      </span>
      <div class="ml-auto flex items-center gap-2">
        <button class="btn btn-sm" onclick={() => app.refreshLog()} title="Protokoll aktualisieren">
          <RotateCcw class="h-3 w-3" /> Log
        </button>
        <button class="btn btn-primary" onclick={() => (app.sortOpen = true)}>
          <ArrowDownUp class="h-3.5 w-3.5" /> Sortieren…
        </button>
      </div>
    </div>
  {/if}
</div>
