<script>
  import { onMount } from "svelte";
  import { app } from "./lib/store.svelte.js";
  import Overview from "./lib/tabs/Overview.svelte";
  import Duplicates from "./lib/tabs/Duplicates.svelte";
  import Rules from "./lib/tabs/Rules.svelte";
  import LogTab from "./lib/tabs/LogTab.svelte";
  import SortDrawer from "./lib/components/SortDrawer.svelte";
  import { formatBytes, formatCount } from "./lib/format.js";
  import {
    Gauge,
    Copy,
    Tag,
    History,
    ArrowDownUp,
    Cpu,
    Film,
    X,
    CheckCircle2,
    AlertTriangle,
    Info,
    HardDrive,
  } from "lucide-svelte";

  onMount(() => {
    app.init();
  });

  const tabs = [
    { id: "overview", label: "Übersicht & Kategorien", icon: Gauge },
    { id: "duplicates", label: "Duplikate & Vergleich", icon: Copy },
    { id: "rules", label: "Kategorie-Regeln", icon: Tag },
    { id: "log", label: "Protokoll / Undo", icon: History },
  ];

  let dupCount = $derived(app.dups.length);
  let marked = $derived(app.dupFileCount);

  function toastColor(kind) {
    if (kind === "error") return "border-danger/50 bg-[#2a1216]";
    if (kind === "ok") return "border-ok/40 bg-[#0f2018]";
    if (kind === "warn") return "border-warn/50 bg-[#251a09]";
    return "border-line bg-panel2";
  }
</script>

<header class="flex h-12 flex-none items-center gap-3 border-b border-line bg-[#0d121b] px-4">
  <div class="flex items-center gap-2">
    <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-gradient-to-br from-[#5b95ff] to-[#22d3ee]">
      <HardDrive class="h-4 w-4 text-[#071018]" />
    </div>
    <div class="leading-tight">
      <div class="text-[13px] font-semibold tracking-tight">Fylr</div>
      <div class="text-[9.5px] text-[#5f6d84]">Scanner · Dedupe · Sorter</div>
    </div>
  </div>

  <div class="mx-2 h-6 w-px bg-line"></div>

  <div class="flex min-w-0 items-center gap-2">
    {#if app.root}
      <span class="mono max-w-[420px] truncate text-[11px] text-[#93a2ba]" title={app.root}>{app.root}</span>
    {:else}
      <span class="text-[11px] text-muted">kein Quellordner</span>
    {/if}
    {#if app.summary}
      <span class="chip num">{formatCount(app.summary.files)} Dateien</span>
      <span class="chip num">{formatBytes(app.summary.bytes)}</span>
    {/if}
  </div>

  <div class="ml-auto flex items-center gap-3 text-[10.5px] text-muted">
    <span class="flex items-center gap-1 tooltip" data-tip="Parallelität (Rayon/jwalk)">
      <Cpu class="h-3 w-3" /> {app.sys.threads || "–"} Threads
    </span>
    <span class="flex items-center gap-1 tooltip" data-tip="ffprobe für Video-Metadaten">
      <Film class="h-3 w-3 {app.sys.ffprobe ? 'text-ok' : 'text-warn'}" />
      {app.sys.ffprobe ? "ffprobe" : "mp4parse"}
    </span>
    <span class="chip">v{app.sys.version || "0.1.0"}</span>
    {#if app.busy}
      <span class="flex items-center gap-1 text-accent">
        <span class="h-2 w-2 animate-pulse rounded-full bg-accent"></span> aktiv
      </span>
    {/if}
  </div>
</header>

<div class="flex min-h-0 flex-1">
  <aside class="flex w-[218px] flex-none flex-col border-r border-line bg-[#0d121b]">
    <nav class="flex flex-col gap-0.5 p-2">
      {#each tabs as t (t.id)}
        <button
          class="flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-[12px] transition-colors hover:bg-panel2"
          class:nav-active={app.tab === t.id}
          class:text-ink={app.tab === t.id}
          class:text-muted={app.tab !== t.id}
          onclick={() => (app.tab = t.id)}
        >
          <t.icon class="h-3.5 w-3.5 {app.tab === t.id ? 'text-accent' : ''}" />
          <span class="truncate">{t.label}</span>
          {#if t.id === "duplicates" && dupCount}
            <span class="badge badge-accent ml-auto">{dupCount}</span>
          {/if}
          {#if t.id === "log" && app.logs.length}
            <span class="badge badge-muted ml-auto">{app.logs.length}</span>
          {/if}
        </button>
      {/each}
    </nav>

    <div class="mx-2 h-px bg-line"></div>

    <div class="flex flex-col gap-2 p-2">
      <button class="btn btn-primary w-full" onclick={() => (app.sortOpen = true)}>
        <ArrowDownUp class="h-3.5 w-3.5" /> Sortieren
      </button>
      {#if marked}
        <div class="rounded-lg border border-line bg-panel2 px-2.5 py-2 text-[10.5px]">
          <div class="flex items-center justify-between">
            <span class="text-muted">Duplikate markiert</span>
            <span class="num text-warn">{formatCount(marked)}</span>
          </div>
          <div class="mt-1 flex items-center justify-between">
            <span class="text-muted">redundant</span>
            <span class="num">{formatBytes(app.dupWastedBytes)}</span>
          </div>
          <div class="mt-1 flex items-center justify-between">
            <span class="text-muted">Richtlinie</span>
            <span class="text-ink">{app.sort.duplicatePolicy}</span>
          </div>
        </div>
      {/if}
    </div>

    <div class="mt-auto flex flex-col gap-1 border-t border-line p-2 text-[10px] text-[#4b586e]">
      <span>Tauri v2 · Rust · Svelte 5</span>
      <span>BLAKE3 · ignore · rayon</span>
      <span class="mono truncate">~/.config/fylr/</span>
    </div>
  </aside>

  <main class="min-w-0 flex-1 overflow-hidden">
    {#if app.tab === "overview"}
      <Overview />
    {:else if app.tab === "duplicates"}
      <Duplicates />
    {:else if app.tab === "rules"}
      <Rules />
    {:else}
      <LogTab />
    {/if}
  </main>
</div>

<SortDrawer />

<div class="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[380px] flex-col gap-2">
  {#each app.toasts as t (t.id)}
    <div class="pointer-events-auto flex items-start gap-2 rounded-lg border px-3 py-2 text-[11.5px] shadow-xl {toastColor(t.kind)}">
      {#if t.kind === "error"}
        <AlertTriangle class="mt-0.5 h-3.5 w-3.5 flex-none text-danger" />
      {:else if t.kind === "ok"}
        <CheckCircle2 class="mt-0.5 h-3.5 w-3.5 flex-none text-ok" />
      {:else if t.kind === "warn"}
        <AlertTriangle class="mt-0.5 h-3.5 w-3.5 flex-none text-warn" />
      {:else}
        <Info class="mt-0.5 h-3.5 w-3.5 flex-none text-accent" />
      {/if}
      <span class="flex-1 leading-relaxed select-text">{t.message}</span>
      <button class="btn btn-icon btn-ghost h-5 w-5" onclick={() => app.dismiss(t.id)}>
        <X class="h-3 w-3" />
      </button>
    </div>
  {/each}
</div>
