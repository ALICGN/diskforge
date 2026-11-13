<script>
  import { app } from "../store.svelte.js";
  import { formatBytes, formatCount } from "../format.js";
  import ProgressPanel from "./ProgressPanel.svelte";
  import {
    X,
    FolderOpen,
    ArrowRight,
    FlaskConical,
    Play,
    AlertTriangle,
    Undo2,
    SkipForward,
    FolderInput,
    Trash2,
    FolderTree,
    FileType2,
    Layers,
    CheckCircle2,
  } from "lucide-svelte";

  let confirmDelete = $state(false);
  let plan = $derived(app.lastPlan);
  let result = $derived(app.lastSort);
  let sel = $derived(app.selectedStats());

  let sampleExt = $derived(app.selExts[0] || "jpg");
  let preview = $derived.by(() => {
    const s = app.sort;
    const base = (s.target || "/Ziel").replace(/\/$/, "");
    if (s.structure === "ext") return `${base}/${sampleExt.toUpperCase()}/datei.${sampleExt}`;
    if (s.structure === "category") return `${base}/Bilder/datei.${sampleExt}`;
    return `${base}/Bilder/${sampleExt.toUpperCase()}/datei.${sampleExt}`;
  });

  const structures = [
    { id: "ext", title: "Nur nach Endung", example: "Ziel/JPG/datei.jpg", icon: FileType2 },
    { id: "category", title: "Nur nach Überkategorie", example: "Ziel/Bilder/datei.jpg", icon: FolderTree },
    { id: "categoryExt", title: "Hierarchisch", example: "Ziel/Bilder/JPG/datei.jpg", icon: Layers },
  ];

  async function pickTarget() {
    const p = await app.pickDirectory("Zielordner wählen");
    if (p) app.sort.target = p;
  }

  async function simulate() {
    confirmDelete = false;
    const batch = await app.buildPlan();
    if (batch) app.toast(`${batch.ops.length} geplante Aktionen berechnet`, "info");
  }

  async function run() {
    if (app.sort.duplicatePolicy === "delete" && !confirmDelete) {
      app.toast("Lösch-Richtlinie erfordert die Bestätigung im Dialog.", "warn");
      return;
    }
    await app.runSort();
    confirmDelete = false;
  }
</script>

