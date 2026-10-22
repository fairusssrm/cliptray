<script lang="ts">
  import { onMount, tick } from "svelte";
  import * as api from "./lib/api";
  import type { Clip, Kind } from "./lib/api";
  import Icon from "./lib/Icon.svelte";
  import type { IconName } from "./lib/icons";

  type Filter = "all" | "pinned" | Kind;
  type Theme = "light" | "dark" | "system";
  /** "history" menyisakan item yang disematkan, "all" menghapus semuanya. */
  type ClearMode = "history" | "all";

  const KIND_LABEL: Record<Kind, string> = {
    text: "Teks",
    link: "Link",
    code: "Kode",
    color: "Warna",
  };

  const FILTERS: { id: Filter; label: string }[] = [
    { id: "all", label: "Semua" },
    { id: "pinned", label: "Pin" },
    { id: "text", label: "Teks" },
    { id: "link", label: "Link" },
    { id: "code", label: "Kode" },
    { id: "color", label: "Warna" },
  ];

  const THEMES: { id: Theme; label: string; icon: IconName }[] = [
    { id: "light", label: "Terang", icon: "sun" },
    { id: "dark", label: "Gelap", icon: "moon" },
    { id: "system", label: "Sistem", icon: "monitor" },
  ];

  const SHORTCUTS: [string, string][] = [
    ["Ctrl+Shift+V", "Buka / tutup popup"],
    ["↑ ↓", "Pilih item"],
    ["Enter", "Salin item"],
    ["Ctrl+1–9", "Salin cepat"],
    ["Ctrl+P", "Sematkan / lepas"],
    ["Ctrl+D", "Hapus item"],
    ["Tab", "Ganti filter"],
    ["Esc", "Tutup"],
  ];

  const THEME_KEY = "cliptray-theme";

  function readTheme(): Theme {
    try {
      const saved = localStorage.getItem(THEME_KEY);
      if (saved === "light" || saved === "dark" || saved === "system") return saved;
    } catch {}
    return "system";
  }

  let items = $state<Clip[]>([]);
  let query = $state("");
  let filter = $state<Filter>("all");
  let selected = $state(0);
  let paused = $state(false);
  let autostart = $state(false);
  let theme = $state<Theme>(readTheme());
  let settingsOpen = $state(false);
  let confirming = $state<ClearMode | null>(null);
  let ctrlHeld = $state(false);
  let now = $state(Date.now());
  let listEl: HTMLUListElement | undefined = $state();
  let searchEl: HTMLInputElement | undefined = $state();

  const visible = $derived(
    items.filter((c) =>
      filter === "all" ? true : filter === "pinned" ? c.pinned : c.kind === filter,
    ),
  );

  const pinnedCount = $derived(items.filter((c) => c.pinned).length);

  const subtitle = $derived(
    query.trim()
      ? `${items.length} hasil pencarian`
      : `${items.length} tersimpan · ${pinnedCount} disematkan`,
  );

  $effect(() => {
    const root = document.documentElement;
    if (theme === "system") delete root.dataset.theme;
    else root.dataset.theme = theme;
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch {}
  });

  let requestId = 0;
  async function load() {
    const id = ++requestId;
    const result = await api.getHistory(query);
    if (id !== requestId) return;
    items = result;
    now = Date.now();
    if (selected >= visible.length) selected = Math.max(0, visible.length - 1);
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function onSearch() {
    clearTimeout(searchTimer);
    selected = 0;
    searchTimer = setTimeout(load, 120);
  }

  function clearSearch() {
    query = "";
    selected = 0;
    load();
    searchEl?.focus();
  }

  function setFilter(f: Filter) {
    filter = f;
    selected = 0;
    scrollToSelected();
  }

  async function scrollToSelected() {
    await tick();
    listEl?.querySelector(`[data-index="${selected}"]`)?.scrollIntoView({ block: "nearest" });
  }

  function move(delta: number) {
    if (!visible.length) return;
    selected = Math.min(Math.max(selected + delta, 0), visible.length - 1);
    scrollToSelected();
  }

  async function closeSettings() {
    settingsOpen = false;
    confirming = null;
    await tick();
    searchEl?.focus();
  }

  const copy = (c: Clip) => api.copyItem(c.id);
  const togglePin = (c: Clip) => api.setPinned(c.id, !c.pinned);
  const remove = (c: Clip) => api.deleteItem(c.id);

  async function togglePause() {
    paused = await api.setPaused(!paused);
  }

  async function toggleAutostart() {
    autostart = await api.setAutostart(!autostart);
  }

  /** Klik pertama meminta konfirmasi (3 detik), klik kedua benar-benar menghapus. */
  let confirmTimer: ReturnType<typeof setTimeout> | undefined;
  function requestClear(mode: ClearMode) {
    clearTimeout(confirmTimer);
    if (confirming !== mode) {
      confirming = mode;
      confirmTimer = setTimeout(() => (confirming = null), 3000);
      return;
    }
    confirming = null;
    api.clearAll(mode === "all");
  }

  /** Teks biasa diringkas ke satu paragraf; kode dirapikan indentasinya. */
  function display(c: Clip): string {
    if (c.kind !== "code") return c.preview.replace(/\s+/g, " ").trim();
    const lines = c.preview.replace(/^\s*\n/, "").trimEnd().split("\n");
    const indents = lines.filter((l) => l.trim()).map((l) => l.match(/^[ \t]*/)![0].length);
    const cut = indents.length ? Math.min(...indents) : 0;
    return lines.map((l) => l.slice(cut)).join("\n");
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Control") ctrlHeld = true;
    if (e.ctrlKey && e.key.toLowerCase() === "p") e.preventDefault(); // cegah dialog print WebView2

    if (settingsOpen) {
      if (e.key === "Escape") {
        e.preventDefault();
        closeSettings();
      }
      return;
    }

    const current = visible[selected];
    if (e.key === "Escape") {
      e.preventDefault();
      if (confirming) confirming = null;
      else if (query) clearSearch();
      else api.hidePopup();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      move(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      move(-1);
    } else if (e.key === "Enter" && current) {
      e.preventDefault();
      copy(current);
    } else if (e.key === "Tab") {
      e.preventDefault();
      const i = FILTERS.findIndex((f) => f.id === filter);
      const next = (i + (e.shiftKey ? -1 : 1) + FILTERS.length) % FILTERS.length;
      setFilter(FILTERS[next].id);
    } else if (e.ctrlKey && e.key.toLowerCase() === "p") {
      if (current) togglePin(current);
    } else if (e.ctrlKey && e.key.toLowerCase() === "d") {
      e.preventDefault();
      if (current) remove(current);
    } else if (e.ctrlKey && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      const target = visible[Number(e.key) - 1];
      if (target) copy(target);
    }
  }

  function onKeyup(e: KeyboardEvent) {
    if (e.key === "Control") ctrlHeld = false;
  }

  function sectionTitle(i: number): string | null {
    if (filter === "pinned") return null;
    const c = visible[i];
    if (i === 0 && c.pinned) return "Disematkan";
    if (!c.pinned && i > 0 && visible[i - 1].pinned) return "Terbaru";
    return null;
  }

  function ago(ts: number): string {
    const s = Math.max(0, (now - ts) / 1000);
    if (s < 60) return "baru saja";
    const m = s / 60;
    if (m < 60) return `${Math.floor(m)} mnt lalu`;
    const h = m / 60;
    if (h < 24) return `${Math.floor(h)} jam lalu`;
    const d = h / 24;
    if (d < 7) return `${Math.floor(d)} hari lalu`;
    return new Date(ts).toLocaleDateString("id-ID", { day: "numeric", month: "short" });
  }

  function size(c: Clip): string {
    return c.lines > 1
      ? `${c.lines.toLocaleString("id-ID")} baris`
      : `${c.length.toLocaleString("id-ID")} karakter`;
  }

  onMount(() => {
    load();
    api.getPaused().then((p) => (paused = p));
    api.getAutostart().then((a) => (autostart = a));
    searchEl?.focus();
    const unlisteners = [
      api.onHistoryChanged(load),
      api.onPausedChanged((p) => (paused = p)),
      api.onAutostartChanged((a) => (autostart = a)),
      api.onPopupShown(async () => {
        query = "";
        filter = "all";
        selected = 0;
        ctrlHeld = false;
        settingsOpen = false;
        confirming = null;
        await load();
        scrollToSelected();
        searchEl?.focus();
      }),
    ];
    return () => unlisteners.forEach((p) => p.then((off) => off()));
  });
</script>

<svelte:window
  onkeydown={onKeydown}
  onkeyup={onKeyup}
  onblur={() => (ctrlHeld = false)}
  oncontextmenu={(e) => e.preventDefault()}
/>

<main>
  <!-- data-tauri-drag-region hanya berlaku pada elemen itu sendiri, jadi dipasang juga di teks judul. -->
  <header data-tauri-drag-region>
    {#if settingsOpen}
      <button class="icon-btn" onclick={closeSettings} aria-label="Kembali">
        <Icon name="back" />
      </button>
      <div class="titles" data-tauri-drag-region>
        <h1 data-tauri-drag-region>Pengaturan</h1>
        <p data-tauri-drag-region>Tema, pencatatan, dan data</p>
      </div>
    {:else}
      <div class="logo" data-tauri-drag-region><Icon name="clipboard" size={20} /></div>
      <div class="titles" data-tauri-drag-region>
        <h1 data-tauri-drag-region>ClipTray</h1>
        <p data-tauri-drag-region>{subtitle}</p>
      </div>
      {#if confirming !== "history"}
        <button
          class="status"
          class:paused
          onclick={togglePause}
          title={paused ? "Lanjutkan pencatatan" : "Jeda pencatatan"}
        >
          <span class="dot"></span>{paused ? "Dijeda" : "Merekam"}
        </button>
      {/if}
      {#if items.length || confirming === "history"}
        <button
          class="icon-btn clear-btn"
          class:confirm={confirming === "history"}
          onclick={() => requestClear("history")}
          title="Hapus semua riwayat (item yang disematkan tetap disimpan)"
          aria-label="Hapus semua riwayat"
        >
          <Icon name="trash" size={17} />
          {#if confirming === "history"}<span>Hapus semua?</span>{/if}
        </button>
      {/if}
      <button class="icon-btn" onclick={() => (settingsOpen = true)} aria-label="Pengaturan">
        <Icon name="settings" />
      </button>
    {/if}
  </header>

  {#if settingsOpen}
    <section class="settings">
      <div class="group">
        <h2>Tampilan</h2>
        <div class="segmented">
          {#each THEMES as t (t.id)}
            <button class:active={theme === t.id} onclick={() => (theme = t.id)}>
              <Icon name={t.icon} size={15} />
              {t.label}
            </button>
          {/each}
        </div>
      </div>

      <div class="group">
        <h2>Umum</h2>
        <div class="row">
          <div class="row-text">
            <strong>Rekam clipboard</strong>
            <span>Catat setiap teks yang di-copy</span>
          </div>
          <button
            class="switch"
            class:on={!paused}
            role="switch"
            aria-checked={!paused}
            aria-label="Rekam clipboard"
            onclick={togglePause}
          ><span></span></button>
        </div>
        <div class="row">
          <div class="row-text">
            <strong>Start with Windows</strong>
            <span>Jalankan otomatis saat login</span>
          </div>
          <button
            class="switch"
            class:on={autostart}
            role="switch"
            aria-checked={autostart}
            aria-label="Start with Windows"
            onclick={toggleAutostart}
          ><span></span></button>
        </div>
      </div>

      <div class="group">
        <h2>Data</h2>
        <div class="row">
          <div class="row-text">
            <strong>Hapus riwayat</strong>
            <span>Item yang disematkan tetap disimpan</span>
          </div>
          <button class="danger" class:confirm={confirming === "history"} onclick={() => requestClear("history")}>
            {confirming === "history" ? "Yakin?" : "Hapus"}
          </button>
        </div>
        <div class="row">
          <div class="row-text">
            <strong>Hapus semua</strong>
            <span>Termasuk item yang disematkan</span>
          </div>
          <button class="danger" class:confirm={confirming === "all"} onclick={() => requestClear("all")}>
            {confirming === "all" ? "Yakin?" : "Hapus semua"}
          </button>
        </div>
      </div>

      <div class="group">
        <h2>Pintasan keyboard</h2>
        <dl class="keys">
          {#each SHORTCUTS as [key, desc] (key)}
            <div>
              <dt>{desc}</dt>
              <dd><kbd>{key}</kbd></dd>
            </div>
          {/each}
        </dl>
      </div>
    </section>
  {:else}
    <div class="search">
      <Icon name="search" size={18} />
      <input
        bind:this={searchEl}
        bind:value={query}
        oninput={onSearch}
        placeholder="Cari di riwayat…"
        spellcheck="false"
        autocomplete="off"
        aria-label="Cari di riwayat"
      />
      {#if query}
        <button class="clear-query" onclick={clearSearch} aria-label="Hapus pencarian">
          <Icon name="x" size={13} />
        </button>
      {/if}
    </div>

    <nav class="segmented filters">
      {#each FILTERS as f (f.id)}
        <button class:active={filter === f.id} onclick={() => setFilter(f.id)}>{f.label}</button>
      {/each}
    </nav>

    {#if visible.length === 0}
      <section class="empty">
        <div class="empty-icon"><Icon name={query ? "search" : filter === "pinned" ? "pin" : "clipboard"} size={28} /></div>
        <h2>
          {#if query}Tidak ada yang cocok{:else if filter === "pinned"}Belum ada yang disematkan{:else if filter !== "all"}Belum ada item {KIND_LABEL[filter].toLowerCase()}{:else}Riwayat masih kosong{/if}
        </h2>
        <p>
          {#if query}Coba kata kunci lain atau ganti filter.
          {:else if filter === "pinned"}Tekan Ctrl+P pada item agar selalu ada di atas.
          {:else if paused}Pencatatan sedang dijeda.
          {:else}Copy teks apa saja, nanti muncul di sini.{/if}
        </p>
      </section>
    {:else}
      <ul class="list" role="listbox" aria-label="Riwayat clipboard" bind:this={listEl}>
        {#each visible as c, i (c.id)}
          {@const title = sectionTitle(i)}
          {#if title}
            <li class="section" role="presentation">{title}</li>
          {/if}
          <!-- Navigasi keyboard ditangani global di onKeydown. -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <li
            role="option"
            aria-selected={i === selected}
            data-index={i}
            class="card {c.kind}"
            class:selected={i === selected}
            onmouseenter={() => (selected = i)}
            onclick={() => copy(c)}
          >
            {#if c.kind === "color"}
              <div class="icon swatch" style:background={c.preview.trim()}></div>
            {:else}
              <div class="icon"><Icon name={c.kind} size={18} /></div>
            {/if}
            <div class="body">
              <p class="text">{display(c)}</p>
              <div class="meta">
                <span>{KIND_LABEL[c.kind]}</span>
                <span class="sep">·</span>
                <span>{ago(c.last_used_at)}</span>
                <span class="sep">·</span>
                <span>{size(c)}</span>
              </div>
            </div>
            <div class="side">
              {#if ctrlHeld && i < 9}
                <kbd>{i + 1}</kbd>
              {:else}
                <button
                  class="act"
                  class:on={c.pinned}
                  title={c.pinned ? "Lepas pin (Ctrl+P)" : "Sematkan (Ctrl+P)"}
                  aria-label={c.pinned ? "Lepas pin" : "Sematkan"}
                  onclick={(e) => { e.stopPropagation(); togglePin(c); }}
                ><Icon name="pin" size={15} /></button>
                <button
                  class="act"
                  title="Hapus (Ctrl+D)"
                  aria-label="Hapus"
                  onclick={(e) => { e.stopPropagation(); remove(c); }}
                ><Icon name="trash" size={15} /></button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</main>

<style>
  :global(:root) {
    --bg: #ecedf1;
    --surface: #ffffff;
    --ink: #121217;
    --on-ink: #ffffff;
    --muted: #64656f;
    --faint: #9a9ba4;
    --line: #e2e3e8;
    --ring: rgba(18, 18, 23, 0.55);
    --lavender: #f0dbf9;
    --sky: #dff3fb;
    --mint: #daf5e6;
    --butter: #fdf1c5;
    --rose: #fde2e2;
    --danger: #b91c1c;
    --accent-text: #8b3fd9;
    --accent-link: #0284c7;
    --accent-code: #15803d;
    --green: #22c55e;
    --amber: #d4a106;
    color-scheme: light;
    font-family: "Urbanist Variable", "Segoe UI", system-ui, sans-serif;
  }
  @media (prefers-color-scheme: dark) {
    :global(:root:not([data-theme="light"])) {
      --bg: #111114;
      --surface: #1e1e24;
      --ink: #f1f1f4;
      --on-ink: #111114;
      --muted: #a0a1ab;
      --faint: #6b6c76;
      --line: #2a2a31;
      --ring: rgba(241, 241, 244, 0.6);
      --lavender: #2a2135;
      --sky: #182a33;
      --mint: #172b22;
      --butter: #2e2915;
      --rose: #3b1f22;
      --danger: #fca5a5;
      --accent-text: #c9a3f5;
      --accent-link: #7dd3fc;
      --accent-code: #86efac;
      color-scheme: dark;
    }
  }
  :global(:root[data-theme="dark"]) {
    --bg: #111114;
    --surface: #1e1e24;
    --ink: #f1f1f4;
    --on-ink: #111114;
    --muted: #a0a1ab;
    --faint: #6b6c76;
    --line: #2a2a31;
    --ring: rgba(241, 241, 244, 0.6);
    --lavender: #2a2135;
    --sky: #182a33;
    --mint: #172b22;
    --butter: #2e2915;
    --rose: #3b1f22;
    --danger: #fca5a5;
    --accent-text: #c9a3f5;
    --accent-link: #7dd3fc;
    --accent-code: #86efac;
    color-scheme: dark;
  }
  :global(html, body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
    color: var(--ink);
    font-size: 14px;
    user-select: none;
    -webkit-font-smoothing: antialiased;
  }
  :global(*) {
    box-sizing: border-box;
  }
  button {
    font: inherit;
    color: inherit;
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  /* Header */
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 18px 16px 14px;
  }
  .logo {
    width: 40px;
    height: 40px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 13px;
    background: var(--ink);
    color: var(--on-ink);
  }
  .logo :global(svg) {
    pointer-events: none;
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 800;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }
  .titles p {
    margin: 1px 0 0;
    font-size: 12.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .icon-btn {
    width: 36px;
    height: 36px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--surface);
    transition: transform 0.12s;
  }
  .icon-btn:hover {
    transform: scale(1.06);
  }
  .clear-btn:hover {
    color: var(--danger);
  }
  .clear-btn.confirm {
    width: auto;
    display: inline-flex;
    gap: 6px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--danger);
    color: var(--surface);
    font-size: 12.5px;
    font-weight: 700;
  }
  .clear-btn.confirm:hover {
    transform: none;
    color: var(--surface);
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 36px;
    padding: 0 13px;
    border-radius: 999px;
    background: var(--surface);
    font-size: 12.5px;
    font-weight: 700;
  }
  .status .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--green);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--green) 25%, transparent);
  }
  .status.paused {
    background: var(--butter);
  }
  .status.paused .dot {
    background: var(--amber);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--amber) 25%, transparent);
  }

  /* Pencarian */
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 16px;
    padding: 0 8px 0 16px;
    height: 44px;
    border-radius: 999px;
    background: var(--surface);
    color: var(--faint);
    box-shadow: 0 0 0 0 transparent;
    transition: box-shadow 0.15s;
  }
  .search:focus-within {
    box-shadow: 0 0 0 2px var(--ring);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    font-size: 14.5px;
    font-weight: 500;
    color: var(--ink);
  }
  .search input::placeholder {
    color: var(--faint);
  }
  .clear-query {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--bg);
    color: var(--muted);
  }

  /* Segmented control (filter & tema) */
  .segmented {
    display: flex;
    gap: 2px;
    padding: 4px;
    border-radius: 999px;
    background: var(--surface);
  }
  .segmented button {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 30px;
    border-radius: 999px;
    font-size: 12.5px;
    font-weight: 700;
    color: var(--muted);
    transition: background 0.15s, color 0.15s;
  }
  .segmented button:hover {
    color: var(--ink);
  }
  .segmented button.active {
    background: var(--ink);
    color: var(--on-ink);
  }
  .filters {
    margin: 10px 16px 12px;
  }

  /* Daftar */
  .list {
    flex: 1;
    list-style: none;
    margin: 0;
    padding: 0 16px 16px;
    overflow-y: auto;
  }
  .list > :global(li) {
    margin-bottom: 8px;
  }
  .list::-webkit-scrollbar {
    width: 6px;
  }
  .list::-webkit-scrollbar-thumb {
    background: var(--line);
    border-radius: 3px;
  }
  .section {
    padding: 4px 4px 0;
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
  }

  .card {
    --accent: var(--accent-text);
    display: grid;
    grid-template-columns: 38px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 12px 12px 12px 12px;
    border-radius: 20px;
    background: var(--lavender);
    cursor: pointer;
    outline: 2px solid transparent;
    outline-offset: -2px;
    transition: outline-color 0.12s;
  }
  .card.link {
    --accent: var(--accent-link);
    background: var(--sky);
  }
  .card.code {
    --accent: var(--accent-code);
    background: var(--mint);
  }
  .card.color {
    background: var(--butter);
  }
  .card.selected {
    outline-color: var(--ring);
  }

  .icon {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    align-self: start;
    border-radius: 12px;
    background: var(--surface);
    color: var(--accent);
  }
  .icon.swatch {
    border: 4px solid var(--surface);
  }

  .body {
    min-width: 0;
  }
  .text {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 600;
    line-height: 1.4;
    word-break: break-word;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .card.selected .text {
    -webkit-line-clamp: 5;
    line-clamp: 5;
  }
  .card.code .text {
    white-space: pre;
    font-family: "Cascadia Mono", Consolas, monospace;
    font-variant-ligatures: none;
    font-size: 12.5px;
    font-weight: 500;
    line-height: 1.5;
    -webkit-line-clamp: 3;
    line-clamp: 3;
  }
  .card.code.selected .text {
    -webkit-line-clamp: 8;
    line-clamp: 8;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
  }
  .sep {
    color: var(--faint);
  }

  .side {
    display: flex;
    gap: 4px;
    align-self: start;
  }
  .act {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--surface);
    color: var(--muted);
    opacity: 0;
    transition: opacity 0.12s, color 0.12s;
  }
  .card:hover .act,
  .card.selected .act,
  .act.on {
    opacity: 1;
  }
  .act:hover {
    color: var(--ink);
  }
  .act.on {
    color: var(--ink);
  }

  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 24px;
    height: 24px;
    padding: 0 7px;
    border-radius: 8px;
    background: var(--surface);
    font: inherit;
    font-size: 11.5px;
    font-weight: 700;
    color: var(--ink);
  }

  /* Kosong */
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 0 40px 48px;
  }
  .empty-icon {
    width: 68px;
    height: 68px;
    display: grid;
    place-items: center;
    margin-bottom: 14px;
    border-radius: 22px;
    background: var(--surface);
    color: var(--muted);
  }
  .empty h2 {
    margin: 0 0 6px;
    font-size: 17px;
    font-weight: 800;
  }
  .empty p {
    margin: 0;
    color: var(--muted);
    line-height: 1.5;
  }

  /* Pengaturan */
  .settings {
    flex: 1;
    overflow-y: auto;
    padding: 0 16px 16px;
  }
  .settings::-webkit-scrollbar {
    width: 6px;
  }
  .settings::-webkit-scrollbar-thumb {
    background: var(--line);
    border-radius: 3px;
  }
  .group {
    margin-bottom: 18px;
  }
  .group h2 {
    margin: 0 0 8px 4px;
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    background: var(--surface);
  }
  .row:first-of-type {
    border-radius: 18px 18px 0 0;
  }
  .row:last-of-type {
    border-radius: 0 0 18px 18px;
  }
  .row:only-of-type {
    border-radius: 18px;
  }
  .row + .row {
    border-top: 1px solid var(--line);
  }
  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row-text strong {
    font-size: 14px;
    font-weight: 700;
  }
  .row-text span {
    font-size: 12px;
    color: var(--muted);
  }

  .switch {
    position: relative;
    width: 42px;
    height: 24px;
    flex: none;
    border-radius: 999px;
    background: var(--line);
    transition: background 0.18s;
  }
  .switch span {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    transition: transform 0.18s;
  }
  .switch.on {
    background: var(--green);
  }
  .switch.on span {
    transform: translateX(18px);
  }

  .danger {
    flex: none;
    height: 32px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--rose);
    color: var(--danger);
    font-size: 12.5px;
    font-weight: 700;
  }
  .danger.confirm {
    background: var(--danger);
    color: var(--surface);
  }

  .keys {
    margin: 0;
    padding: 6px 14px;
    border-radius: 18px;
    background: var(--surface);
  }
  .keys div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 0;
  }
  .keys div + div {
    border-top: 1px solid var(--line);
  }
  .keys dt {
    font-size: 13px;
    font-weight: 600;
  }
  .keys dd {
    margin: 0;
  }
  .keys kbd {
    background: var(--bg);
  }
</style>
