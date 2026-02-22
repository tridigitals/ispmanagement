<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import IncidentRunbook from '$lib/components/network/IncidentRunbook.svelte';
  import IncidentTimeline from '$lib/components/network/IncidentTimeline.svelte';

  type IncidentRow = {
    id: string;
    router_id: string;
    interface_name?: string | null;
    incident_type: string;
    severity: string;
    status: string;
    title: string;
    message: string;
    owner_user_id?: string | null;
    notes?: string | null;
    is_auto_escalated?: boolean;
    escalated_at?: string | null;
    first_seen_at?: string | null;
    acked_at?: string | null;
    last_seen_at: string;
    resolved_at?: string | null;
    updated_at: string;
  };

  type RouterRow = {
    identity?: string | null;
    name?: string | null;
    host?: string | null;
    port?: number | null;
    latency_ms?: number | null;
  };

  type RouterMetricRow = {
    cpu_load?: number | null;
    rx_bps?: number | null;
    tx_bps?: number | null;
    free_memory_bytes?: number | null;
    total_memory_bytes?: number | null;
  };

  type TeamMemberLite = {
    user_id?: string;
    name: string;
    email: string;
  };

  type RunbookStep = {
    title: string;
    detail?: string;
    command?: string;
  };

  type ActivityItem = {
    ts: string;
    title: string;
    detail?: string;
  };

  let {
    open = false,
    incident = null,
    loading = false,
    router = null,
    metric = null,
    teamMembers = [],
    selectedOwnerId = '',
    draftNotes = '',
    saving = false,
    canManage = false,
    emailNotifyEnabled = false,
    slaState = 'ok',
    slaOpenDuration = '—',
    appTimezone = 'UTC',
    runbookSteps = [],
    activityItems = [],
    ownerLabel,
    typeLabel,
    severityLabel,
    formatDateTime,
    formatBps,
    memoryUsePct,
    onClose,
    onOpenRouter,
    onAcknowledge,
    onResolve,
    onSave,
    onOpenNetworkSettings,
    onOwnerChange,
    onNotesChange,
    onCopyRunbookCommand,
    onAddRunbookStep,
  }: {
    open?: boolean;
    incident?: IncidentRow | null;
    loading?: boolean;
    router?: RouterRow | null;
    metric?: RouterMetricRow | null;
    teamMembers?: TeamMemberLite[];
    selectedOwnerId?: string;
    draftNotes?: string;
    saving?: boolean;
    canManage?: boolean;
    emailNotifyEnabled?: boolean;
    slaState?: 'ok' | 'warn' | 'breach' | string;
    slaOpenDuration?: string;
    appTimezone?: string;
    runbookSteps?: RunbookStep[];
    activityItems?: ActivityItem[];
    ownerLabel: (ownerUserId?: string | null) => string;
    typeLabel: (incidentType: string) => string;
    severityLabel: (severity: string) => string;
    formatDateTime: (value: string, options?: Record<string, unknown>) => string;
    formatBps: (value?: number | null) => string;
    memoryUsePct: (total?: number | null, free?: number | null) => number | null;
    onClose: () => void;
    onOpenRouter: (routerId: string) => void;
    onAcknowledge: (id: string) => void | Promise<void>;
    onResolve: (id: string) => void | Promise<void>;
    onSave: () => void | Promise<void>;
    onOpenNetworkSettings: () => void;
    onOwnerChange: (value: string) => void;
    onNotesChange: (value: string) => void;
    onCopyRunbookCommand: (command: string) => void | Promise<void>;
    onAddRunbookStep: (step: RunbookStep) => void;
  } = $props();
</script>

