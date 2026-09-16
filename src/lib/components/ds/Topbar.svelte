<!--
  Ds/Topbar — topbar shell v2.

  Berbeda dari Topbar lama (590 baris CSS scoped), di sini hanya utility dan
  tinggi dikunci 56px supaya sejajar dengan header NavRail.

  Global search diporta penuh dari Topbar lama: debounce, Ctrl/⌘+K, panah
  naik-turun, Enter navigasi, Escape tutup, panel hasil memakai komponen
  TopbarGlobalSearchPanel yang sama. Bedanya konteks scope dihitung dari
  path v2 (/v2/admin/... == admin), bukan path mentah.
-->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onDestroy, onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import type { Snippet } from 'svelte';
  import { can, isSuperAdmin, user } from '$lib/stores/auth';
  import { searchGlobalTopbar } from '$lib/search/globalSearchService';
  import type {
    GlobalSearchProviderContext,
    GlobalSearchResult,
  } from '$lib/search/globalSearchModel';
  import { globalSearch } from '$lib/stores/globalSearch';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import Icon from './Icon.svelte';
  import TopbarGlobalSearchPanel from '../layout/TopbarGlobalSearchPanel.svelte';

  interface Props {
    /** Judul konteks, biasanya nama halaman aktif. */
    title?: string;
    /** Isi kanan: notifikasi, menu user, dsb. */
    right?: Snippet;
    onMenuClick?: () => void;
    /** false untuk portal pelanggan — tidak punya global search. */
    search?: boolean;
    searchPlaceholder?: string;
  }

  let {
    title,
    right,
    onMenuClick,
    search = true,
    searchPlaceholder,
  }: Props = $props();

  const SEARCH_DEBOUNCE_MS = 220;

  let inputEl = $state<HTMLInputElement | null>(null);
  let highlightedIndex = $state(-1);
  let closePanelTimer: ReturnType<typeof setTimeout> | null = null;
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  let latestSearchRequestId = 0;

  const ph = $derived(
    searchPlaceholder ?? $t('components.topbar.search_placeholder'),
  );

  /* Provider lama mengenali path '/admin/...'; shell v2 memakai '/v2/admin/...'.
     Strip prefiks /v2 supaya scope (admin/superadmin/workspace) terhitung sama. */
  const searchContext = $derived.by<GlobalSearchProviderContext>(() => {
    const pathname = $page.url.pathname || '/';
    const logicalPath = pathname.replace(/^\/v2(?=\/|$)/, '') || '/';
    const tenantCtx = resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      routeTenantSlug: $page.params.tenant,
    });
    return {
      can: $can,
      isSuperAdmin: $isSuperAdmin,
      shellScope: logicalPath.startsWith('/superadmin')
        ? 'superadmin'
        : logicalPath.startsWith('/admin')
          ? 'admin'
          : 'workspace',
      tenantPrefix: tenantCtx.tenantPrefix,
    };
  });

  const flatItems = $derived.by(() =>
    $globalSearch.groups.flatMap((group) =>
      group.items.map((item) => ({
        groupKey: group.key,
        item,
      })),
    ),
  );
  const showSearchPanel = $derived($globalSearch.open && $globalSearch.query.trim().length > 0);

  async function runSearch(query: string) {
    const requestId = ++latestSearchRequestId;
    const trimmedQuery = query.trim();
    globalSearch.setQuery(query);

    if (!trimmedQuery) {
      globalSearch.setLoading(false);
      globalSearch.setResults([]);
      highlightedIndex = -1;
      return;
    }

    globalSearch.setLoading(true);
    const result = await searchGlobalTopbar(trimmedQuery, searchContext);
    if (requestId !== latestSearchRequestId) return;
    globalSearch.setResults(result.groups);
    globalSearch.setLoading(false);
    highlightedIndex = result.groups.length ? 0 : -1;
  }

  function scheduleSearch(query: string) {
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => {
      void runSearch(query);
    }, SEARCH_DEBOUNCE_MS);
  }

  function handleInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value;
    globalSearch.setQuery(value);
    if (!value.trim()) {
      latestSearchRequestId += 1;
      globalSearch.close();
      highlightedIndex = -1;
      if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
      return;
    }
    globalSearch.open();
    scheduleSearch(value);
  }

  function handleFocus() {
    if (closePanelTimer) clearTimeout(closePanelTimer);
  }

  function handleBlur() {
    closePanelTimer = setTimeout(() => {
      latestSearchRequestId += 1;
      globalSearch.close();
      highlightedIndex = -1;
    }, 120);
  }

  async function handleResultSelect(item: GlobalSearchResult) {
    globalSearch.close();
    latestSearchRequestId += 1;
    highlightedIndex = -1;
    inputEl?.blur();
    /* href dari provider memakai path legacy; layout (app) sudah memetakannya
       ke /v2 lewat v2RedirectFor, jadi tidak perlu konversi di sini. */
    await goto(item.href);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' && flatItems.length) {
      event.preventDefault();
      highlightedIndex = (highlightedIndex + 1 + flatItems.length) % flatItems.length;
      return;
    }
    if (event.key === 'ArrowUp' && flatItems.length) {
      event.preventDefault();
      highlightedIndex = (highlightedIndex - 1 + flatItems.length) % flatItems.length;
      return;
    }
    if (event.key === 'Enter' && highlightedIndex >= 0) {
      event.preventDefault();
      const item = flatItems[highlightedIndex]?.item;
      if (item) void handleResultSelect(item);
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      latestSearchRequestId += 1;
      globalSearch.close();
      highlightedIndex = -1;
      inputEl?.blur();
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      inputEl?.focus();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleWindowKeydown);
    return () => window.removeEventListener('keydown', handleWindowKeydown);
  });

  onDestroy(() => {
    if (closePanelTimer) clearTimeout(closePanelTimer);
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  });
</script>

