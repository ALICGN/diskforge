<script>
  import { app } from "../store.svelte.js";
  import { formatBytes, formatCount, basename } from "../format.js";
  import VirtualList from "../components/VirtualList.svelte";
  import DupCard from "../components/DupCard.svelte";
  import ProgressPanel from "../components/ProgressPanel.svelte";
  import {
    Copy,
    Search,
    CalendarClock,
    Route,
    Eraser,
    ArrowDownUp,
    Files,
    X,
    Play,
    Pause,
    Columns2,
    AlertTriangle,
  } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";

  let selectedId = $state(null);
  let metas = $state({});
  let loadingMeta = $state(false);
  let zoomed = $state(null);
  let filter = $state("");
  let videosRef = $state(null);
  let syncing = $state(false);

  let groups = $derived(
    app.dups.filter((g) => {
      if (!filter.trim()) return true;
      const f = filter.toLowerCase();
      return g.files.some((x) => x.path.toLowerCase().includes(f)) || g.hash.startsWith(f);
    }),
  );
  let selected = $derived(groups.find((g) => g.id === selectedId) ?? groups[0] ?? null);

  let fileCount = $derived(app.dups.reduce((a, g) => a + g.files.length, 0));
  let redundant = $derived(app.dups.reduce((a, g) => a + (g.files.length - 1), 0));

  let oldest = $derived.by(() => {
    if (!selected) return null;
    let best = null;
    for (const f of selected.files) {
      const m = metas[f.path];
      if (!m?.mtimeMs) continue;
      if (!best || m.mtimeMs < best.mtimeMs) best = m;
    }
    return best?.path ?? null;
  });
  let newest = $derived.by(() => {
    if (!selected) return null;
    let best = null;
    for (const f of selected.files) {
      const m = metas[f.path];
      if (!m?.mtimeMs) continue;
      if (!best || m.mtimeMs > best.mtimeMs) best = m;
    }
    return best?.path ?? null;
  });

  async function loadMeta(group) {
    if (!group) return;
    loadingMeta = true;
    try {
      const list = await invoke("file_meta", {
        paths: group.files.map((f) => f.path),
        probe: true,
      });
      const next = { ...metas };
      for (const m of list) next[m.path] = m;
      metas = next;
    } catch (err) {
      app.toast(String(err), "error");
    } finally {
      loadingMeta = false;
    }
  }

  $effect(() => {
    if (selected && !metas[selected.files[0]?.path]) loadMeta(selected);
  });

  function mark(file, on) {
    app.markDuplicate(file.path, on);
  }

  function keepOldest(group, scopeAll = false) {
    if (scopeAll) return keepOldestAll();
    const paths = group.files.map((f) => ({ path: f.path, mtime: metas[f.path]?.mtimeMs ?? 0 }));
    const oldestPath = paths.reduce((a, b) => (b.mtime && (!a.mtime || b.mtime < a.mtime) ? b : a), paths[0])?.path;
    app.markGroup(group.files, oldestPath);
    app.toast(`Älteste Datei behalten: ${basename(oldestPath)}`, "info");
  }

  function keepShortest(group) {
    const shortest = [...group.files].sort((a, b) => a.path.length - b.path.length)[0]?.path;
    app.markGroup(group.files, shortest);
    app.toast(`Kürzester Pfad behalten: ${basename(shortest)}`, "info");
  }

  async function keepOldestAll() {
    if (!app.dups.length) return;
    app.busy = true;
    try {
      const all = app.dups.flatMap((g) => g.files.map((f) => f.path));
      const list = await invoke("file_meta", { paths: all, probe: false });
      const byPath = Object.fromEntries(list.map((m) => [m.path, m]));
      const next = {};
      for (const g of app.dups) {
        let best = null;
        for (const f of g.files) {
          const m = byPath[f.path];
          if (!m?.mtimeMs) continue;
          if (!best || m.mtimeMs < best.ms) best = { path: f.path, ms: m.mtimeMs };
        }
        const keep = best?.path ?? g.files[0]?.path;
        for (const f of g.files) if (f.path !== keep) next[f.path] = true;
      }
      app.dupMarks = next;
      app.toast(`Älteste Datei in allen ${app.dups.length} Gruppen behalten`, "ok");
    } catch (err) {
      app.toast(String(err), "error");
    } finally {
      app.busy = false;
    }
  }

  function syncPlay() {
    if (!videosRef) return;
    const vids = [...videosRef.querySelectorAll("video")];
    if (!vids.length) return;
    const t = vids[0].currentTime;
    for (const v of vids) v.currentTime = t;
    if (syncing) {
      for (const v of vids) v.pause();
    } else {
      for (const v of vids) v.play().catch(() => {});
    }
    syncing = !syncing;
  }

  function scrub(e) {
    if (!videosRef) return;
    const vids = [...videosRef.querySelectorAll("video")];
    const ratio = Number(e.currentTarget.value) / 100;
    for (const v of vids) {
      if (v.duration) v.currentTime = ratio * v.duration;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && (zoomed = null)} />

<div class="flex h-full flex-col overflow-hidden">
  <!-- Toolbar -->
  <div class="flex flex-wrap items-center gap-2 border-b border-line bg-panel px-4 py-2.5">
    <Copy class="h-4 w-4 text-accent" />
    <h2 class="text-[13px] font-semibold">Duplikate</h2>
    <div class="flex items-center gap-1.5">
      <span class="text-[11px] text-muted">min. Größe</span>
      <input
        class="input h-7 w-16 px-2 text-[11px]"
        type="number"
        min="0"
        step="1"
        bind:value={app.dupMinMb}
      />
      <span class="text-[11px] text-muted">MB</span>
    </div>
    <button class="btn btn-primary btn-sm" onclick={() => app.findDuplicates()} disabled={app.busy || !app.summary}>
      <Search class="h-3 w-3" /> Duplikate suchen
    </button>
    <div class="mx-1 h-5 w-px bg-line"></div>
    <button class="btn btn-sm" onclick={() => keepOldestAll()} disabled={!app.dups.length || app.busy}>
      <CalendarClock class="h-3 w-3" /> immer älteste behalten
    </button>
    <button
      class="btn btn-sm"
      onclick={() => {
        for (const g of app.dups) keepShortest(g);
      }}
      disabled={!app.dups.length}
    >
      <Route class="h-3 w-3" /> kürzester Pfad
    </button>
    <button class="btn btn-sm btn-ghost" onclick={() => app.clearAllMarks()} disabled={!app.dupFileCount}>
      <Eraser class="h-3 w-3" /> Markierungen
    </button>
    <div class="ml-auto flex items-center gap-3 text-[11px] text-muted num">
      <span><span class="text-ink">{formatCount(app.dups.length)}</span> Gruppen</span>
      <span><span class="text-ink">{formatCount(fileCount)}</span> Dateien</span>
      <span><span class="text-warn">{formatBytes(app.dupGroupWasted)}</span> redundant</span>
      {#if app.dupFileCount}
        <button class="btn btn-sm btn-danger" onclick={() => (app.sortOpen = true)}>
          <ArrowDownUp class="h-3 w-3" /> {app.dupFileCount} markiert
        </button>
      {/if}
    </div>
  </div>

  <div class="px-4 pt-2">
    <ProgressPanel compact />
  </div>

  {#if !app.dups.length}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 p-10 text-center">
      <div class="rounded-2xl border border-line bg-panel2 p-4"><Copy class="h-7 w-7 text-accent" /></div>
      <div class="text-[14px] font-semibold">Noch keine Duplikat-Analyse</div>
      <p class="max-w-md text-[12px] leading-relaxed text-muted">
        DiskForge gruppiert zuerst nach exakter Byte-Größe und hasht nur echte Kandidaten mit
        BLAKE3. Dateien über 50 MB werden über Kopf- und Fuß-64 KB vorgefiltert.
      </p>
      <button class="btn btn-primary" onclick={() => app.findDuplicates()} disabled={!app.summary || app.busy}>
        <Search class="h-3.5 w-3.5" /> Jetzt suchen
      </button>
      {#if !app.summary}
        <div class="text-[11px] text-warn">Zuerst ein Verzeichnis scannen.</div>
      {/if}
    </div>
  {:else}
    <div class="flex min-h-0 flex-1">
      <!-- Gruppenliste -->
      <aside class="flex w-[330px] flex-none flex-col border-r border-line">
        <div class="relative border-b border-line p-2">
          <Search class="absolute left-4 top-4 h-3.5 w-3.5 text-muted" />
          <input class="input h-7 pl-8 text-[11.5px]" placeholder="Gruppe / Hash filtern…" bind:value={filter} />
        </div>
        <div class="min-h-0 flex-1">
          <VirtualList items={groups} itemHeight={44} empty="Keine Treffer">
            {#snippet row(g)}
              <button
                class="flex h-[44px] w-full items-center gap-2 border-b border-line/40 px-2.5 text-left hover:bg-panel2"
                class:sel-row={selected?.id === g.id}
                onclick={() => (selectedId = g.id)}
              >
                <span class="flex flex-col">
                  <span class="num text-[11.5px] font-medium">{formatBytes(g.size)}</span>
                  <span class="mono text-[9.5px] text-[#5f6d84]">{g.hash.slice(0, 12)}…</span>
                </span>
                <span class="ml-auto flex items-center gap-1">
                  {#each g.files.slice(0, 3) as f}
                    <span
                      class="h-1.5 w-1.5 rounded-full"
                      class:bg-danger={app.dupMarks[f.path]}
                      class:bg-ok={!app.dupMarks[f.path]}
                    ></span>
                  {/each}
                  <span class="badge badge-muted">{g.files.length}</span>
                </span>
              </button>
            {/snippet}
          </VirtualList>
        </div>
      </aside>

      <!-- Vergleich -->
      <section class="flex min-w-0 flex-1 flex-col overflow-hidden">
        {#if selected}
          <div class="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2">
            <span class="text-[12px] font-medium">{selected.files.length} identische Dateien</span>
            <span class="chip num">{formatBytes(selected.size)}</span>
            <span class="mono text-[10px] text-[#5f6d84]">BLAKE3 {selected.hash.slice(0, 20)}…</span>
            <div class="ml-auto flex items-center gap-1.5">
              <button class="btn btn-sm" onclick={() => keepOldest(selected)}>
                <CalendarClock class="h-3 w-3" /> älteste behalten
              </button>
              <button class="btn btn-sm" onclick={() => keepShortest(selected)}>
                <Route class="h-3 w-3" /> kürzester Pfad
              </button>
              <button class="btn btn-sm btn-ghost" onclick={() => app.clearGroup(selected.files)}>
                <Eraser class="h-3 w-3" /> zurücksetzen
              </button>
              {#if selected.files.some((f) => metas[f.path]?.kind === "video")}
                <button class="btn btn-sm" onclick={syncPlay}>
                  {#if syncing}<Pause class="h-3 w-3" /> alle pausieren{:else}<Play class="h-3 w-3" /> alle abspielen{/if}
                </button>
              {/if}
            </div>
          </div>

          <div class="min-h-0 flex-1 overflow-y-auto p-3" bind:this={videosRef}>
            {#if loadingMeta}
              <div class="mb-2 flex items-center gap-2 text-[11px] text-muted">
                <span class="h-3 w-3 animate-spin rounded-full border border-accent border-t-transparent"></span>
                Metadaten werden gelesen…
              </div>
            {/if}
            <div class="grid grid-cols-1 gap-3 xl:grid-cols-2 2xl:grid-cols-3">
              {#each selected.files as f (f.path)}
                <DupCard
                  meta={metas[f.path] ?? {
                    path: f.path,
                    name: f.name,
                    ext: f.ext,
                    size: f.size,
                    kind: "file",
                  }}
                  mark={!!app.dupMarks[f.path]}
                  oldest={oldest === f.path}
                  newest={newest === f.path && selected.files.length > 1}
                  onMark={mark}
                  onZoom={(m) => (zoomed = m)}
                />
              {/each}
            </div>
            <div class="mt-3 flex items-start gap-2 rounded-lg border border-line bg-panel2 px-3 py-2 text-[11px] text-muted">
              <AlertTriangle class="mt-0.5 h-3.5 w-3.5 flex-none text-warn" />
              <span>
                Markierte Dateien werden beim Sortieren gemäß deiner Duplikat-Richtlinie
                übersprungen, in <span class="mono text-ink">_Duplikate</span> verschoben oder
                gelöscht. Markierungen wirken nur auf diesen Lauf — es wird nichts automatisch
                entfernt.
              </span>
            </div>
          </div>
        {/if}
      </section>
    </div>
  {/if}
</div>

{#if zoomed}
  <div
    class="fixed inset-0 z-50 flex flex-col items-center justify-center bg-black/92 p-6"
    onclick={() => (zoomed = null)}
    role="presentation"
  >
    <img src={app.assetUrl(zoomed.path)} alt={zoomed.name} class="max-h-[86vh] max-w-full object-contain" />
    <div class="mt-3 flex items-center gap-3 text-[11px] text-[#9fb0c9]">
      <span class="mono truncate">{zoomed.path}</span>
      {#if zoomed.width}<span class="chip num">{zoomed.width} × {zoomed.height} px</span>{/if}
      <span class="chip num">{formatBytes(zoomed.size)}</span>
    </div>
    <button class="btn btn-icon btn-ghost absolute right-4 top-4" onclick={() => (zoomed = null)}>
      <X class="h-4 w-4" />
    </button>
  </div>
{/if}
