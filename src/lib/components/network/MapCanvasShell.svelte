<script lang="ts">
  import { createEventDispatcher, onDestroy } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';

  type SearchResult = {
    display_name: string;
    lat: string;
    lon: string;
  };

  const dispatch = createEventDispatcher<{
    searchselect: { lat: number; lng: number; label: string };
  }>();

  export let loading = false;
  export let mapUnavailable = false;
  export let mapErrorMessage = '';
  export let mapUnavailableTitle = 'Map preview unavailable on this device';
  export let mapUnavailableSubtitle = 'WebGL context failed. Data is still loaded and counts are visible.';
  export let viewMode: 'standard' | 'satellite' = 'standard';
  export let showViewSwitch = true;
  export let showSearch = true;
  export let searchPlaceholder = 'Search city or address...';
  export let height = 'min(62vh, 700px)';
  export let mapEl: HTMLDivElement | null = null;

  let searchQuery = '';
  let searching = false;
  let searchError = '';
  let results: SearchResult[] = [];
  let searchOpen = false;
  let searchDockEl: HTMLDivElement | null = null;
  let viewMenuOpen = false;
  let viewMenuEl: HTMLDivElement | null = null;

  async function submitSearch() {
    const q = searchQuery.trim();
    if (!q) return;
    searching = true;
    searchError = '';
    try {
      const url = new URL('https://nominatim.openstreetmap.org/search');
      url.searchParams.set('q', q);
      url.searchParams.set('format', 'jsonv2');
      url.searchParams.set('limit', '6');
      const res = await fetch(url.toString(), {
        headers: {
          Accept: 'application/json',
        },
      });
      if (!res.ok) throw new Error(`Search failed (${res.status})`);
      const data = (await res.json()) as SearchResult[];
      results = Array.isArray(data) ? data : [];
      if (!results.length) searchError = 'Location not found';
    } catch (err: any) {
      searchError = err?.message || 'Failed to search location';
      results = [];
    } finally {
      searching = false;
    }
  }

  function selectResult(item: SearchResult) {
    const lat = Number(item.lat);
    const lng = Number(item.lon);
    if (!Number.isFinite(lat) || !Number.isFinite(lng)) return;
    searchQuery = item.display_name;
    results = [];
    dispatch('searchselect', {
      lat,
      lng,
      label: item.display_name,
    });
  }

  function toggleSearchOpen() {
    searchOpen = !searchOpen;
    if (!searchOpen) {
      results = [];
      searchError = '';
    }
  }

  function toggleViewMenu() {
    viewMenuOpen = !viewMenuOpen;
  }

  function setViewMode(mode: 'standard' | 'satellite') {
    viewMode = mode;
    viewMenuOpen = false;
  }

  function onGlobalPointerDown(event: PointerEvent) {
    const target = event.target as Node | null;
    if (!target) return;
    if (showSearch && searchOpen && !searchDockEl?.contains(target)) {
      searchOpen = false;
      results = [];
      searchError = '';
    }
    if (showViewSwitch && viewMenuOpen && !viewMenuEl?.contains(target)) {
      viewMenuOpen = false;
    }
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('pointerdown', onGlobalPointerDown, { passive: true });
  }

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('pointerdown', onGlobalPointerDown);
    }
  });
</script>