<header
  class="flex h-14 shrink-0 items-center gap-3 border-b border-ink-200 bg-white px-4"
>
  {#if onMenuClick}
    <button
      onclick={onMenuClick}
      aria-label="Buka menu"
      class="focus-ring grid size-8 place-items-center rounded-lg text-ink-500 hover:bg-ink-100 lg:hidden"
    >
      <Icon name="menu" size={18} />
    </button>
  {/if}

  {#if title}
    <div class="truncate text-md font-semibold text-ink-900">{title}</div>
  {/if}

  {#if search}
    <div class="relative ml-auto hidden w-full max-w-sm md:block">
      <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400">
        <Icon name="search" size={15} />
      </span>
      <input
        value={$globalSearch.query}
        bind:this={inputEl}
        oninput={handleInput}
        onfocus={handleFocus}
        onblur={handleBlur}
        onkeydown={handleKeydown}
        type="search"
        role="combobox"
        aria-expanded={showSearchPanel}
        aria-controls="global-search-panel"
        aria-autocomplete="list"
        placeholder={ph}
        aria-label={ph}
        class="h-8 w-full rounded-lg bg-ink-50 pr-14 pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400 focus:bg-white focus:ring-brand-600 focus:outline-none"
      />
      <kbd
        class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 rounded border border-ink-200 bg-white px-1 font-mono text-2xs text-ink-400"
      >
        Ctrl K
      </kbd>

      {#if showSearchPanel}
        <div id="global-search-panel" class="absolute inset-x-0 top-full z-60">
          <TopbarGlobalSearchPanel
            groups={$globalSearch.groups}
            loading={$globalSearch.loading}
            query={$globalSearch.query}
            {highlightedIndex}
            onSelect={handleResultSelect}
          />
        </div>
      {/if}
    </div>
  {/if}

  <div class="ml-auto flex items-center gap-1">
    {#if right}{@render right()}{/if}
  </div>
</header>

<style>
  /* Panel warisan memposisikan dirinya absolute; di shell v2 kita jepit ke
     kolom input (kanan) supaya tidak keluar viewport di layar medium. */
  :global(#global-search-panel .search-results-panel) {
    left: auto;
    right: 0;
    transform: none;
    width: min(640px, calc(100vw - 32px));
    box-shadow: 0 14px 34px rgba(9, 12, 20, 0.14);
    border-color: var(--color-ink-200);
    background: var(--color-ink-0);
  }
</style>
