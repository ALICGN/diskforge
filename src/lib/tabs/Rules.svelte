<script>
  import { app } from "../store.svelte.js";
  import {
    Plus,
    Trash2,
    Save,
    RotateCcw,
    Tag,
    ArrowRightLeft,
    AlertTriangle,
    Palette,
  } from "lucide-svelte";

  let draft = $state([]);
  let dirty = $state(false);
  let newCat = $state("");
  let newColor = $state("#4f8cff");
  let moveExt = $state("");
  let moveTo = $state("");
  let extInputs = $state({});

  const palette = ["#38bdf8", "#a78bfa", "#f472b6", "#fbbf24", "#fb923c", "#4ade80", "#94a3b8", "#ef4444", "#22d3ee", "#e879f9"];

  $effect(() => {
    const incoming = app.categories?.categories ?? [];
    if (!dirty) draft = structuredClone($state.snapshot(incoming));
  });

  let conflicts = $derived.by(() => {
    const seen = new Map();
    const list = [];
    for (const c of draft) {
      for (const e of c.exts) {
        if (seen.has(e)) list.push({ ext: e, a: seen.get(e), b: c.name });
        else seen.set(e, c.name);
      }
    }
    return list;
  });

  let totalExts = $derived(draft.reduce((a, c) => a + c.exts.length, 0));

  function normExt(v) {
    return String(v || "")
      .trim()
      .toLowerCase()
      .replace(/^\.+/, "")
      .replace(/\s+/g, "");
  }

  function addExt(cat) {
    const raw = extInputs[cat.name] || "";
    for (const part of raw.split(/[,\s]+/)) {
      const e = normExt(part);
      if (!e) continue;
      const owner = draft.find((c) => c.exts.includes(e));
      if (owner && owner.name !== cat.name) {
        app.toast(`„${e}“ ist bereits bei ${owner.name} — dort entfernen, um es zu verschieben.`, "warn");
        continue;
      }
      if (!cat.exts.includes(e)) cat.exts.push(e);
    }
    extInputs = { ...extInputs, [cat.name]: "" };
    dirty = true;
  }

  function removeExt(cat, ext) {
    cat.exts = cat.exts.filter((e) => e !== ext);
    dirty = true;
  }

  function moveExtNow() {
    const e = normExt(moveExt);
    const target = draft.find((c) => c.name === moveTo);
    if (!e || !target) return;
    for (const c of draft) c.exts = c.exts.filter((x) => x !== e);
    if (!target.exts.includes(e)) target.exts.push(e);
    target.exts = [...target.exts].sort();
    moveExt = "";
    dirty = true;
  }

  function addCategory() {
    const name = newCat.trim();
    if (!name) return;
    if (draft.some((c) => c.name.toLowerCase() === name.toLowerCase())) {
      app.toast("Kategorie existiert bereits", "warn");
      return;
    }
    draft.splice(draft.length - 1, 0, { name, color: newColor, exts: [], builtin: false });
    newCat = "";
    dirty = true;
  }

  function removeCategory(cat) {
    if (cat.name === "Sonstiges") return;
    draft = draft.filter((c) => c.name !== cat.name);
    dirty = true;
  }

  async function save() {
    const list = draft.map((c) => ({ ...c, exts: [...c.exts].sort() }));
    const res = await app.saveCategories(list);
    if (res) {
      dirty = false;
      draft = structuredClone(res.categories);
    }
  }

  async function reset() {
    const res = await app.resetCategories();
    if (res) {
      dirty = false;
      draft = structuredClone(res.categories);
    }
  }
</script>