<div class="map-wrap">
  <div class="map-shell" style={`--map-shell-height:${height};`}>
    {#if loading}
      <div class="map-loading">Loading...</div>
    {/if}
    {#if mapUnavailable}
      <div class="map-unavailable">
        <div>
          <div class="map-unavailable-title">{mapUnavailableTitle}</div>
          <div class="map-unavailable-sub">{mapUnavailableSubtitle}</div>
          {#if mapErrorMessage}
            <div class="map-unavailable-sub">{mapErrorMessage}</div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="map-canvas" bind:this={mapEl}></div>
    {/if}

    {#if showViewSwitch}
      <div class="map-view-switch" bind:this={viewMenuEl}>
        <button
          type="button"
          class={`map-view-toggle ${viewMenuOpen ? 'active' : ''}`}
          onclick={toggleViewMenu}
          title="Map view mode"
          aria-label="Map view mode"
        >
          <span class="map-view-toggle-label">View</span>
        </button>
        {#if viewMenuOpen}
          <div class="map-view-menu">
            <button
              type="button"
              class={`map-view-item ${viewMode === 'standard' ? 'active' : ''}`}
              onclick={() => setViewMode('standard')}
            >
              <Icon name="map" size={14} />
              <span>Standard</span>
            </button>
            <button
              type="button"
              class={`map-view-item ${viewMode === 'satellite' ? 'active' : ''}`}
              onclick={() => setViewMode('satellite')}
            >
              <Icon name="satellite" size={14} />
              <span>Satellite</span>
            </button>
          </div>
        {/if}
      </div>
    {/if}

    {#if showSearch}
      <div class="map-search-dock" bind:this={searchDockEl}>
        <button class={`map-search-toggle ${searchOpen ? 'active' : ''}`} type="button" onclick={toggleSearchOpen}>
          <Icon name="search" size={16} />
        </button>

        {#if searchOpen}
          <div class="map-search">
            <form
              class="map-search-form"
              onsubmit={(e) => {
                e.preventDefault();
                void submitSearch();
              }}
            >
              <input
                class="map-search-input"
                type="text"
                bind:value={searchQuery}
                placeholder={searchPlaceholder}
                autocomplete="off"
              />
              <button class="map-search-btn" type="submit" disabled={searching}>
                {searching ? '...' : 'Search'}
              </button>
            </form>
            {#if searchError}
              <div class="map-search-error">{searchError}</div>
            {/if}
            {#if results.length}
              <div class="map-search-results">
                {#each results as item}
                  <button class="map-search-item" type="button" onclick={() => selectResult(item)}>
                    {item.display_name}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <slot name="overlay"></slot>
  </div>
</div>

<style>
  .map-wrap {
    margin-top: 14px;
  }

  .map-shell {
    position: relative;
    height: var(--map-shell-height);
    min-height: 360px;
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
    background: #070b15;
  }

  .map-canvas {
    width: 100%;
    height: 100%;
  }

  .map-loading {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(7, 11, 21, 0.45);
    color: var(--text-secondary);
    z-index: 8;
    font-weight: 700;
  }

  .map-unavailable {
    position: absolute;
    inset: 0;
    padding: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    background: rgba(7, 11, 21, 0.82);
    color: var(--text-secondary);
    z-index: 9;
  }

  .map-unavailable-title {
    color: var(--text-primary);
    font-weight: 800;
    margin-bottom: 6px;
  }

  .map-unavailable-sub {
    font-size: 0.92rem;
  }

  .map-view-switch {
    position: absolute;
    right: 12px;
    top: 62px;
    z-index: 12;
    display: inline-block;
  }

  .map-view-toggle {
    width: 38px;
    height: 38px;
    border: 1px solid rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    background: #ffffff;
    color: #1f2937;
    display: grid;
    place-items: center;
    cursor: pointer;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }

  .map-view-toggle-label {
    font-size: 0.62rem;
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .map-view-toggle.active {
    border-color: rgba(59, 130, 246, 0.55);
    color: #1d4ed8;
  }

  .map-view-menu {
    margin-top: 6px;
    min-width: 156px;
    border: 1px solid rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    background: #ffffff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.18);
    overflow: hidden;
  }

  .map-view-item {
    width: 100%;
    border: none;
    background: transparent;
    color: #111827;
    padding: 9px 10px;
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
    cursor: pointer;
    font-size: 0.88rem;
  }

  .map-view-item:hover {
    background: #f3f4f6;
  }

  .map-view-item.active {
    background: #eff6ff;
    color: #1d4ed8;
  }

  .map-search-dock {
    position: absolute;
    left: 12px;
    top: 12px;
    z-index: 12;
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .map-search-toggle {
    width: 44px;
    height: 44px;
    border-radius: 4px;
    border: 1px solid rgba(0, 0, 0, 0.2);
    background: #ffffff;
    color: #1f2937;
    display: grid;
    place-items: center;
    cursor: pointer;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }

  .map-search-toggle.active {
    border-color: rgba(59, 130, 246, 0.55);
    color: #1d4ed8;
  }

  .map-search {
    width: min(460px, calc(100vw - 200px));
    background: #ffffff;
    border: 1px solid rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    padding: 8px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.18);
  }

  .map-search-form {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }

  .map-search-input {
    height: 38px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: #ffffff;
    color: #111827;
    padding: 0 12px;
    outline: none;
  }

  .map-search-input:focus {
    border-color: rgba(99, 102, 241, 0.55);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.16);
  }

  .map-search-btn {
    height: 38px;
    padding: 0 12px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: #ffffff;
    color: #111827;
    font-weight: 700;
    cursor: pointer;
  }

  .map-search-results {
    margin-top: 6px;
    max-height: 220px;
    overflow: auto;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: #ffffff;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.22);
  }

  .map-search-item {
    display: block;
    width: 100%;
    border: none;
    background: transparent;
    color: #111827;
    text-align: left;
    padding: 10px 12px;
    cursor: pointer;
    font-size: 0.9rem;
    line-height: 1.35;
  }

  .map-search-item:hover {
    background: #f3f4f6;
  }

  .map-search-error {
    margin-top: 6px;
    font-size: 0.83rem;
    color: #fca5a5;
  }

  @media (max-width: 760px) {
    .map-search-dock {
      left: 12px;
      top: 12px;
    }

    .map-search {
      width: calc(100vw - 92px);
      max-width: none;
    }
  }

</style>
