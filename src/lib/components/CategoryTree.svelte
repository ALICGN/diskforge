<script>
  import { app } from "../store.svelte.js";
  import { formatBytes, formatCount, pct, extLabel } from "../format.js";
  import { ChevronRight, Check } from "lucide-svelte";

  let { summary = null, mode = "categories", filter = "" } = $props();

  let needle = $derived(filter.trim().toLowerCase());
  let totalSize = $derived(summary?.bytes || 1);
  let maxCatSize = $derived(Math.max(1, ...(summary?.categories ?? []).map((c) => c.size)));

  let categories = $derived(
    (summary?.categories ?? [])
      .map((c) => ({
        ...c,
        exts: c.exts.filter(
          (e) =>
            !needle ||
            e.ext.toLowerCase().includes(needle) ||
            c.name.toLowerCase().includes(needle),
        ),
      }))
      .filter((c) => !needle || c.exts.length > 0 || c.name.toLowerCase().includes(needle)),
  );

  let flat = $derived(
    (summary?.categories ?? [])
      .flatMap((c) => c.exts.map((e) => ({ ...e, categoryName: c.name, color: c.color })))
      .filter(
        (e) =>
          !needle ||
          e.ext.toLowerCase().includes(needle) ||
          e.categoryName.toLowerCase().includes(needle),
      )
      .sort((a, b) => b.size - a.size),
  );

  let maxExtSize = $derived(Math.max(1, ...flat.map((e) => e.size)));

  function catChecked(c) {
    return app.selCats.includes(c.name);
  }
  function catPartial(c) {
    return !catChecked(c) && c.exts.some((e) => app.selExts.includes(e.ext.toLowerCase()));
  }
  function extKey(e) {
    return e.ext === "Ohne_Endung" ? "" : e.ext.toLowerCase();
  }
  function extChecked(e) {
    const k = extKey(e);
    return app.selExts.includes(k) || app.selExts.includes(e.ext.toLowerCase());
  }
  function toggleCat(c) {
    app.toggleCat(c.name);
  }
  function toggleCatOpen(c) {
    app.openCats = { ...app.openCats, [c.name]: !app.openCats[c.name] };
  }
</script>

