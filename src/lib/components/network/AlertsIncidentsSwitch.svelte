<script lang="ts">
  import { t } from 'svelte-i18n';

  let {
    current = 'alerts',
    nocHref = '',
    alertsHref = '/admin/network/alerts',
    incidentsHref = '/admin/network/incidents',
  }: {
    current?: 'noc' | 'alerts' | 'incidents';
    nocHref?: string;
    alertsHref?: string;
    incidentsHref?: string;
  } = $props();
</script>

<div class="view-switch" role="tablist" aria-label={$t('sidebar.sections.network') || 'Network'}>
  {#if nocHref}
    <a class={`switch-btn ${current === 'noc' ? 'active' : ''}`} href={nocHref}>
      {$t('sidebar.noc') || 'NOC'}
    </a>
  {/if}
  <a class={`switch-btn ${current === 'alerts' ? 'active' : ''}`} href={alertsHref}>
    {$t('sidebar.alerts') || 'Alerts'}
  </a>
  <a class={`switch-btn ${current === 'incidents' ? 'active' : ''}`} href={incidentsHref}>
    {$t('sidebar.incidents') || 'Incidents'}
  </a>
</div>

<style>
  .view-switch {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
    padding: 3px;
    gap: 2px;
  }
  .switch-btn {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    font-weight: 800;
    border-radius: 9px;
    padding: 8px 12px;
    cursor: pointer;
    text-decoration: none;
  }
  .switch-btn.active {
    background: color-mix(in srgb, var(--accent) 16%, var(--bg-surface));
    color: var(--text-primary);
  }
  .switch-btn:hover:not(.active) {
    color: var(--text-primary);
    background: var(--bg-hover);
  }
</style>
