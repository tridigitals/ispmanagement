<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { buildImportCenterSources } from '$lib/components/network/import-center/importCenterTypes';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { t } from 'svelte-i18n';

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const sources = $derived.by(() => buildImportCenterSources(tenantPrefix));
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={$t('admin.network.import_center.title') || 'Network Imports'}
    subtitle={$t('network.import.title') || 'Pusat import data jaringan.'}
  />

  <section class="import-overview" aria-label={$t('network.import.overview') || 'Import center overview'}>
    <div>
      <h2>{$t('admin.network.import_center.panel_title') || 'Import aman untuk lifecycle ISP'}</h2>
      <p>
        {$t('admin.network.import_center.panel_description') ||
          'Import melewati staging, mapping, preview, lalu eksekusi.'}
      </p>
    </div>
    <div class="overview-metrics">
      <div>
        <strong>{sources.length}</strong>
        <span>{$t('admin.network.import_center.sources_count') || 'Source tersedia'}</span>
      </div>
      <div>
        <strong>4</strong>
        <span>{$t('admin.network.import_center.lifecycle_count') || 'Lifecycle dijaga'}</span>
      </div>
    </div>
  </section>

  <section class="source-section" aria-label={$t('network.import.sources') || 'Import sources'}>
    <div class="section-head">
      <div>
        <h2>{$t('admin.network.import_center.sources_title') || 'Pilih source import'}</h2>
        <p>
          {$t('admin.network.import_center.sources_subtitle') || 'Pilih sumber import yang tersedia.'}
        </p>
      </div>
    </div>

    <div class="source-list">
      {#each sources as source}
        <button class="source-row" type="button" onclick={() => goto(source.href)}>
          <span class="source-icon">
            <Icon name={source.icon} size={20} />
          </span>
          <span class="source-copy">
            <span class="source-title">
              <strong>{source.title}</strong>
              <span class={`status ${source.status}`}>
                {source.status === 'ready'
                  ? $t('admin.network.import_center.status.ready') || 'Ready'
                  : $t('admin.network.import_center.status.coming_soon') || 'Coming soon'}
              </span>
            </span>
            <span class="source-description">{source.description}</span>
          </span>
          <span class="source-action">
            {$t('admin.network.import_center.open_wizard') || 'Buka wizard'}
            <Icon name="arrow-right" size={16} />
          </span>
        </button>
      {/each}
    </div>
  </section>
</div>

<style>
  .page-content {
    display: grid;
    gap: 16px;
    padding: 22px;
    max-width: 1460px;
    margin: 0 auto;
  }

  .import-overview,
  .source-section {
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.22));
    background: var(--bg-surface);
    border-radius: var(--radius-lg);
    box-shadow: none;
  }

  .import-overview {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 16px;
    align-items: center;
    padding: 18px;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    color: var(--text-primary);
    font-size: 1.08rem;
    margin-top: 6px;
  }

  p,
  .source-description {
    color: var(--text-secondary);
    line-height: 1.55;
  }

  .import-overview p {
    margin-top: 8px;
    max-width: 760px;
  }

  .overview-metrics {
    display: grid;
    grid-template-columns: repeat(2, minmax(120px, 1fr));
    gap: 12px;
  }

  .overview-metrics div {
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.2));
    border-radius: var(--radius-lg);
    padding: 14px;
    min-width: 120px;
  }

  .overview-metrics strong {
    display: block;
    color: var(--text-primary);
    font-size: 1.35rem;
  }

  .overview-metrics span {
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .source-section {
    display: grid;
    gap: 16px;
    padding: 20px;
  }

  .section-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .section-head p {
    margin-top: 6px;
  }

  .source-list {
    display: grid;
    gap: 12px;
  }

  .source-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 14px;
    align-items: center;
    width: 100%;
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.18));
    background: color-mix(in srgb, var(--bg-surface) 84%, transparent);
    border-radius: var(--radius-lg);
    color: inherit;
    cursor: pointer;
    padding: 16px;
    text-align: left;
    transition:
      border-color 0.16s ease,
      transform 0.16s ease,
      background 0.16s ease;
  }

  .source-row:hover {
    border-color: rgba(14, 165, 233, 0.42);
    background: color-mix(in srgb, var(--bg-surface) 92%, #0ea5e9 8%);
    transform: translateY(-1px);
  }

  .source-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 14px;
    background: rgba(14, 165, 233, 0.14);
    color: #38bdf8;
  }

  .source-copy {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .source-title {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }

  .status {
    border: 1px solid currentColor;
    border-radius: 999px;
    font-size: 0.76rem;
    font-weight: 700;
    padding: 3px 9px;
  }

  .status.ready {
    color: #22c55e;
  }

  .status.coming_soon {
    color: var(--text-secondary);
  }

  .source-action {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: #38bdf8;
    font-size: 0.9rem;
    font-weight: 700;
    white-space: nowrap;
  }

  @media (max-width: 760px) {
    .page-content {
      padding: 20px;
    }

    .import-overview,
    .source-row {
      grid-template-columns: 1fr;
    }

    .overview-metrics {
      grid-template-columns: 1fr;
    }

    .source-action {
      justify-content: flex-start;
    }
  }
</style>
