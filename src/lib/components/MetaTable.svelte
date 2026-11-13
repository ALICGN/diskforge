<script>
  import { formatBytes, formatDate, formatCount, formatDuration } from "../format.js";
  import { Copy, CalendarClock, Sparkles, HardDrive, Clock, Film, Image as ImageIcon } from "lucide-svelte";

  let { meta, oldest = false, newest = false, compact = false } = $props();

  let rows = $derived(
    meta
      ? [
          {
            label: "Größe",
            value: `${formatCount(meta.size)} B · ${formatBytes(meta.size, 2)}`,
            icon: HardDrive,
          },
          {
            label: "Auflösung",
            value:
              meta.width && meta.height ? `${meta.width} × ${meta.height} px` : "—",
            icon: meta.kind === "video" ? Film : ImageIcon,
          },
          {
            label: "Dauer / Bitrate",
            value:
              meta.durationS != null
                ? `${formatDuration((meta.durationS || 0) * 1000)}${meta.bitrateKbps ? ` · ${formatCount(meta.bitrateKbps)} kbit/s` : ""}`
                : "—",
            icon: Clock,
            hidden: meta.kind === "image" || meta.kind === "file",
          },
          {
            label: "Geändert (mtime)",
            value: formatDate(meta.mtimeMs) ?? "—",
            icon: CalendarClock,
            tone: oldest ? "old" : newest ? "new" : null,
          },
          {
            label: "Erstellt (btime)",
            value: formatDate(meta.btimeMs) ?? "—",
            icon: Sparkles,
            hidden: !meta.btimeMs,
          },
          {
            label: "Inode geändert (ctime)",
            value: formatDate(meta.ctimeMs) ?? "—",
            icon: Sparkles,
            hidden: !meta.ctimeMs,
          },
        ]
      : [],
  );
</script>

{#if meta}
  <div class="flex flex-col gap-1">
    <div class="mono flex items-start gap-1.5 break-all text-[10px] leading-relaxed text-[#7d8ba1]">
      <Copy class="mt-0.5 h-2.5 w-2.5 flex-none" />
      <span class="select-text">{meta.path}</span>
    </div>
    <div class="mt-1 grid grid-cols-1 gap-x-3">
      {#each rows.filter((r) => !r.hidden) as r}
        <div class="flex items-center gap-2 border-b border-line/40 py-[3px] last:border-0">
          <r.icon class="h-3 w-3 flex-none text-[#5f6d84]" />
          <span class="text-[10.5px] text-muted">{r.label}</span>
          <span
            class="num ml-auto text-right text-[11px]"
            class:text-ok={r.tone === "old"}
            class:text-warn={r.tone === "new"}>{r.value}</span
          >
        </div>
      {/each}
    </div>
    {#if meta.error}
      <div class="text-[10px] text-warn">{meta.error}</div>
    {/if}
  </div>
{/if}
