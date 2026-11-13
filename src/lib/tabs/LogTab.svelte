<script>
  import { app } from "../store.svelte.js";
  import { formatBytes, formatDate, formatCount, formatDuration, basename } from "../format.js";
  import VirtualList from "../components/VirtualList.svelte";
  import {
    History,
    Undo2,
    RefreshCw,
    ChevronRight,
    CheckCircle2,
    Copy,
    Trash2,
    AlertTriangle,
    ArrowDownUp,
    FolderOpen,
    FlaskConical,
  } from "lucide-svelte";

  let expanded = $state(null);
  let showErrorsOnly = $state(false);

  let batches = $derived([...(app.logs ?? [])].reverse());

  const actionMeta = {
    moved: { label: "verschoben", class: "text-ok", icon: ArrowDownUp },
    copied: { label: "kopiert+löschen", class: "text-accent", icon: Copy },
    deleted: { label: "gelöscht", class: "text-danger", icon: Trash2 },
    "duplicate-moved": { label: "Duplikat→Ordner", class: "text-warn", icon: Copy },
    "skipped-duplicate": { label: "Duplikat übersprungen", class: "text-muted", icon: AlertTriangle },
    "skipped-same": { label: "schon am Ziel", class: "text-muted", icon: CheckCircle2 },
    planned: { label: "geplant", class: "text-accent", icon: FlaskConical },
    "planned-duplicate": { label: "Duplikat geplant", class: "text-warn", icon: FlaskConical },
    "would-delete": { label: "würde löschen", class: "text-danger", icon: Trash2 },
    error: { label: "Fehler", class: "text-danger", icon: AlertTriangle },
  };

  function opsOf(batch) {
    if (!showErrorsOnly) return batch.ops;
    return batch.ops.filter((o) => o.action === "error");
  }

  async function undo(batch) {
    await app.undoBatch(batch.id);
  }
</script>

<div class="flex h-full flex-col overflow-hidden">
  <div class="flex flex-wrap items-center gap-2 border-b border-line bg-panel px-4 py-2.5">
    <History class="h-4 w-4 text-accent" />
    <h2 class="text-[13px] font-semibold">Protokoll / Undo</h2>
    <span class="text-[11px] text-muted">letzte {app.logs.length} Vorgänge · move_log.json</span>
    <div class="ml-auto flex items-center gap-2">
      <button
        class="flex items-center gap-1.5 text-[11px] text-muted hover:text-ink"
        onclick={() => (showErrorsOnly = !showErrorsOnly)}
      >
        <span class="switch" class:on={showErrorsOnly}></span> nur Fehler
      </button>
      <button class="btn btn-sm btn-ghost" onclick={() => app.refreshLog()}>
        <RefreshCw class="h-3 w-3" /> Aktualisieren
      </button>
      <button
        class="btn btn-sm btn-danger"
        onclick={() => app.undoLast()}
        disabled={app.busy || !app.logs.some((b) => !b.dryRun && !b.undone)}
      >
        <Undo2 class="h-3 w-3" /> letzten Vorgang rückgängig
      </button>
    </div>
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto p-3">
    {#if batches.length === 0}
      <div class="flex h-full flex-col items-center justify-center gap-2 text-center">
        <History class="h-7 w-7 text-[#3b475c]" />
        <div class="text-[12px] text-muted">Noch keine Sortiervorgänge protokolliert.</div>
      </div>
    {:else}
      <div class="mx-auto flex max-w-[1400px] flex-col gap-2">
        {#each batches as batch (batch.id)}
          <div class="card overflow-hidden">
            <button
              class="flex w-full items-center gap-3 px-3 py-2 text-left hover:bg-panel2"
              onclick={() => (expanded = expanded === batch.id ? null : batch.id)}
            >
              <ChevronRight
                class="h-3.5 w-3.5 flex-none text-muted transition-transform"
                style="transform: rotate({expanded === batch.id ? 90 : 0}deg)"
              />
              <span class="num w-[130px] flex-none text-[11px] text-muted">
                {formatDate(batch.startedMs)}
              </span>
              {#if batch.dryRun}
                <span class="badge badge-accent"><FlaskConical class="h-2.5 w-2.5" /> Simulation</span>
              {:else if batch.undone}
                <span class="badge badge-muted">rückgängig</span>
              {:else}
                <span class="badge badge-ok">angewendet</span>
              {/if}
              <span class="badge badge-muted">{batch.structure}</span>
              <span class="flex items-center gap-2 text-[11px] num">
                {#if batch.moved}<span class="text-ok">{formatCount(batch.moved)} verschoben</span>{/if}
                {#if batch.copied}<span class="text-accent">{formatCount(batch.copied)} kopiert</span>{/if}
                {#if batch.deleted}<span class="text-danger">{formatCount(batch.deleted)} gelöscht</span>{/if}
                {#if batch.skipped}<span class="text-muted">{formatCount(batch.skipped)} übersprungen</span>{/if}
                {#if batch.errors}
                  <span class="text-danger">{formatCount(batch.errors)} Fehler</span>
                {/if}
              </span>
              <span class="ml-auto flex items-center gap-3">
                <span class="chip num">{formatBytes(batch.bytes)}</span>
                <span class="text-[10.5px] text-muted num">{formatDuration(batch.durationMs)}</span>
                <span class="mono max-w-[280px] truncate text-[10px] text-[#5f6d84]" title={batch.target}>
                  → {batch.target}
                </span>
              </span>
            </button>

            {#if expanded === batch.id}
              <div class="border-t border-line">
                <div class="flex items-center gap-2 px-3 py-1.5 text-[10.5px] text-muted">
                  <span class="mono truncate">Quelle: {batch.root}</span>
                  <span class="ml-auto">{formatCount(batch.ops.length)} Aktionen</span>
                  {#if !batch.dryRun && !batch.undone}
                    <button class="btn btn-sm btn-danger" onclick={() => undo(batch)} disabled={app.busy}>
                      <Undo2 class="h-3 w-3" /> diesen Vorgang zurückrollen
                    </button>
                  {/if}
                </div>
                <div class="h-[320px] border-t border-line/60">
                  <VirtualList items={opsOf(batch)} itemHeight={26} empty="Keine Einträge">
                    {#snippet row(op)}
                      <div class="flex h-[26px] items-center gap-2 border-b border-line/30 px-3 text-[10.5px]">
                        {#if actionMeta[op.action]}
                          {@const am = actionMeta[op.action]}
                          <am.icon class="h-3 w-3 flex-none {am.class}" />
                          <span class="w-[110px] flex-none truncate {am.class}">{am.label}</span>
                        {:else}
                          <span class="w-[126px] flex-none truncate text-muted">{op.action}</span>
                        {/if}
                        <span class="mono truncate text-[#93a2ba]" title={op.src}>{basename(op.src)}</span>
                        {#if op.dst}
                          <span class="text-[#4b586e]">→</span>
                          <span class="mono truncate text-[#6f7f96]" title={op.dst}>{op.dst}</span>
                        {/if}
                        {#if op.error}
                          <span class="truncate text-danger" title={op.error}>{op.error}</span>
                        {/if}
                        <span class="ml-auto flex-none num text-[10px] text-muted">{formatBytes(op.size)}</span>
                      </div>
                    {/snippet}
                  </VirtualList>
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
