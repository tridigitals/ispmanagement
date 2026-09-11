<!--
  Ds/NavRail — sidebar ikon yang melebar saat hover; grup accordion tertutup.

  Menggantikan Sidebar.svelte (1.139 baris) untuk shell v2. Perbedaan pokok:
  - Lebar 56px saat diam, 240px saat hover/fokus. Isi utama dapat ruang lebih.
  - Ikon sinkron (bukan dynamic import), jadi tidak ada pop-in.
  - Default SEMUA seksi tertutup; klik judul untuk buka. Bersifat accordion
    single-open: membuka satu seksi menutup seksi lain. Seksi yang memuat
    menu aktif otomatis terbuka (termasuk saat pindah rute), dan hanya bisa
    ditutup manual kalau menu aktif ikut pindah / user menutupnya sendiri.
    Rail dengan satu grup (portal) selalu terbuka — tidak masuk akal folded.
    State lipatan dipersist ke sessionStorage per tab.
  - Hanya SATU item yang boleh bertanda aktif: prefix terpanjang menang
    (activeRailHref) — dulu 'Beranda' (/v2/admin) ikut nyala di semua
    sub-route karena hanya '/admin' yang dikecualikan dari prefix-match.
-->
<script lang="ts">
  import { browser } from '$app/environment';
  import Icon from './Icon.svelte';
  import { activeRailHref, type RailGroup } from './nav-types';

  interface Props {
    groups: RailGroup[];
    /** Path aktif saat ini untuk menandai item terpilih. */
    current: string;
    brand?: string;
    tenant?: string;
  }

  let { groups, current, brand = 'ISP Management', tenant }: Props = $props();

  let expanded = $state(false);

  const OPEN_KEY = 'navra…pens';

  /** Seksi yang DIBUKA eksplisit oleh user. Absen dari map = ikut default
   *  (tertutup, kecuali grup aktif / rail satu grup). Record biasa, bukan
   *  Set — mutasi Set in-place tidak memicu reaktivitas di Svelte 5. */
  let openGroups = $state<Record<string, boolean>>({});

  function readOpen(): Record<string, boolean> {
    if (!browser) return {};
    try {
      const raw = sessionStorage.getItem(OPEN_KEY);
      return raw ? JSON.parse(raw) : {};
    } catch {
      return {};
    }
  }

  function writeOpen() {
    if (!browser) return;
    try {
      sessionStorage.setItem(OPEN_KEY, JSON.stringify(openGroups));
    } catch {
      /* storage penuh/incognito: lipatan tetap hidup di memori sesi ini */
    }
  }

  // Baca ulang saat mount (SSR-safe): nilai klien baru tersedia di browser.
  $effect(() => {
    openGroups = readOpen();
  });

  const allHrefs = $derived(groups.flatMap((g) => g.items.map((i) => i.href)));
  const activeHref = $derived(activeRailHref(allHrefs, current));
  const singleGroup = $derived(groups.length <= 1);

  function groupHasActive(g: RailGroup) {
    return g.items.some((i) => i.href === activeHref);
  }

  const activeGroupTitle = $derived(groups.find((g) => groupHasActive(g))?.title);

  function isOpen(g: RailGroup): boolean {
    if (singleGroup) return true;
    if (g.title in openGroups) return openGroups[g.title];
    // Default: tertutup, kecuali grup yang memuat menu aktif.
    return groupHasActive(g);
  }

  function toggleGroup(g: RailGroup) {
    const opening = !isOpen(g);
    // Accordion: menutup semua seksi lain saat satu dibuka.
    for (const other of groups) openGroups[other.title] = other.title === g.title ? opening : false;
    writeOpen();
  }

  // Pindah rute: seksi milik menu aktif ikut terbuka (accordion menutup yang
  // lain). Saat MOUNT tidak dipaksa — kalau tidak, menutup seksi aktif lalu
  // reload akan langsung 'melawan' pilihan user. lastActive===null = belum
  // terisi (mount pertama).
  let lastActive: string | null = null;
  $effect(() => {
    const href = activeHref;
    if (lastActive === null) {
      lastActive = href;
      return;
    }
    if (href === lastActive) return;
    lastActive = href;
    const t = activeGroupTitle;
    if (t === undefined || singleGroup) return;
    for (const other of groups) openGroups[other.title] = other.title === t;
    writeOpen();
  });
</script>

<nav
  onmouseenter={() => (expanded = true)}
  onmouseleave={() => (expanded = false)}
  onfocusin={() => (expanded = true)}
  onfocusout={() => (expanded = false)}
  aria-label="Navigasi utama"
  class="group/rail flex h-full shrink-0 flex-col border-r border-ink-200 bg-white transition-[width] duration-150 ease-out
    {expanded ? 'w-60' : 'w-14'}"
>
  <!-- Identitas tenant -->
  <div class="flex h-14 items-center gap-2.5 border-b border-ink-100 px-3.5">
    <div
      class="grid size-7 shrink-0 place-items-center rounded-md bg-ink-900 text-2xs font-bold text-white"
    >
      ISP
    </div>
    {#if expanded}
      <div class="min-w-0">
        <div class="truncate text-sm font-semibold text-ink-900">{brand}</div>
        {#if tenant}
          <div class="truncate font-mono text-2xs text-ink-400">{tenant}</div>
        {/if}
      </div>
    {/if}
  </div>

  <div class="flex-1 overflow-x-hidden overflow-y-auto py-2">
    {#each groups as g}
      <div class="mb-1">
        {#if expanded}
          <button
            type="button"
            class="focus-ring group/sect flex h-7 w-full items-center gap-1 px-3.5 text-2xs font-semibold tracking-[0.12em] text-ink-400 uppercase transition-colors hover:text-ink-600"
            aria-expanded={isOpen(g)}
            title={isOpen(g) ? 'Lipat bagian ini' : 'Buka bagian ini'}
            onclick={() => toggleGroup(g)}
          >
            <span class="truncate">{g.title}</span>
            <span
              class="ml-auto grid size-4 place-items-center transition-[transform,opacity] duration-150 {isOpen(g)
                ? 'opacity-70'
                : '-rotate-90 opacity-0 group-hover/sect:opacity-70'}"
            >
              <Icon name="chevronDown" size={12} />
            </span>
          </button>
        {:else}
          <div class="mx-3.5 my-2 border-t border-ink-100"></div>
        {/if}

        {#if !expanded || isOpen(g)}
          {#each g.items as it}
            {@const active = it.href === activeHref}
            <a
              href={it.href}
              aria-current={active ? 'page' : undefined}
              title={expanded ? undefined : it.label}
              class="focus-ring relative mx-1.5 flex h-9 items-center gap-3 rounded-lg px-2 text-base
                {active
                ? 'bg-ink-100 font-medium text-ink-900'
                : 'text-ink-500 hover:bg-ink-50 hover:text-ink-900'}"
            >
              {#if active}
                <span class="absolute top-1.5 -left-1.5 h-6 w-[3px] rounded-r bg-ink-900"></span>
              {/if}
              <span class="grid size-5 shrink-0 place-items-center">
                <Icon name={it.icon} size={16} />
              </span>
              {#if expanded}
                <span class="truncate">{it.label}</span>
                {#if it.badge}
                  <span class="num ml-auto rounded bg-red-100 px-1.5 py-px text-2xs font-semibold text-red-700">
                    {it.badge}
                  </span>
                {/if}
              {:else if it.badge}
                <span class="absolute top-1.5 right-1.5 size-1.5 rounded-full bg-red-500"></span>
              {/if}
            </a>
          {/each}
        {/if}
      </div>
    {/each}
  </div>
</nav>