{#if app.sortOpen}
  <div
    class="fixed inset-0 z-40 flex justify-end bg-black/50 backdrop-blur-[1px]"
    onclick={(e) => e.target === e.currentTarget && (app.sortOpen = false)}
    role="presentation"
  >
    <div class="flex h-full w-[560px] flex-col border-l border-line bg-[#0d121b] shadow-2xl">
      <div class="flex items-center gap-2 border-b border-line px-4 py-3">
        <ArrowRight class="h-4 w-4 text-accent" />
        <h2 class="text-[13px] font-semibold">Sortieren & Verschieben</h2>
        <button class="btn btn-icon btn-ghost ml-auto" onclick={() => (app.sortOpen = false)}>
          <X class="h-4 w-4" />
        </button>
      </div>

      <div class="min-h-0 flex-1 space-y-3 overflow-y-auto p-4">
        <!-- Auswahl -->
        <div class="card px-3 py-2 text-[11.5px]">
          <div class="flex items-center justify-between">
            <span class="text-muted">Auswahl</span>
            <span class="num">
              {formatCount(sel.files)} Dateien · <span class="text-ink">{formatBytes(sel.bytes)}</span>
            </span>
          </div>
          <div class="mt-1 flex items-center justify-between">
            <span class="text-muted">Duplikate markiert</span>
            <span class="num">
              {formatCount(app.dupFileCount)} · <span class="text-warn">{formatBytes(app.dupWastedBytes)}</span>
            </span>
          </div>
          <div class="mt-1 flex flex-wrap gap-1">
            {#if app.selCats.length}
              {#each app.selCats as c}<span class="chip">{c}</span>{/each}
            {/if}
            {#if app.selExts.length}
              {#each app.selExts.slice(0, 12) as e}<span class="chip mono">{e || "ohne"}</span>{/each}
              {#if app.selExts.length > 12}<span class="chip">+{app.selExts.length - 12}</span>{/if}
            {/if}
            {#if !app.selCats.length && !app.selExts.length}
              <span class="chip">alle Endungen</span>
            {/if}
          </div>
        </div>

        <!-- Ziel -->
        <div class="flex flex-col gap-1">
          <span class="text-[10.5px] uppercase tracking-wider text-muted">Ziel-Basisordner</span>
          <div class="flex items-center gap-2">
            <input class="input mono" placeholder="/run/media/ali/ExterneHD/Sortiert" bind:value={app.sort.target} />
            <button class="btn" onclick={pickTarget}><FolderOpen class="h-3.5 w-3.5" /></button>
          </div>
        </div>

        <!-- Struktur -->
        <div class="flex flex-col gap-1.5">
          <span class="text-[10.5px] uppercase tracking-wider text-muted">Zielstruktur</span>
          {#each structures as s (s.id)}
            <button
              class="flex items-center gap-3 rounded-lg border px-3 py-2 text-left"
              class:border-accent={app.sort.structure === s.id}
              class:sel-card={app.sort.structure === s.id}
              class:border-line={app.sort.structure !== s.id}
              class:bg-panel2={app.sort.structure !== s.id}
              onclick={() => (app.sort.structure = s.id)}
            >
              <s.icon class="h-4 w-4 flex-none text-muted" />
              <div class="min-w-0">
                <div class="text-[12px] font-medium">{s.title}</div>
                <div class="mono truncate text-[10px] text-[#5f6d84]">{s.example}</div>
              </div>
              {#if app.sort.structure === s.id}
                <CheckCircle2 class="ml-auto h-4 w-4 text-accent" />
              {/if}
            </button>
          {/each}
          <div class="mono rounded-lg bg-panel2 px-2.5 py-1.5 text-[10.5px] text-[#7d8ba1]">
            Vorschau: {preview}
          </div>
        </div>

        <!-- Duplikate -->
        <div class="flex flex-col gap-1.5">
          <span class="text-[10.5px] uppercase tracking-wider text-muted">
            Duplikate ({app.dupFileCount} markiert)
          </span>
          <div class="flex gap-1.5">
            {#each [
              { id: "skip", label: "Überspringen", icon: SkipForward },
              { id: "folder", label: "_Duplikate-Ordner", icon: FolderInput },
              { id: "delete", label: "Endgültig löschen", icon: Trash2 },
            ] as p (p.id)}
              <button
                class="btn flex-1"
                class:btn-primary={app.sort.duplicatePolicy === p.id}
                class:btn-danger={app.sort.duplicatePolicy === p.id && p.id === "delete"}
                onclick={() => (app.sort.duplicatePolicy = p.id)}
              >
                <p.icon class="h-3 w-3" /> {p.label}
              </button>
            {/each}
          </div>
          {#if app.sort.duplicatePolicy === "delete"}
            <label class="flex items-start gap-2 rounded-lg border border-danger/40 bg-danger/5 px-2.5 py-2 text-[11px] text-[#fca5a5]">
              <input type="checkbox" class="mt-0.5" bind:checked={confirmDelete} />
              <span>
                Ich verstehe, dass {formatCount(app.dupFileCount)} markierte Dateien
                ({formatBytes(app.dupWastedBytes)}) ohne Papierkorb gelöscht werden. Dieser Vorgang
                ist nicht rückgängig zu machen.
              </span>
            </label>
          {/if}
        </div>

        <!-- Optionen -->
        <div class="card flex flex-col gap-2 px-3 py-2">
          <button
            class="flex items-center gap-2 text-[11.5px]"
            onclick={() => (app.sort.dryRun = !app.sort.dryRun)}
          >
            <span class="switch" class:on={app.sort.dryRun}></span>
            <span>Dry-Run (nur simulieren)</span>
            <span class="ml-auto text-[10px] text-muted">nichts wird verändert</span>
          </button>
          <button
            class="flex items-center gap-2 text-[11.5px]"
            onclick={() => (app.sort.preserveMtime = !app.sort.preserveMtime)}
          >
            <span class="switch" class:on={app.sort.preserveMtime}></span>
            <span>Änderungszeit übernehmen</span>
            <span class="ml-auto text-[10px] text-muted">wichtig für Behalten-Ältestes</span>
          </button>
        </div>

        <ProgressPanel compact />

        <!-- Plan -->
        {#if plan || result}
          {@const b = result ?? plan}
          <div class="card overflow-hidden">
            <div class="flex items-center gap-2 border-b border-line px-3 py-1.5">
              {#if b.dryRun}<FlaskConical class="h-3.5 w-3.5 text-accent" />{:else}<CheckCircle2 class="h-3.5 w-3.5 text-ok" />{/if}
              <span class="text-[11.5px] font-medium">
                {b.dryRun ? "Simulation" : "Ausgeführt"} — {formatCount(b.ops.length)} Aktionen
              </span>
              <span class="ml-auto chip num">{formatBytes(b.bytes)}</span>
              {#if b.errors}<span class="badge badge-danger">{b.errors} Fehler</span>{/if}
            </div>
            <div class="flex flex-wrap gap-3 px-3 py-1.5 text-[10.5px] num">
              <span class="text-ok">{formatCount(b.moved)} verschoben</span>
              <span class="text-accent">{formatCount(b.copied)} kopiert</span>
              <span class="text-danger">{formatCount(b.deleted)} gelöscht</span>
              <span class="text-muted">{formatCount(b.skipped)} übersprungen</span>
              <span class="ml-auto text-muted">{b.durationMs} ms</span>
            </div>
            <div class="max-h-56 overflow-y-auto border-t border-line/60">
              {#each b.ops.slice(0, 200) as op}
                <div class="flex items-center gap-2 border-b border-line/30 px-3 py-1 text-[10px]">
                  <span
                    class="w-[86px] flex-none truncate"
                    class:text-ok={op.action === "moved"}
                    class:text-accent={op.action === "copied" || op.action === "planned"}
                    class:text-danger={op.action === "error" || op.action === "deleted"}
                    class:text-warn={op.action.includes("duplicate")}
                    class:text-muted={op.action.startsWith("skipped")}
                    >{op.action}</span
                  >
                  <span class="mono truncate text-[#93a2ba]" title={op.src}>{op.src.split("/").pop()}</span>
                  <ArrowRight class="h-2.5 w-2.5 flex-none text-[#4b586e]" />
                  <span class="mono truncate text-[#6f7f96]" title={op.dst}>{op.dst.replace(app.sort.target, "…")}</span>
                  {#if op.error}<span class="truncate text-danger" title={op.error}>{op.error}</span>{/if}
                </div>
              {/each}
              {#if b.ops.length > 200}
                <div class="px-3 py-1.5 text-[10px] text-muted">… {b.ops.length - 200} weitere</div>
              {/if}
            </div>
          </div>
        {/if}

        {#if result && !result.dryRun}
          <button class="btn btn-danger w-full" onclick={() => app.undoLast()} disabled={app.busy}>
            <Undo2 class="h-3.5 w-3.5" /> Diesen Vorgang rückgängig machen
          </button>
        {/if}
      </div>

      <div class="flex items-center gap-2 border-t border-line px-4 py-3">
        {#if app.sort.duplicatePolicy === "delete"}
          <AlertTriangle class="h-3.5 w-3.5 text-danger" />
        {/if}
        <button class="btn" onclick={simulate} disabled={app.busy || !app.sort.target}>
          <FlaskConical class="h-3.5 w-3.5" /> Vorschau
        </button>
        <div class="ml-auto flex items-center gap-2">
          <button class="btn btn-ghost" onclick={() => (app.sortOpen = false)}>Schließen</button>
          <button
            class="btn btn-primary"
            onclick={run}
            disabled={app.busy || !app.sort.target || (app.sort.duplicatePolicy === "delete" && !confirmDelete)}
          >
            <Play class="h-3.5 w-3.5" />
            {app.sort.dryRun ? "Simulation starten" : "Jetzt sortieren"}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
