<script lang="ts">
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
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

<div class="toolbar-wrapper">
  <TableToolbar
    bind:searchQuery
    placeholder={$t('superadmin.audit_logs.search') || 'Search logs...'}
    onsearch={onSearch}
  >
    {#snippet filters()}
      <div class="filters-row">
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

        <div
          class="quick-row"
          aria-label={$t('superadmin.audit_logs.aria.quick_ranges') || 'Quick ranges'}
        >
          <button type="button" class="chip" onclick={() => setQuickRange(1)}>24h</button>
          <button type="button" class="chip" onclick={() => setQuickRange(7)}>7d</button>
          <button type="button" class="chip" onclick={() => setQuickRange(30)}>30d</button>
          <button type="button" class="chip danger" onclick={onClear}>
            {$t('common.clear') || 'Clear'}
          </button>
        </div>
      </div>
    {/snippet}

    {#snippet actions()}
      {#if !isMobile}
        <div
          class="view-toggle"
          aria-label={$t('superadmin.audit_logs.aria.view_mode') || 'View mode'}
        >
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
    {/snippet}
  </TableToolbar>
</div>

<style>
  .toolbar-wrapper {
    padding: 0.9rem 1.1rem 0.2rem 1.1rem;
  }

  .filters-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.65rem;
    align-items: flex-end;
    width: 100%;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    flex: 1;
    min-width: 180px;
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
    min-height: 38px;
    padding: 0.55rem 0.72rem;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.88rem;
    transition: border-color 0.2s;
  }

  .field-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .quick-row {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
    min-width: 0;
    padding-bottom: 2px;
  }

  .chip {
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    min-height: 34px;
    padding: 0.4rem 0.62rem;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 700;
    font-size: 0.78rem;
    transition: all 0.2s;
    white-space: nowrap;
  }

  :global([data-theme='light']) .chip {
    border-color: var(--border-color);
    background: var(--bg-tertiary);
  }

  .chip:hover {
    background: var(--color-primary-subtle);
    color: var(--text-primary);
    border-color: color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
  }

  .chip.danger:hover {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-danger) 28%, var(--border-color));
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

  :global([data-theme='light']) .view-toggle {
    border-color: var(--border-color);
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

  :global([data-theme='light']) .view-btn:hover {
    background: var(--bg-hover);
  }

  .view-btn.active {
    background: var(--color-primary-subtle);
    border: 1px solid color-mix(in srgb, var(--color-primary) 25%, var(--border-color));
    color: var(--text-primary);
  }

  @media (max-width: 768px) {
    .toolbar-wrapper {
      padding: 0.85rem 0.9rem 0.15rem;
    }
  }
</style>
