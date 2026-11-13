<script>
  let { items = [], itemHeight = 34, row, empty = "Keine Einträge", keyOf = null } = $props();

  let el = $state(null);
  let scrollTop = $state(0);
  let viewH = $state(400);
  const overscan = 6;

  let start = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - overscan));
  let visibleCount = $derived(Math.ceil(viewH / itemHeight) + overscan * 2);
  let visible = $derived(items.slice(start, start + visibleCount));

  $effect(() => {
    if (!el) return;
    const measure = () => {
      viewH = el.clientHeight || 400;
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  });
</script>

<div
  class="h-full overflow-y-auto overflow-x-hidden"
  bind:this={el}
  onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
>
  {#if items.length === 0}
    <div class="flex h-full items-center justify-center p-8 text-center text-[12px] text-muted">
      {empty}
    </div>
  {:else}
    <div style="height:{items.length * itemHeight}px; position:relative">
      {#each visible as item, i (start + i)}
        <div
          style="position:absolute; left:0; right:0; top:{(start + i) * itemHeight}px; height:{itemHeight}px"
        >
          {@render row(item, start + i)}
        </div>
      {/each}
    </div>
  {/if}
</div>
