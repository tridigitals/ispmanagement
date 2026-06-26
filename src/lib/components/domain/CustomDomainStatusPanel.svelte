<script lang="ts">
  import { resolveCustomDomainStatusView } from '$lib/utils/customDomainStatus';
  import CustomDomainStatusBadge from './CustomDomainStatusBadge.svelte';
  import { t } from 'svelte-i18n';

  let {
    customDomain = null,
    status = null,
    failureReason = null,
    verifiedAt = null,
  } = $props<{
    customDomain?: string | null;
    status?: string | null;
    failureReason?: string | null;
    verifiedAt?: string | null;
  }>();

  const view = $derived(
    resolveCustomDomainStatusView({
      customDomain,
      status,
      failureReason,
    }),
  );
</script>

<section class="domain-status-panel">
  <div class="domain-status-header">
    <div>
      <div class="domain-status-title">{$t('admin.settings.branding.domain_status')}</div>
      <div class="domain-status-domain">{customDomain || $t('admin.settings.branding.not_configured') || 'Belum dikonfigurasi'}</div>
    </div>
    <CustomDomainStatusBadge {customDomain} {status} {failureReason} />
  </div>

  <p class="domain-status-description">{view.description}</p>

  {#if verifiedAt && view.key === 'active'}
    <div class="domain-status-meta">Terverifikasi: {new Date(verifiedAt).toLocaleString('id-ID')}</div>
  {/if}
</section>

<style>
  .domain-status-panel {
    margin-top: 0.75rem;
    padding: 0.9rem 1rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
  }

  .domain-status-header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .domain-status-title {
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .domain-status-domain {
    margin-top: 0.15rem;
    color: var(--text-primary);
    font-size: 0.96rem;
    font-weight: 700;
    line-height: 1.35;
    word-break: break-word;
  }

  .domain-status-description {
    margin: 0.65rem 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.5;
  }

  .domain-status-meta {
    margin-top: 0.5rem;
    color: var(--text-muted);
    font-size: 0.8rem;
  }
</style>
