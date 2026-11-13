<script>
  import { app } from "../store.svelte.js";
  import { formatBytes, basename, dirname } from "../format.js";
  import MetaTable from "./MetaTable.svelte";
  import { Eye, Check, Trash2, ImageOff, Film, FileQuestion, Play, Pause, MoveRight } from "lucide-svelte";

  let { meta, mark = false, oldest = false, newest = false, onMark, onZoom } = $props();

  let mediaKind = $derived(
    meta?.kind === "image" && meta.width ? "image" : meta?.kind === "video" ? "video" : "none",
  );
  let src = $derived(meta ? app.assetUrl(meta.path) : "");
  let failed = $state(false);
  $effect(() => {
    // reset error state when the path changes
    meta?.path;
    failed = false;
  });
</script>

<div
  class="card flex flex-col overflow-hidden"
  class:border-bad={mark}
  class:border-good={!mark && oldest}
>
  <div class="flex items-center gap-2 border-b border-line px-2.5 py-1.5">
    <span class="truncate text-[11.5px] font-medium" title={meta?.name}>{basename(meta?.path)}</span>
    {#if oldest}
      <span class="badge badge-ok">ältestes</span>
    {/if}
    {#if newest}
      <span class="badge badge-warn">neuestes</span>
    {/if}
    {#if mark}
      <span class="badge badge-danger">aussortieren</span>
    {/if}
    <div class="ml-auto flex items-center gap-1">
      {#if mediaKind === "image"}
        <button class="btn btn-icon btn-ghost" title="Vollbild" onclick={() => onZoom?.(meta)}>
          <Eye class="h-3.5 w-3.5" />
        </button>
      {/if}
      <button
        class="btn btn-icon btn-ghost"
        class:text-danger={mark}
        class:text-ok={!mark}
        title={mark ? "behalten" : "als Duplikat markieren"}
        onclick={() => onMark?.(meta, !mark)}
      >
        {#if mark}<Trash2 class="h-3.5 w-3.5" />{:else}<Check class="h-3.5 w-3.5" />{/if}
      </button>
    </div>
  </div>

  <div class="relative flex h-[260px] items-center justify-center overflow-hidden bg-black/60">
    {#if mediaKind === "image" && !failed}
      <button class="contents" onclick={() => onZoom?.(meta)} title="Vollbild">
        <img
          src={src}
          alt={meta?.name}
          class="max-h-full max-w-full object-contain"
          onerror={() => (failed = true)}
        />
      </button>
    {:else if mediaKind === "video"}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        {src}
        class="max-h-full max-w-full"
        controls
        preload="metadata"
        playsinline
        data-compare-video
        onerror={() => (failed = true)}
      ></video>
    {:else}
      <div class="flex flex-col items-center gap-2 text-[#4b586e]">
        {#if failed}
          <ImageOff class="h-7 w-7" />
          <span class="text-[10.5px]">Vorschau nicht möglich</span>
        {:else if meta?.kind === "video"}
          <Film class="h-7 w-7" />
          <span class="text-[10.5px]">{meta?.ext?.toUpperCase() || "Video"}</span>
        {:else}
          <FileQuestion class="h-7 w-7" />
          <span class="text-[10.5px]">{meta?.ext?.toUpperCase() || "Datei"}</span>
        {/if}
      </div>
    {/if}
    <span class="badge badge-muted absolute left-2 top-2">{formatBytes(meta?.size ?? 0)}</span>
  </div>

  <div class="flex min-h-[132px] flex-col gap-1 px-2.5 py-2">
    <div class="mb-0.5 flex items-center gap-1.5 text-[10px] text-muted">
      <MoveRight class="h-3 w-3" />
      <span class="mono truncate" title={meta?.path}>{dirname(meta?.path)}</span>
    </div>
    <MetaTable {meta} {oldest} {newest} />
  </div>
</div>