{#if open && incident}
  <button class="drawer-backdrop" type="button" onclick={onClose} aria-label={$t('common.close') || 'Close'}></button>
  <aside class="drawer" aria-label={$t('common.details') || 'Details'}>
    <div class="drawer-head">
      <div>
        <div class="drawer-title">{$t('common.details') || 'Details'}</div>
        <div class="drawer-sub">{incident.title}</div>
      </div>
      <button class="icon-btn" type="button" onclick={onClose} title={$t('common.close') || 'Close'}>
        <Icon name="x" size={16} />
      </button>
    </div>

    <div class="drawer-body">
      {#if loading}
        <div class="muted">{$t('common.loading') || 'Loading...'}</div>
      {/if}

      <div class="detail-grid">
        <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.status') || 'Status'}</span><span class="mono">{incident.status}</span></div>
        <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.type') || 'Type'}</span><span class="mono">{typeLabel(incident.incident_type)}</span></div>
        <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.severity') || 'Severity'}</span><span class="mono">{severityLabel(incident.severity)}</span></div>
        <div class="drow">
          <span class="muted">{$t('admin.network.incidents.labels.auto_escalated') || 'Auto Escalated'}</span>
          <span class="mono">
            {incident.is_auto_escalated
              ? formatDateTime(incident.escalated_at || incident.updated_at, { timeZone: appTimezone })
              : ($t('common.no') || 'No')}
          </span>
        </div>
        <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.seen') || 'Last Seen'}</span><span class="mono">{formatDateTime(incident.last_seen_at, { timeZone: appTimezone })}</span></div>
        <div class="drow">
          <span class="muted">{$t('admin.network.incidents.drawer.email_notify') || 'Email notify'}</span>
          <span class="mono">
            <span class:flag-on={emailNotifyEnabled} class:flag-off={!emailNotifyEnabled} class="flag">
              {emailNotifyEnabled
                ? $t('admin.network.incidents.drawer.on') || 'On'
                : $t('admin.network.incidents.drawer.off') || 'Off'}
            </span>
          </span>
        </div>
        <div class="drow">
          <span class="muted">{$t('admin.network.incidents.sla.title') || 'SLA Timer'}</span>
          <span class="mono">
            <span class="sla-badge" class:warn={slaState === 'warn'} class:breach={slaState === 'breach'}>
              {slaOpenDuration}
            </span>
          </span>
        </div>
        <div class="drow"><span class="muted">{$t('admin.network.incidents.drawer.assignee') || 'Assignee'}</span><span class="mono">{ownerLabel(incident.owner_user_id)}</span></div>
        <div class="drow"><span class="muted">Router</span><span class="mono">{router?.identity || router?.name || incident.router_id}</span></div>
        <div class="drow"><span class="muted">Interface</span><span class="mono">{incident.interface_name || '-'}</span></div>
        {#if router}
          <div class="drow"><span class="muted">Host</span><span class="mono">{router.host}:{router.port}</span></div>
          <div class="drow"><span class="muted">Latency</span><span class="mono">{router.latency_ms == null ? '—' : `${router.latency_ms} ms`}</span></div>
        {/if}
        {#if metric}
          <div class="drow"><span class="muted">CPU</span><span class="mono">{metric.cpu_load == null ? '—' : `${metric.cpu_load}%`}</span></div>
          <div class="drow"><span class="muted">RX/TX</span><span class="mono">{formatBps(metric.rx_bps)} / {formatBps(metric.tx_bps)}</span></div>
          <div class="drow"><span class="muted">Memory Use</span><span class="mono">{memoryUsePct(metric.total_memory_bytes, metric.free_memory_bytes) == null ? '—' : `${memoryUsePct(metric.total_memory_bytes, metric.free_memory_bytes)}%`}</span></div>
        {/if}
      </div>

      <div class="detail-message">{incident.message}</div>

      <div class="detail-edit">
        <div class="field">
          <label for="incident-owner">{$t('admin.network.incidents.drawer.assignee') || 'Assignee'}</label>
          {#if canManage}
            <select
              id="incident-owner"
              class="input"
              value={selectedOwnerId}
              onchange={(e) => onOwnerChange((e.currentTarget as HTMLSelectElement).value)}
            >
              <option value="">{($t('admin.network.incidents.drawer.unassigned') || 'Unassigned')}</option>
              {#each teamMembers as member}
                <option value={member.user_id}>{member.name} ({member.email})</option>
              {/each}
            </select>
          {:else}
            <div class="readonly">{ownerLabel(incident.owner_user_id)}</div>
          {/if}
        </div>
        <div class="field">
          <label for="incident-notes">{$t('admin.network.incidents.drawer.notes') || 'Notes'}</label>
          {#if canManage}
            <textarea
              id="incident-notes"
              class="textarea"
              rows="4"
              value={draftNotes}
              oninput={(e) => onNotesChange((e.currentTarget as HTMLTextAreaElement).value)}
              placeholder={$t('admin.network.incidents.drawer.notes_placeholder') || 'Add operator notes, handover context, and actions...'}
            ></textarea>
          {:else}
            <div class="readonly">{incident.notes || ($t('common.na') || '—')}</div>
          {/if}
        </div>
        {#if canManage}
          <div class="save-row">
            <button class="btn ghost" type="button" onclick={() => void onSave()} disabled={saving}>
              <Icon name="save" size={16} />
              {saving
                ? $t('common.saving') || 'Saving...'
                : $t('admin.network.incidents.drawer.save') || 'Save Notes'}
            </button>
          </div>
          <div class="save-row">
            <button class="btn ghost" type="button" onclick={onOpenNetworkSettings}>
              <Icon name="settings" size={16} />
              {$t('admin.network.incidents.drawer.open_network_settings') || 'Open Network Settings'}
            </button>
          </div>
        {/if}
      </div>

      <IncidentRunbook
        steps={runbookSteps}
        {canManage}
        onCopyCommand={onCopyRunbookCommand}
        onAddStep={onAddRunbookStep}
      />

      <IncidentTimeline items={activityItems} />
    </div>

    <div class="drawer-actions">
      <button class="btn ghost" type="button" onclick={() => onOpenRouter(incident.router_id)}>
        <Icon name="arrow-right" size={16} />
        {$t('common.open') || 'Open'}
      </button>
      {#if incident.status !== 'ack' && incident.status !== 'resolved' && canManage}
        <button class="btn ghost" type="button" onclick={() => void onAcknowledge(incident.id)}>
          <Icon name="check" size={16} />
          {$t('admin.network.alerts.actions.ack') || 'Acknowledge'}
        </button>
      {/if}
      {#if incident.status !== 'resolved' && canManage}
        <button class="btn ghost" type="button" onclick={() => void onResolve(incident.id)}>
          <Icon name="check-circle" size={16} />
          {$t('admin.network.alerts.actions.resolve') || 'Resolve'}
        </button>
      {/if}
    </div>
  </aside>
{/if}

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 50;
  }
  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    width: min(560px, 92vw);
    height: 100vh;
    background: var(--bg-surface);
    border-left: 1px solid var(--border-color);
    z-index: 51;
    display: grid;
    grid-template-rows: auto 1fr auto;
  }
  .drawer-head {
    padding: 16px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .drawer-title {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .drawer-sub {
    margin-top: 6px;
    font-size: 1.05rem;
    font-weight: 900;
    color: var(--text-primary);
  }
  .drawer-body {
    padding: 16px;
    display: grid;
    gap: 14px;
    overflow: auto;
  }
  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .drow {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 10px;
    display: grid;
    gap: 4px;
  }
  .flag {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 3px 9px;
    font-size: 0.72rem;
    font-weight: 800;
    border: 1px solid var(--border-color);
  }
  .flag-on {
    color: var(--color-success);
    border-color: color-mix(in srgb, var(--color-success) 45%, var(--border-color));
  }
  .flag-off {
    color: var(--text-secondary);
    border-color: var(--border-color);
  }
  .sla-badge {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 0.7rem;
    font-weight: 800;
    white-space: nowrap;
  }
  .sla-badge.warn {
    color: var(--color-warning);
    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
  }
  .sla-badge.breach {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 45%, var(--border-color));
  }
  .detail-message {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 12px;
    color: var(--text-primary);
    line-height: 1.45;
    white-space: pre-wrap;
  }
  .detail-edit {
    display: grid;
    gap: 10px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 12px;
    background: color-mix(in srgb, var(--bg-card) 70%, transparent);
  }
  .field {
    display: grid;
    gap: 6px;
  }
  .field label {
    color: var(--text-secondary);
    font-size: 0.8rem;
    font-weight: 700;
  }
  .input,
  .textarea {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 10px 12px;
    outline: none;
  }
  .input:focus,
  .textarea:focus {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
  }
  .readonly {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
    color: var(--text-primary);
    padding: 10px 12px;
    white-space: pre-wrap;
  }
  .save-row {
    display: flex;
    justify-content: flex-end;
  }
  .drawer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
    padding: 12px 16px;
    border-top: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 80%, transparent);
  }
  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--bg-hover);
  }
  @media (max-width: 720px) {
    .detail-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
