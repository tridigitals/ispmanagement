<script lang="ts">
  import { resolveCustomDomainStatusView } from '$lib/utils/customDomainStatus';

  let {
    customDomain = null,
    status = null,
    failureReason = null,
  } = $props<{
    customDomain?: string | null;
    status?: string | null;
    failureReason?: string | null;
  }>();

  const view = $derived(
    resolveCustomDomainStatusView({
      customDomain,
      status,
      failureReason,
    }),
  );
</script>

<span class="domain-status-badge {view.tone}" title={view.description}>{view.label}</span>

<style>
  .domain-status-badge {
    display: inline-flex;
    align-items: center;
    min-height: 1.75rem;
    padding: 0.1rem 0.55rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }

  .domain-status-badge.success {
    color: var(--color-success);
    border-color: color-mix(in srgb, var(--color-success) 28%, var(--border-color));
    background: color-mix(in srgb, var(--color-success) 10%, var(--bg-surface));
  }

  .domain-status-badge.warning {
    color: var(--color-warning);
    border-color: color-mix(in srgb, var(--color-warning) 30%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 10%, var(--bg-surface));
  }

  .domain-status-badge.danger {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 28%, var(--border-color));
    background: color-mix(in srgb, var(--color-danger) 9%, var(--bg-surface));
  }
</style>
