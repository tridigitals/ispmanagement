<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  let { page = $bindable(0), pageCount = 1 }: { page?: number; pageCount?: number } = $props();
</script>

{#if pageCount > 1}
  <div class="top-pager" aria-label={$t('admin.network.wallboard.pager.aria') || 'Pages'}>
    <button
      class="top-pager-btn"
      type="button"
      onclick={() => (page = Math.max(0, page - 1))}
      disabled={page === 0}
      aria-label={$t('admin.network.wallboard.pager.prev') || 'Previous page'}
    >
      <Icon name="chevron-left" size={15} />
    </button>
    <span class="top-pager-label">
      {($t('common.page') || 'Page') + ' ' + (page + 1) + '/' + pageCount}
    </span>
    <button
      class="top-pager-btn"
      type="button"
      onclick={() => (page = Math.min(pageCount - 1, page + 1))}
      disabled={page >= pageCount - 1}
      aria-label={$t('admin.network.wallboard.pager.next') || 'Next page'}
    >
      <Icon name="chevron-right" size={15} />
    </button>
  </div>
{/if}

<style>
  .top-pager {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }
  .top-pager-label {
    font-weight: 850;
    font-size: 12px;
    color: var(--text-muted);
    min-width: 76px;
    text-align: center;
    white-space: nowrap;
  }
  .top-pager-btn {
    width: 30px;
    height: 30px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
    color: var(--text-primary);
    display: grid;
    place-items: center;
    cursor: pointer;
    padding: 0;
  }
  .top-pager-btn:disabled {
    opacity: 0.55;
    cursor: default;
  }

  @media (max-width: 920px) {
    .top-pager {
      padding: 5px 7px;
    }
    .top-pager-label {
      min-width: 68px;
      font-size: 11px;
    }
  }
</style>
