<script>
  import { app } from "../store.svelte.js";
  import { formatBytes, formatRate, formatDuration, pct } from "../format.js";
  import { X, Loader } from "lucide-svelte";

  let { compact = false } = $props();

  let p = $derived(app.progress);
  let determinate = $derived(!!p && (p.totalBytes > 0 || p.phase === "moving" || p.phase === "simulating"));
  let percent = $derived(determinate ? pct(p?.doneBytes ?? 0, p?.totalBytes ?? 1) : 0);
  let speedText = $derived(
    !p || p.speed <= 0
      ? ""
      : p.phase === "scanning"
        ? `${Math.round(p.speed).toLocaleString("de-DE")} Dateien/s`
        : formatRate(p.speed),
  );
  let label = $derived(
    p?.phase === "hashing"
      ? "BLAKE3-Hashing"
      : p?.phase === "moving"
        ? "Sortieren"
        : p?.phase === "simulating"
          ? "Simulation"
          : p?.phase === "done"
            ? "Fertig"
            : "Scannen",
  );
</script>

{#if p && p.phase !== "done"}
  <div class="card px-4 py-3" class:py-2={compact}>
    <div class="mb-2 flex items-center gap-3">
      <Loader class="h-3.5 w-3.5 animate-spin text-accent" />
      <span class="text-[12px] font-semibold tracking-wide text-ink">{label}</span>
      {#if p.message}
        <span class="truncate text-[11px] text-muted">{p.message}</span>
      {/if}
      <div class="ml-auto flex items-center gap-3 text-[11px] text-muted num">
        <span>{formatBytes(p.bytes)}</span>
        {#if p.phase === "scanning"}
          <span>{p.files.toLocaleString("de-DE")} Dateien</span>
        {:else if p.totalBytes > 0}
          <span>{formatBytes(p.doneBytes)} / {formatBytes(p.totalBytes)}</span>
        {/if}
        {#if speedText}
          <span>{speedText}</span>
        {/if}
        {#if p.etaS > 0 && isFinite(p.etaS)}
          <span>ETA {formatDuration(p.etaS * 1000)}</span>
        {/if}
        <span>{formatDuration(p.elapsedMs)}</span>
        <button class="btn btn-sm btn-danger" onclick={() => app.cancel()}>
          <X class="h-3 w-3" /> Abbrechen
        </button>
      </div>
    </div>
    <div class="h-1.5 w-full overflow-hidden rounded-full bg-panel3">
      {#if determinate}
        <div
          class="h-full rounded-full bg-gradient-to-r from-accent to-cyan transition-[width] duration-200"
          style="width:{percent.toFixed(1)}%"
        ></div>
      {:else}
        <div class="h-full w-1/3 rounded-full bg-gradient-to-r from-accent/0 via-cyan to-accent/0 df-indet"></div>
      {/if}
    </div>
    {#if p.current}
      <div class="mono mt-1.5 truncate text-[10.5px] text-[#5f6d84]">{p.current}</div>
    {/if}
  </div>
{/if}