<div class="flex h-full flex-col overflow-hidden">
  <div class="flex flex-wrap items-center gap-2 border-b border-line bg-panel px-4 py-2.5">
    <Tag class="h-4 w-4 text-accent" />
    <h2 class="text-[13px] font-semibold">Kategorie-Regeln</h2>
    <span class="text-[11px] text-muted">
      {draft.length} Kategorien · {totalExts} Endungen · gespeichert in
      <span class="mono">~/.config/diskforge/categories.json</span>
    </span>
    {#if dirty}<span class="badge badge-warn">ungespeichert</span>{/if}
    <div class="ml-auto flex items-center gap-2">
      <button class="btn btn-sm btn-ghost" onclick={reset}>
        <RotateCcw class="h-3 w-3" /> Standard
      </button>
      <button class="btn btn-primary btn-sm" onclick={save} disabled={!dirty}>
        <Save class="h-3 w-3" /> Speichern
      </button>
    </div>
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto p-4">
    <div class="mx-auto flex max-w-[1400px] flex-col gap-3">
      {#if conflicts.length}
        <div class="card flex items-start gap-2 border-warn/40 bg-warn/5 px-3 py-2 text-[11.5px] text-warn">
          <AlertTriangle class="mt-0.5 h-3.5 w-3.5 flex-none" />
          <div>
            Doppelt zugewiesene Endungen (die erste Kategorie gewinnt):
            {#each conflicts as c}
              <span class="chip mono ml-1">{c.ext}: {c.a} / {c.b}</span>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Neue Kategorie + Verschieben -->
      <div class="card flex flex-wrap items-end gap-2 p-3">
        <div class="flex flex-col gap-1">
          <label class="text-[10px] uppercase tracking-wider text-muted" for="newcat">Neue Kategorie</label>
          <input
            id="newcat"
            class="input w-52"
            placeholder="z.B. 3D-Druck"
            bind:value={newCat}
            onkeydown={(e) => e.key === "Enter" && addCategory()}
          />
        </div>
        <div class="flex flex-col gap-1">
          <span class="text-[10px] uppercase tracking-wider text-muted">Farbe</span>
          <div class="flex items-center gap-1">
            {#each palette as p}
              <button
                class="h-5 w-5 rounded-full border-2"
                class:border-white={newColor === p}
                class:border-transparent={newColor !== p}
                style="background:{p}"
                onclick={() => (newColor = p)}
                aria-label="Farbe {p}"
              ></button>
            {/each}
          </div>
        </div>
        <button class="btn" onclick={addCategory}><Plus class="h-3.5 w-3.5" /> Anlegen</button>

        <div class="mx-2 h-9 w-px bg-line"></div>

        <div class="flex flex-col gap-1">
          <label class="text-[10px] uppercase tracking-wider text-muted" for="moveext">Endung verschieben</label>
          <div class="flex items-center gap-1.5">
            <input id="moveext" class="input mono w-24" placeholder="stl" bind:value={moveExt} />
            <ArrowRightLeft class="h-3.5 w-3.5 text-muted" />
            <select class="input w-44" bind:value={moveTo}>
              <option value="">Ziel…</option>
              {#each draft as c}
                <option value={c.name}>{c.name}</option>
              {/each}
            </select>
            <button class="btn btn-sm" onclick={moveExtNow} disabled={!moveExt || !moveTo}>verschieben</button>
          </div>
        </div>
      </div>

      <!-- Kategorien -->
      <div class="grid grid-cols-1 gap-3 xl:grid-cols-2">
        {#each draft as cat (cat.name)}
          <div class="card flex flex-col gap-2 p-3">
            <div class="flex items-center gap-2">
              <span class="h-2.5 w-2.5 rounded-full" style="background:{cat.color}"></span>
              <input
                class="input h-7 flex-1 text-[12px] font-medium"
                bind:value={cat.name}
                oninput={() => (dirty = true)}
                disabled={cat.name === "Sonstiges"}
              />
              <span class="badge badge-muted">{cat.exts.length}</span>
              {#if cat.builtin}<span class="badge badge-accent">Standard</span>{/if}
              <button
                class="btn btn-icon btn-ghost text-danger"
                onclick={() => removeCategory(cat)}
                disabled={cat.name === "Sonstiges"}
                title="Kategorie löschen"
              >
                <Trash2 class="h-3.5 w-3.5" />
              </button>
            </div>

            {#if cat.name === "Sonstiges"}
              <p class="text-[10.5px] text-muted">
                Auffangkategorie: alle Endungen, die keiner anderen Kategorie zugeordnet sind, sowie
                Dateien ohne Endung.
              </p>
            {:else}
              <div class="flex flex-wrap gap-1">
                {#each cat.exts as ext (ext)}
                  <span class="chip mono group">
                    {ext}
                    <button
                      class="text-[#7d8ba1] hover:text-danger"
                      onclick={() => removeExt(cat, ext)}
                      aria-label="{ext} entfernen">×</button
                    >
                  </span>
                {/each}
                {#if cat.exts.length === 0}
                  <span class="text-[11px] text-muted">keine Endungen zugewiesen</span>
                {/if}
              </div>
              <div class="flex items-center gap-1.5">
                <input
                  class="input mono h-7 flex-1 text-[11px]"
                  placeholder="Endungen, komma-getrennt (jpg, png)"
                  value={extInputs[cat.name] ?? ""}
                  oninput={(e) => (extInputs = { ...extInputs, [cat.name]: e.currentTarget.value })}
                  onkeydown={(e) => e.key === "Enter" && addExt(cat)}
                />
                <button class="btn btn-sm" onclick={() => addExt(cat)}><Plus class="h-3 w-3" /></button>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>
