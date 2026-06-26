<script lang="ts">
  import CompactFilterToolbar from '$lib/components/superadmin/shared/CompactFilterToolbar.svelte';
  import { t } from 'svelte-i18n';

  let {
    searchQuery = $bindable(),
    roleFilter = $bindable(),
    statusFilter = $bindable(),
    viewMode = $bindable(),
    isMobile,
  } = $props<{
    searchQuery: string;
    roleFilter: 'all' | 'superadmin' | 'admin' | 'user';
    statusFilter: 'all' | 'active' | 'inactive';
    viewMode: 'table' | 'cards';
    isMobile: boolean;
  }>();

  let filtersOpen = $state(false);

  const activeFilterCount = $derived.by(() => {
    let count = 0;
    if (roleFilter !== 'all') count += 1;
    if (statusFilter !== 'all') count += 1;
    return count;
  });

  function resetFilters() {
    roleFilter = 'all';
    statusFilter = 'all';
    searchQuery = '';
  }
</script>

<CompactFilterToolbar
  bind:searchQuery
  placeholder={$t('superadmin.users.search')}
  bind:filterPanelOpen={filtersOpen}
  {activeFilterCount}
  onReset={resetFilters}
  {isMobile}
  bind:viewMode
>
  {#snippet advancedFilters()}
    <div class="field-grid">
      <div class="field">
        <label for="user-role-filter">{$t('common.role')}</label>
        <select id="user-role-filter" bind:value={roleFilter}>
          <option value="all">{$t('superadmin.users.filters.all_roles')}</option>
          <option value="admin">{$t('superadmin.users.filters.admin')}</option>
          <option value="user">{$t('superadmin.users.filters.user')}</option>
          <option value="superadmin">{$t('superadmin.users.filters.superadmin')}</option>
        </select>
      </div>

      <div class="field">
        <label for="user-status-filter">{$t('common.status')}</label>
        <select id="user-status-filter" bind:value={statusFilter}>
          <option value="all">{$t('common.all')}</option>
          <option value="active">{$t('superadmin.users.filters.active')}</option>
          <option value="inactive">{$t('superadmin.users.filters.inactive')}</option>
        </select>
      </div>
    </div>
  {/snippet}
</CompactFilterToolbar>

<style>
  .field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 220px));
    gap: 0.75rem;
  }

  .field {
    display: grid;
    gap: 0.32rem;
  }

  .field label {
    font-size: 0.74rem;
    font-weight: 800;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .field select {
    min-height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 0 0.75rem;
    outline: none;
  }

  .field select:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  @media (max-width: 760px) {
    .field-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
