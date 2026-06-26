<script lang="ts">
  import { t } from 'svelte-i18n';
  import type { AuditLog } from '$lib/api/client';

  let { log } = $props<{
    log: AuditLog;
  }>();

  function safeParseJson(input: string | null) {
    if (!input) return null;
    try {
      return JSON.parse(input);
    } catch {
      return null;
    }
  }

  function shortId(id: unknown) {
    const s = String(id || '');
    if (!s) return '—';
    return s.length > 10 ? `${s.slice(0, 8)}…` : s;
  }

  function fmtBool(v: unknown) {
    if (v === true) return $t('common.yes');
    if (v === false) return $t('common.no');
    return '—';
  }

  function pick(obj: any, path: string) {
    try {
      return path.split('.').reduce((acc: any, k) => acc?.[k], obj);
    } catch {
      return undefined;
    }
  }

  const parsed = $derived.by(() => safeParseJson(log.details));
  const pretty = $derived.by(() => (parsed ? JSON.stringify(parsed, null, 2) : null));

  const annChanged = $derived.by(() => {
    if (log.resource !== 'announcements') return [];
    if (log.action !== 'update') return [];
    const v = pick(parsed, 'changed');
    return Array.isArray(v) ? v : [];
  });
</script>

<div class="details-block">
  <div class="details-title">
    {$t('superadmin.audit_logs.labels.details')}
  </div>

  {#if parsed && typeof parsed === 'object'}
    {#if log.resource === 'announcements'}
      <div class="kv-grid">
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.action')}</div>
          <div class="v"><span class="pill">{log.action}</span></div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.scope')}</div>
          <div class="v">{pick(parsed, 'scope') || (log.tenant_id ? 'tenant' : 'global')}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('common.title')}</div>
          <div class="v">{pick(parsed, 'announcement.title') || '—'}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.severity')}</div>
          <div class="v">{pick(parsed, 'announcement.severity') || '—'}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.audience')}</div>
          <div class="v">{pick(parsed, 'announcement.audience') || '—'}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.mode')}</div>
          <div class="v">{pick(parsed, 'announcement.mode') || '—'}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.deliver_in_app')}</div>
          <div class="v">{fmtBool(pick(parsed, 'announcement.deliver_in_app'))}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.deliver_email')}</div>
          <div class="v">{fmtBool(pick(parsed, 'announcement.deliver_email'))}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.starts')}</div>
          <div class="v">{pick(parsed, 'announcement.starts_at') || '—'}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.ends')}</div>
          <div class="v">{pick(parsed, 'announcement.ends_at') || '—'}</div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns_extended.notified')}</div>
          <div class="v">{pick(parsed, 'announcement.notified_at') || '—'}</div>
        </div>
        {#if pick(parsed, 'cause')}
          <div class="kv">
            <div class="k">{$t('superadmin.audit_logs.columns_extended.cause')}</div>
            <div class="v">{pick(parsed, 'cause')}</div>
          </div>
        {/if}
        {#if typeof pick(parsed, 'delivered_immediately') !== 'undefined'}
          <div class="kv">
            <div class="k">{$t('superadmin.audit_logs.columns_extended.delivered_now')}</div>
            <div class="v">{fmtBool(pick(parsed, 'delivered_immediately'))}</div>
          </div>
        {/if}
      </div>

      {#if annChanged.length > 0}
        <div class="sub-block">
          <div class="sub-title">{$t('superadmin.audit_logs.columns_extended.changed')}</div>
          <div class="chips">
            {#each annChanged as f}
              <span class="chip">{f}</span>
            {/each}
          </div>
        </div>
      {/if}
    {:else if log.resource === 'support_ticket'}
      <div class="kv-grid">
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns.action')}</div>
          <div class="v"><span class="pill">{log.action}</span></div>
        </div>
        <div class="kv">
          <div class="k">{$t('superadmin.audit_logs.columns_extended.subject')}</div>
          <div class="v">{pick(parsed, 'subject') || '—'}</div>
        </div>
        {#if pick(parsed, 'priority')}
          <div class="kv">
            <div class="k">{$t('superadmin.audit_logs.columns_extended.priority')}</div>
            <div class="v">{pick(parsed, 'priority')}</div>
          </div>
        {/if}
        {#if typeof pick(parsed, 'is_internal') !== 'undefined'}
          <div class="kv">
            <div class="k">{$t('superadmin.audit_logs.columns_extended.internal')}</div>
            <div class="v">{fmtBool(pick(parsed, 'is_internal'))}</div>
          </div>
        {/if}
        {#if pick(parsed, 'message_id')}
          <div class="kv">
            <div class="k">{$t('superadmin.audit_logs.columns_extended.message')}</div>
            <div class="v text-mono">{shortId(pick(parsed, 'message_id'))}</div>
          </div>
        {/if}
        {#if typeof pick(parsed, 'attachments') !== 'undefined'}
          <div class="kv">
            <div class="k">{$t('superadmin.audit_logs.columns_extended.attachments')}</div>
            <div class="v">{pick(parsed, 'attachments')}</div>
          </div>
        {/if}
        {#if pick(parsed, 'from.status') || pick(parsed, 'to.status')}
          <div class="kv">
            <div class="k">{$t('common.status')}</div>
            <div class="v">
              {pick(parsed, 'from.status') || '—'} → {pick(parsed, 'to.status') || '—'}
            </div>
          </div>
        {/if}
        {#if pick(parsed, 'from.assigned_to') || pick(parsed, 'to.assigned_to')}
          <div class="kv">
            <div class="k">{$t('common.assignee')}</div>
            <div class="v">
              <span class="text-mono">{shortId(pick(parsed, 'from.assigned_to'))}</span>
              →
              <span class="text-mono">{shortId(pick(parsed, 'to.assigned_to'))}</span>
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <div class="details-text">{pretty}</div>
    {/if}
  {:else}
    <div class="details-text">
      {log.details || '—'}
    </div>
  {/if}
</div>

<style>
  .details-block {
    margin-top: 1rem;
    padding: 0.75rem;
    background: var(--bg-primary);
    border-radius: 8px;
    border: 1px solid var(--border-color);
  }

  :global([data-theme='light']) .details-block {
    background: var(--bg-primary);
    border-color: var(--border-color);
  }

  .details-title {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-secondary);
    margin-bottom: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .details-text {
    font-family: var(--font-mono);
    font-size: 0.85rem;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-all;
  }

  .kv-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem 1rem;
    margin-top: 0.25rem;
  }

  @media (max-width: 720px) {
    .kv-grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .kv {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
  }

  .k {
    font-size: 0.72rem;
    font-weight: 700;
    color: var(--text-secondary);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .v {
    font-size: 0.9rem;
    color: var(--text-primary);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .text-mono {
    font-family: var(--font-mono);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 0.15rem 0.45rem;
    border-radius: 999px;
    background: var(--color-primary-subtle);
    border: 1px solid color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
    color: var(--text-primary);
    font-weight: 700;
    font-size: 0.75rem;
  }

  :global([data-theme='light']) .pill {
    background: var(--color-primary-subtle);
    border-color: color-mix(in srgb, var(--color-primary) 25%, var(--border-color));
  }

  .sub-block {
    margin-top: 0.85rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
  }

  :global([data-theme='light']) .sub-block {
    border-top-color: var(--border-color);
  }

  .sub-title {
    font-size: 0.72rem;
    font-weight: 800;
    color: var(--text-secondary);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin-bottom: 0.35rem;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    font-size: 0.78rem;
    font-family: var(--font-mono);
  }

  :global([data-theme='light']) .chip {
    background: var(--bg-tertiary);
    border-color: var(--border-color);
  }
</style>