{#if !summary}
  <div class="flex h-full items-center justify-center text-[12px] text-muted">
    Noch keine Daten — starte einen Scan.
  </div>
{:else if mode === "extensions"}
  <div class="h-full overflow-y-auto">
    <div
      class="sticky top-0 z-10 grid grid-cols-[24px_1fr_120px_110px_90px] gap-2 border-b border-line bg-panel px-3 py-1.5 text-[10.5px] font-semibold uppercase tracking-wider text-muted"
    >
      <span></span><span>Endung</span><span>Kategorie</span><span class="text-right">Dateien</span
      ><span class="text-right">Größe</span>
    </div>
    {#each flat as e (e.categoryName + e.ext)}
      <button
        class="grid w-full grid-cols-[24px_1fr_120px_110px_90px] items-center gap-2 border-b border-line/50 px-3 py-1.5 text-left hover:bg-panel2"
        onclick={() => app.toggleExt(extKey(e))}
      >
        <span
          class="flex h-4 w-4 items-center justify-center rounded border"
          class:border-accent={extChecked(e)}
          class:bg-accent={extChecked(e)}
          class:border-line={!extChecked(e)}
        >
          {#if extChecked(e)}<Check class="h-3 w-3 text-white" />{/if}
        </span>
        <span class="flex items-center gap-2">
          <span class="mono rounded bg-panel3 px-1.5 py-0.5 text-[11px] text-ink"
            >{extLabel(e.ext)}</span
          >
          <span class="h-1.5 w-16 overflow-hidden rounded-full bg-panel3">
            <span
              class="block h-full rounded-full"
              style="width:{pct(e.size, maxExtSize)}%; background:{e.color || '#4f8cff'}"
            ></span>
          </span>
        </span>
        <span class="truncate text-[11.5px] text-muted">{e.categoryName}</span>
        <span class="num text-right text-[11.5px]">{formatCount(e.count)}</span>
        <span class="num text-right text-[11.5px] text-muted">{formatBytes(e.size)}</span>
      </button>
    {/each}
    {#if flat.length === 0}
      <div class="p-8 text-center text-[12px] text-muted">Keine Endungen gefunden</div>
    {/if}
  </div>
{:else}
  <div class="h-full overflow-y-auto">
    {#each categories as c (c.name)}
      <div class="border-b border-line/60">
        <div class="flex items-center gap-2.5 px-3 py-2 hover:bg-panel2">
          <button
            class="btn btn-icon btn-ghost h-5 w-5"
            onclick={() => toggleCatOpen(c)}
            aria-label="aufklappen"
          >
            <ChevronRight
              class="h-3.5 w-3.5 transition-transform"
              style="transform: rotate({app.openCats[c.name] ? 90 : 0}deg)"
            />
          </button>
          <button
            class="flex h-4 w-4 flex-none items-center justify-center rounded border"
            class:border-accent={catChecked(c) || catPartial(c)}
            class:bg-accent={catChecked(c)}
            class:border-line={!catChecked(c) && !catPartial(c)}
            onclick={() => toggleCat(c)}
            aria-label="Kategorie wählen"
          >
            {#if catChecked(c)}<Check class="h-3 w-3 text-white" />{:else if catPartial(c)}<span
                class="h-1.5 w-1.5 rounded-sm bg-accent"
              ></span>{/if}
          </button>
          <span class="h-2.5 w-2.5 flex-none rounded-full" style="background:{c.color}"></span>
          <span class="text-[12.5px] font-medium">{c.name}</span>
          <span class="text-[10.5px] text-muted">{c.exts.length} Endungen</span>
          <span class="ml-2 h-1.5 w-40 overflow-hidden rounded-full bg-panel3">
            <span
              class="block h-full rounded-full"
              style="width:{pct(c.size, maxCatSize)}%; background:{c.color}"
            ></span>
          </span>
          <span class="ml-auto num text-[11.5px] text-muted">{formatCount(c.count)} Dateien</span>
          <span class="num w-20 text-right text-[11.5px] font-medium">{formatBytes(c.size)}</span>
          <span class="num w-12 text-right text-[10.5px] text-[#5f6d84]"
            >{pct(c.size, totalSize).toFixed(1)}%</span
          >
        </div>
        {#if app.openCats[c.name]}
          <div class="bg-[#0d121b]/60 pb-1">
            {#each c.exts as e (c.name + e.ext)}
              <button
                class="grid w-full grid-cols-[24px_1fr_110px_100px_60px] items-center gap-2 py-1 pl-11 pr-4 text-left hover:bg-panel2/70"
                onclick={() => app.toggleExt(extKey(e))}
              >
                <span
                  class="flex h-3.5 w-3.5 items-center justify-center rounded border"
                  class:border-accent={extChecked(e)}
                  class:bg-accent={extChecked(e)}
                  class:border-line={!extChecked(e)}
                >
                  {#if extChecked(e)}<Check class="h-2.5 w-2.5 text-white" />{/if}
                </span>
                <span class="mono text-[11px] text-[#c3cfe2]">{extLabel(e.ext)}</span>
                <span class="num text-right text-[11px] text-muted">{formatCount(e.count)}</span>
                <span class="num text-right text-[11px] text-muted">{formatBytes(e.size)}</span>
                <span class="num text-right text-[10px] text-[#5f6d84]"
                  >{pct(e.size, c.size || 1).toFixed(0)}%</span
                >
              </button>
            {/each}
            {#if c.exts.length === 0}
              <div class="py-1.5 pl-11 text-[11px] text-muted">keine Dateien</div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
    {#if categories.length === 0}
      <div class="p-8 text-center text-[12px] text-muted">Keine Treffer für „{filter}“</div>
    {/if}
  </div>
{/if}
