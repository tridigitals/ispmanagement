<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import { appSettings } from '$lib/stores/settings';
  import { t } from 'svelte-i18n';
  import { formatDateTime } from '$lib/utils/date';
  import { toast } from '$lib/stores/toast';

  export let diagnostics: any;

  function formatBytes(bytes?: number | null) {
    if (bytes === null || bytes === undefined) return $t('common.na') || '—';
    const n = Number(bytes);
    if (!Number.isFinite(n)) return String(bytes);
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let v = n;
    let i = 0;
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024;
      i++;
    }
    return `${v.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success($t('common.copied') || 'Copied');
    } catch {
      toast.error($t('common.copy_failed') || 'Copy failed');
    }
  }
</script>

{#if diagnostics}
  <div class="diag-grid">
    <section class="card">
      <div class="card-head">
        <h2>{$t('superadmin.system.diagnostics.database') || 'Database'}</h2>
        <span
          class:ok={diagnostics.database?.is_connected}
          class:bad={!diagnostics.database?.is_connected}
        >
          {diagnostics.database?.is_connected
            ? $t('superadmin.system.db_connected') || 'Database Connected'
            : $t('superadmin.system.db_disconnected') || 'Database Disconnected'}
        </span>
      </div>

      <div class="kv">
        <div class="row">
          <div class="k">Type</div>
          <div class="v">{diagnostics.database?.database_type || $t('common.na') || '—'}</div>
        </div>
        <div class="row">
          <div class="k">Server</div>
          <div class="v">
            <span class="mono">{diagnostics.database_server_version || $t('common.na') || '—'}</span
            >
            {#if diagnostics.database_server_version}
              <button
                class="icon-btn"
                onclick={() => copy(diagnostics.database_server_version)}
                title="Copy"
              >
                <Icon name="copy" size={14} />
              </button>
            {/if}
          </div>
        </div>
        <div class="row">
          <div class="k">Size</div>
          <div class="v">{formatBytes(diagnostics.database?.database_size_bytes)}</div>
        </div>
        <div class="row">
          <div class="k">Tables</div>
          <div class="v">{diagnostics.database?.total_tables ?? ($t('common.na') || '—')}</div>
        </div>
        <div class="row">
          <div class="k">Tenants</div>
          <div class="v">{diagnostics.database?.tenants_count ?? ($t('common.na') || '—')}</div>
        </div>
        <div class="row">
          <div class="k">Users</div>
          <div class="v">{diagnostics.database?.users_count ?? ($t('common.na') || '—')}</div>
        </div>
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>{$t('superadmin.system.diagnostics.migrations') || 'Migrations'}</h2>
        <div class="pill-group">
          <span class="pill">
            {$t('superadmin.system.diagnostics.applied') || 'Applied'}:
            <span class="mono">{diagnostics.migrations?.applied_count ?? 0}</span>
          </span>
          <span class="pill">
            {$t('superadmin.system.diagnostics.resolved') || 'Resolved'}:
            <span class="mono">{diagnostics.migrations?.resolved_count ?? 0}</span>
          </span>
        </div>
      </div>

      {#if (diagnostics.migrations?.missing_count ?? 0) > 0}
        <div class="banner danger">
          <Icon name="alert-triangle" size={16} />
          <div>
            <div class="b-title">Missing migrations detected</div>
            <div class="b-sub mono">
              {diagnostics.migrations?.missing_versions?.join(', ') || ''}
            </div>
          </div>
        </div>
      {/if}

      {#if (diagnostics.migrations?.pending_count ?? 0) > 0}
        <div class="banner warn">
          <Icon name="alert-circle" size={16} />
          <div>
            <div class="b-title">Pending migrations</div>
            <div class="b-sub mono">
              {diagnostics.migrations?.pending_versions?.join(', ') || ''}
            </div>
          </div>
        </div>
      {/if}

      <div class="kv">
        <div class="row">
          <div class="k">Latest Applied</div>
          <div class="v mono">
            {diagnostics.migrations?.latest_applied_version ?? ($t('common.na') || '—')}
          </div>
        </div>
      </div>

      <details class="details">
        <summary
          >{$t('superadmin.system.diagnostics.show_applied') || 'Show applied migrations'}</summary
        >
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Version</th>
                <th>Description</th>
                <th>Installed</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {#each diagnostics.applied_migrations || [] as m}
                <tr>
                  <td class="mono">{m.version}</td>
                  <td>{m.description}</td>
                  <td class="mono">
                    {formatDateTime(m.installed_on, { timeZone: $appSettings.app_timezone })}
                  </td>
                  <td>
                    {#if m.success}
                      <span class="tag ok">OK</span>
                    {:else}
                      <span class="tag bad">FAILED</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </details>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>{$t('superadmin.system.diagnostics.platform') || 'Platform Settings'}</h2>
      </div>
      <div class="kv">
        <div class="row">
          <div class="k">App Name</div>
          <div class="v">{diagnostics.settings?.app_name || $t('common.na') || '—'}</div>
        </div>
        <div class="row">
          <div class="k">Public URL</div>
          <div class="v">
            <span class="mono"
              >{diagnostics.settings?.app_public_url || $t('common.na') || '—'}</span
            >
            {#if diagnostics.settings?.app_public_url}
              <button
                class="icon-btn"
                onclick={() => copy(diagnostics.settings.app_public_url)}
                title="Copy"
              >
                <Icon name="copy" size={14} />
              </button>
            {/if}
          </div>
        </div>
        <div class="row">
          <div class="k">Timezone</div>
          <div class="v mono">{diagnostics.settings?.app_timezone || $t('common.na') || '—'}</div>
        </div>
        <div class="row">
          <div class="k">Base Currency</div>
          <div class="v mono">
            {diagnostics.settings?.base_currency_code || $t('common.na') || '—'}
          </div>
        </div>
        <div class="row">
          <div class="k">Display Currency</div>
          <div class="v mono">{diagnostics.settings?.currency_code || $t('common.na') || '—'}</div>
        </div>
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>{$t('superadmin.system.diagnostics.backups') || 'Backups'}</h2>
      </div>
      <div class="kv">
        <div class="row">
          <div class="k">Global</div>
          <div class="v">
            {#if diagnostics.backups?.global_enabled}
              <span class="tag ok">Enabled</span>
            {:else}
              <span class="tag muted">Disabled</span>
            {/if}
          </div>
        </div>
        <div class="row">
          <div class="k">Schedule</div>
          <div class="v mono">
            {diagnostics.backups?.global_mode || '—'}
            {#if diagnostics.backups?.global_mode === 'minute' || diagnostics.backups?.global_mode === 'hour'}
              /{diagnostics.backups?.global_every ?? '—'}
            {:else}
              @{diagnostics.backups?.global_at || '—'}
              {diagnostics.backups?.global_weekday ? `(${diagnostics.backups.global_weekday})` : ''}
            {/if}
          </div>
        </div>
        <div class="row">
          <div class="k">Last Run (UTC)</div>
          <div class="v mono">
            {diagnostics.backups?.global_last_run_utc
              ? formatDateTime(diagnostics.backups.global_last_run_utc, { timeZone: 'UTC' })
              : $t('common.na') || '—'}
          </div>
        </div>
        <div class="row">
          <div class="k">Retention</div>
          <div class="v mono">
            {diagnostics.backups?.global_retention_days ?? ($t('common.na') || '—')} days
          </div>
        </div>

        <div class="divider"></div>

        <div class="row">
          <div class="k">Tenant</div>
          <div class="v">
            {#if diagnostics.backups?.tenant_enabled}
              <span class="tag ok">Enabled</span>
            {:else}
              <span class="tag muted">Disabled</span>
            {/if}
          </div>
        </div>
        <div class="row">
          <div class="k">Schedule</div>
          <div class="v mono">
            {diagnostics.backups?.tenant_mode || '—'}
            {#if diagnostics.backups?.tenant_mode === 'minute' || diagnostics.backups?.tenant_mode === 'hour'}
              /{diagnostics.backups?.tenant_every ?? '—'}
            {:else}
              @{diagnostics.backups?.tenant_at || '—'}
              {diagnostics.backups?.tenant_weekday ? `(${diagnostics.backups.tenant_weekday})` : ''}
            {/if}
          </div>
        </div>
        <div class="row">
          <div class="k">Retention</div>
          <div class="v mono">
            {diagnostics.backups?.tenant_retention_days ?? ($t('common.na') || '—')} days
          </div>
        </div>
      </div>
    </section>
  </div>

  <div class="foot">
    <Icon name="clock" size={14} />
    {$t('superadmin.system.diagnostics.collected_at') || 'Collected at:'}
    <span class="mono">
      {formatDateTime(diagnostics.collected_at, { timeZone: $appSettings.app_timezone })}
    </span>
  </div>
{/if}

<style>
  .diag-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
    gap: 1.5rem;
    margin-top: 1.5rem;
  }

  .card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .card h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .ok,
  .bad {
    font-size: 0.85rem;
    font-weight: 600;
    padding: 0.25rem 0.5rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
  }

  .ok {
    border-color: rgba(34, 197, 94, 0.35);
    color: rgba(34, 197, 94, 0.95);
    background: rgba(34, 197, 94, 0.08);
  }

  .bad {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgba(239, 68, 68, 0.95);
    background: rgba(239, 68, 68, 0.08);
  }

  .kv {
    display: grid;
    gap: 0.55rem;
  }

  .row {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: 0.75rem;
    align-items: center;
  }

  .k {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .v {
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    font-size: 0.9rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icon-btn {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    background: transparent;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: 0.15s ease;
    flex: 0 0 auto;
  }

  .icon-btn:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .pill-group {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .pill {
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    padding: 0.25rem 0.55rem;
    border-radius: 999px;
    font-size: 0.85rem;
  }

  .banner {
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
    border-radius: 12px;
    padding: 0.75rem 0.85rem;
    margin: 0.75rem 0;
    border: 1px solid var(--border-color);
  }

  .banner.danger {
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.08);
    color: rgba(239, 68, 68, 0.95);
  }

  .banner.warn {
    border-color: rgba(245, 158, 11, 0.3);
    background: rgba(245, 158, 11, 0.08);
    color: rgba(245, 158, 11, 0.95);
  }

  .b-title {
    font-weight: 700;
    font-size: 0.9rem;
    margin-bottom: 0.15rem;
  }

  .b-sub {
    color: inherit;
    opacity: 0.9;
    font-size: 0.85rem;
  }

  .details {
    margin-top: 0.75rem;
  }

  .details summary {
    cursor: pointer;
    color: var(--text-secondary);
    font-weight: 600;
    user-select: none;
  }

  .table-wrap {
    margin-top: 0.75rem;
    overflow: auto;
    border-radius: 12px;
    border: 1px solid var(--border-color);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
  }

  th,
  td {
    padding: 0.6rem 0.75rem;
    border-bottom: 1px solid var(--border-color);
    text-align: left;
    vertical-align: top;
  }

  th {
    color: var(--text-secondary);
    font-weight: 700;
    background: rgba(255, 255, 255, 0.03);
    position: sticky;
    top: 0;
    z-index: 1;
  }

  tr:last-child td {
    border-bottom: none;
  }

  .tag {
    font-size: 0.75rem;
    padding: 0.18rem 0.45rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    display: inline-block;
  }

  .tag.ok {
    border-color: rgba(34, 197, 94, 0.35);
    color: rgba(34, 197, 94, 0.95);
    background: rgba(34, 197, 94, 0.08);
  }

  .tag.bad {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgba(239, 68, 68, 0.95);
    background: rgba(239, 68, 68, 0.08);
  }

  .tag.muted {
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.03);
  }

  .divider {
    height: 1px;
    background: var(--border-color);
    margin: 0.75rem 0;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-top: 1.5rem;
    border-top: 1px solid var(--border-color);
    padding-top: 1rem;
  }

  @media (max-width: 600px) {
    .row {
      grid-template-columns: 1fr;
    }
    .k {
      font-size: 0.85rem;
    }
  }
</style>
