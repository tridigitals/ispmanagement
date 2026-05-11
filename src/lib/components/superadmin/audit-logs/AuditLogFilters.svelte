<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';

  let {
    searchQuery = $bindable(''),
    actionFilter = $bindable(''),
    dateFrom = $bindable(''),
    dateTo = $bindable(''),
    isMobile = false,
    viewMode = $bindable('table'),
    onSearch,
    onClear,
  } = $props<{
    searchQuery: string;
    actionFilter: string;
    dateFrom: string;
    dateTo: string;
    isMobile: boolean;
    viewMode: 'table' | 'cards';
    onSearch: () => void;
    onClear: () => void;
  }>();

  let filtersOpen = $state(false);

  const activeFilterCount = $derived.by(() => {
    let count = 0;
    if (actionFilter.trim()) count += 1;
    if (dateFrom) count += 1;
    if (dateTo) count += 1;
    return count;
  });

  function clearSearch() {
    searchQuery = '';
    onSearch();
  }

  function setQuickRange(days: number) {
    const now = new Date();
    const from = new Date(now.getTime() - days * 24 * 60 * 60 * 1000);

    const toLocal = (d: Date) => {
      const pad = (n: number) => String(n).padStart(2, '0');
      return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
    };

    dateFrom = toLocal(from);
    dateTo = toLocal(now);
    onSearch();
  }
</script>

<div class="toolbar-shell">
  <div class="toolbar-main">
    <div class="search-input-wrapper">
      <span class="search-icon">
        <Icon name="search" size={18} />
      </span>
      <input
        type="text"
        bind:value={searchQuery}
        oninput={onSearch}
        placeholder={$t('superadmin.audit_logs.search') || 'Search logs...'}
      />
      {#if searchQuery}
        <button class="clear-btn" type="button" onclick={clearSearch}>
          <Icon name="x" size={14} />
        </button>
      {/if}
    </div>

    <div class="quick-row" aria-label={$t('superadmin.audit_logs.aria.quick_ranges') || 'Quick ranges'}>
      <button type="button" class="chip" onclick={() => setQuickRange(1)}>24h</button>
      <button type="button" class="chip" onclick={() => setQuickRange(7)}>7d</button>
      <button type="button" class="chip" onclick={() => setQuickRange(30)}>30d</button>
    </div>

    <button
      class="filter-toggle"
      type="button"
      class:open={filtersOpen}
      onclick={() => (filtersOpen = !filtersOpen)}
      aria-expanded={filtersOpen}
    >
      <Icon name="settings" size={16} />
      <span>Filter{activeFilterCount > 0 ? ` (${activeFilterCount})` : ''}</span>
      <Icon name={filtersOpen ? 'chevron-up' : 'chevron-down'} size={16} />
    </button>

    {#if !isMobile}
      <div class="view-toggle" aria-label={$t('superadmin.audit_logs.aria.view_mode') || 'View mode'}>
        <button
          type="button"
          class="view-btn"
          class:active={viewMode === 'table'}
          onclick={() => (viewMode = 'table')}
          title={$t('superadmin.audit_logs.view.table') || 'Table view'}
          aria-label={$t('superadmin.audit_logs.view.table') || 'Table view'}
        >
          <Icon name="list" size={18} />
        </button>
        <button
          type="button"
          class="view-btn"
          class:active={viewMode === 'cards'}
          onclick={() => (viewMode = 'cards')}
          title={$t('superadmin.audit_logs.view.cards') || 'Card view'}
          aria-label={$t('superadmin.audit_logs.view.cards') || 'Card view'}
        >
          <Icon name="grid" size={18} />
        </button>
      </div>
    {/if}
  </div>

  {#if filtersOpen}
    <div class="filter-panel">
      <div class="field-grid">
        <div class="field">
          <label class="field-label" for="filter-action"
            >{$t('superadmin.audit_logs.filters.action') || 'Action (exact)'}</label
          >
          <input
            id="filter-action"
            type="text"
            bind:value={actionFilter}
            oninput={onSearch}
            placeholder={$t('superadmin.audit_logs.filters.action_placeholder') ||
              'e.g. login, create_user'}
            class="field-input"
          />
        </div>

        <div class="field">
          <label class="field-label" for="filter-date-from"
            >{$t('superadmin.audit_logs.filters.from') || 'From'}</label
          >
          <input
            id="filter-date-from"
            type="datetime-local"
            bind:value={dateFrom}
            onchange={onSearch}
            class="field-input"
          />
        </div>

        <div class="field">
          <label class="field-label" for="filter-date-to"
            >{$t('superadmin.audit_logs.filters.to') || 'To'}</label
          >
          <input
            id="filter-date-to"
            type="datetime-local"
            bind:value={dateTo}
            onchange={onSearch}
            class="field-input"
          />
        </div>
      </div>

      <div class="filter-footer">
        <button type="button" class="reset-link" onclick={onClear}>
          {$t('common.clear') || 'Clear'}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .toolbar-shell {
    padding: 0.95rem 1.1rem 0.2rem;
    display: grid;
    gap: 0.85rem;
  }

  .toolbar-main {
    display: grid;
    grid-template-columns: minmax(220px, 1.4fr) auto auto auto;
    gap: 0.75rem;
    align-items: center;
  }

  .search-input-wrapper,
  .filter-toggle,
  .field-input {
    min-height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .search-input-wrapper {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.48rem 0.68rem;
  }

  .search-icon {
    color: var(--text-secondary);
    display: flex;
    align-items: center;
  }

  .search-input-wrapper input {
    width: 100%;
    border: none;
    background: transparent;
    color: inherit;
    font-size: 0.9rem;
    outline: none;
    padding: 0;
  }

  .search-input-wrapper:focus-within,
  .field-input:focus,
  .filter-toggle.open {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .clear-btn {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex: 0 0 auto;
  }

  .clear-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .quick-row {
    display: inline-flex;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .chip {
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    min-height: 36px;
    padding: 0.4rem 0.66rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 700;
    font-size: 0.78rem;
    transition: all 0.2s;
    white-space: nowrap;
  }

  .chip:hover {
    background: var(--color-primary-subtle);
    color: var(--text-primary);
    border-color: color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
  }

  .filter-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.85rem;
    cursor: pointer;
    font-weight: 600;
    white-space: nowrap;
  }

  .view-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.24rem;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .view-btn {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    padding: 0;
  }

  .view-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .view-btn.active {
    background: var(--color-primary-subtle);
    border: 1px solid color-mix(in srgb, var(--color-primary) 25%, var(--border-color));
    color: var(--text-primary);
  }

  .filter-panel {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    background: var(--bg-surface);
    padding: 0.9rem 1rem;
    display: grid;
    gap: 0.85rem;
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.32rem;
  }

  .field-label {
    font-size: 0.74rem;
    font-weight: 800;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .field-input {
    width: 100%;
    padding: 0.55rem 0.72rem;
    font-size: 0.88rem;
    outline: none;
    transition: border-color 0.2s;
  }

  .filter-footer {
    display: flex;
    justify-content: flex-end;
  }

  .reset-link {
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.86rem;
    font-weight: 600;
  }

  .reset-link:hover {
    color: var(--text-primary);
  }

  @media (max-width: 980px) {
    .toolbar-main {
      grid-template-columns: minmax(220px, 1fr) auto auto;
    }

    .view-toggle {
      grid-column: 1 / -1;
      justify-self: start;
    }
  }

  @media (max-width: 760px) {
    .toolbar-main,
    .field-grid {
      grid-template-columns: 1fr;
    }

    .quick-row {
      justify-content: flex-start;
    }
  }
</style>
