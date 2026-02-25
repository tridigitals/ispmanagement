<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { api, type InstallationWorkOrderView, type TeamMember } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import { formatDateTime } from '$lib/utils/date';
  import Icon from '$lib/components/ui/Icon.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';

  let loading = $state(true);
  let busyId = $state<string | null>(null);
  let rows = $state<InstallationWorkOrderView[]>([]);
  let team = $state<TeamMember[]>([]);
  let includeClosed = $state(false);
  let search = $state('');
  let statusFilter = $state('all');

  let detailOpen = $state(false);
  let activeRow = $state<InstallationWorkOrderView | null>(null);
  let formAssignee = $state('');
  let formSchedule = $state('');
  let formNotes = $state('');
  let checkCable = $state(false);
  let checkOnt = $state(false);
  let checkPppoe = $state(false);
  let checkSpeed = $state(false);
  let canReadTeam = $derived(
    $can('read', 'team') || $can('create', 'team') || $can('update', 'team') || $can('delete', 'team'),
  );

  const filteredRows = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return rows.filter((row) => {
      if (statusFilter !== 'all' && row.status !== statusFilter) return false;
      if (!q) return true;
      const hay = [
        row.customer_name,
        row.location_label,
        row.package_name,
        row.assigned_to_name,
        row.status,
      ]
        .filter(Boolean)
        .join(' ')
        .toLowerCase();
      return hay.includes(q);
    });
  });
  const stats = $derived.by(() => ({
    total: rows.length,
    pending: rows.filter((r) => r.status === 'pending').length,
    inProgress: rows.filter((r) => r.status === 'in_progress').length,
    completed: rows.filter((r) => r.status === 'completed').length,
  }));

  onMount(() => {
    if (!$can('read', 'work_orders') && !$can('manage', 'work_orders')) {
      goto('/unauthorized');
      return;
    }
    void loadAll();
  });

  async function loadAll() {
    loading = true;
    try {
      const [workOrders, members] = await Promise.all([
        api.workOrders.list({ include_closed: includeClosed, limit: 300 }),
        canReadTeam ? api.team.list().catch(() => [] as TeamMember[]) : Promise.resolve([] as TeamMember[]),
      ]);
      rows = workOrders;
      team = members;
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load installation work orders');
    } finally {
      loading = false;
    }
  }

  async function setStatus(row: InstallationWorkOrderView, action: 'start' | 'complete' | 'cancel', notes?: string) {
    busyId = row.id;
    try {
      if (action === 'start') await api.workOrders.start(row.id, notes);
      if (action === 'complete') await api.workOrders.complete(row.id, notes);
      if (action === 'cancel') await api.workOrders.cancel(row.id, notes);
      toast.success($t(`admin.network.installations.${action}_ok`) || 'Updated');
      await loadAll();
      if (activeRow?.id === row.id) {
        closeDetail();
      }
    } catch (e: any) {
      toast.error(e?.message || 'Update failed');
    } finally {
      busyId = null;
    }
  }

  function statusClass(status: string) {
    if (status === 'pending') return 'status pending';
    if (status === 'in_progress') return 'status progress';
    if (status === 'completed') return 'status completed';
    if (status === 'cancelled') return 'status cancelled';
    return 'status';
  }

  function openDetail(row: InstallationWorkOrderView) {
    activeRow = row;
    formAssignee = row.assigned_to || '';
    formSchedule = row.scheduled_at ? toLocalInputValue(row.scheduled_at) : '';
    formNotes = '';
    detailOpen = true;
    checkCable = false;
    checkOnt = false;
    checkPppoe = false;
    checkSpeed = false;
  }

  function closeDetail() {
    detailOpen = false;
    activeRow = null;
    formAssignee = '';
    formSchedule = '';
    formNotes = '';
  }

  function toLocalInputValue(raw: string) {
    const d = new Date(raw);
    if (!Number.isFinite(d.getTime())) return '';
    const pad = (n: number) => `${n}`.padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function buildChecklistNote() {
    const lines = [
      `${checkCable ? '[x]' : '[ ]'} Cable installed`,
      `${checkOnt ? '[x]' : '[ ]'} ONT installed`,
      `${checkPppoe ? '[x]' : '[ ]'} PPPoE configured`,
      `${checkSpeed ? '[x]' : '[ ]'} Speed test passed`,
    ];
    return `Installation checklist:\n${lines.join('\n')}`;
  }

  async function savePlan() {
    const row = activeRow;
    if (!row) return;
    const assigned_to = formAssignee.trim();
    if (!assigned_to) {
      toast.error($t('admin.network.installations.assign_required') || 'Choose assignee first');
      return;
    }
    busyId = row.id;
    try {
      const extra = formNotes.trim();
      const checklist = buildChecklistNote();
      const note = extra ? `${extra}\n\n${checklist}` : checklist;
      await api.workOrders.assign(row.id, {
        assigned_to,
        scheduled_at: formSchedule ? new Date(formSchedule).toISOString() : undefined,
        notes: note,
      });
      toast.success($t('admin.network.installations.assigned') || 'Assigned');
      await loadAll();
      const refreshed = rows.find((x) => x.id === row.id);
      if (refreshed) openDetail(refreshed);
    } catch (e: any) {
      toast.error(e?.message || 'Assign failed');
    } finally {
      busyId = null;
    }
  }

  function tr(key: string, fallback: string) {
    const value = $t(key);
    return value && value !== key ? value : fallback;
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={tr('admin.network.installations.title', 'Installation Work Orders')}
    subtitle={tr('admin.network.installations.subtitle', 'Technician pipeline from paid invoices to active service')}
  >
    {#snippet actions()}
      <button
        class="btn ghost"
        type="button"
        onclick={() => {
          includeClosed = !includeClosed;
          void loadAll();
        }}
      >
        {includeClosed
          ? tr('admin.network.installations.hide_closed', 'Hide closed')
          : tr('admin.network.installations.show_closed', 'Show closed')}
      </button>
      <button class="btn ghost" type="button" onclick={() => void loadAll()}>
        <Icon name="refresh-cw" size={14} />
        {tr('common.refresh', 'Refresh')}
      </button>
    {/snippet}
  </NetworkPageHeader>

  <div class="stats-grid">
    <article class="stat-card">
      <div class="stat-label">{tr('common.total', 'Total')}</div>
      <div class="stat-value">{stats.total}</div>
    </article>
    <article class="stat-card warning">
      <div class="stat-label">{tr('common.pending', 'Pending')}</div>
      <div class="stat-value">{stats.pending}</div>
    </article>
    <article class="stat-card info">
      <div class="stat-label">{tr('common.in_progress', 'In Progress')}</div>
      <div class="stat-value">{stats.inProgress}</div>
    </article>
    <article class="stat-card success">
      <div class="stat-label">{tr('common.completed', 'Completed')}</div>
      <div class="stat-value">{stats.completed}</div>
    </article>
  </div>

  <div class="filters-wrap">
    <NetworkFilterPanel>
      <div class="control control-wide">
        <label for="installations-search">{tr('common.search', 'Search')}</label>
        <label class="search-wrap" for="installations-search">
          <Icon name="search" size={14} />
          <input
            id="installations-search"
            type="text"
            placeholder={tr('admin.network.installations.search', 'Search work orders...')}
            bind:value={search}
          />
        </label>
      </div>

      <div class="control">
        <label for="installations-status">{tr('common.status', 'Status')}</label>
        <select id="installations-status" class="input" bind:value={statusFilter}>
          <option value="all">{tr('common.all', 'All')}</option>
          <option value="pending">{tr('common.pending', 'Pending')}</option>
          <option value="in_progress">{tr('common.in_progress', 'In Progress')}</option>
          <option value="completed">{tr('common.completed', 'Completed')}</option>
          <option value="cancelled">{tr('common.cancelled', 'Cancelled')}</option>
        </select>
      </div>
    </NetworkFilterPanel>
  </div>

  {#if loading}
    <div class="card muted">{tr('common.loading', 'Loading...')}</div>
  {:else if filteredRows.length === 0}
    <div class="card muted">{tr('admin.network.installations.empty', 'No installation work orders')}</div>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>{tr('common.customer', 'Customer')}</th>
            <th>{tr('common.location', 'Location')}</th>
            <th>{tr('common.package', 'Package')}</th>
            <th>{tr('common.status', 'Status')}</th>
            <th>{tr('common.assignee', 'Assignee')}</th>
            <th>{tr('common.schedule', 'Schedule')}</th>
            <th>{tr('common.updated_at', 'Updated')}</th>
            <th>{tr('common.actions', 'Actions')}</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredRows as row}
            <tr class="clickable" onclick={() => openDetail(row)}>
              <td>{row.customer_name || row.customer_id}</td>
              <td>{row.location_label || row.location_id}</td>
              <td>{row.package_name || '-'}</td>
              <td><span class={statusClass(row.status)}>{row.status}</span></td>
              <td>{row.assigned_to_name || '-'}</td>
              <td>{row.scheduled_at ? formatDateTime(row.scheduled_at) : '-'}</td>
              <td>{formatDateTime(row.updated_at)}</td>
              <td>
                <div class="actions">
                  <button class="btn ghost" onclick={(e) => { e.stopPropagation(); openDetail(row); }}>
                    {tr('common.view', 'View')}
                  </button>
                  {#if $can('manage', 'work_orders') && row.status === 'pending'}
                    <button class="btn" onclick={(e) => { e.stopPropagation(); setStatus(row, 'start'); }} disabled={busyId === row.id}>
                      {tr('common.start', 'Start')}
                    </button>
                  {/if}
                  {#if $can('manage', 'work_orders') && row.status !== 'completed' && row.status !== 'cancelled'}
                    <button class="btn success" onclick={(e) => { e.stopPropagation(); setStatus(row, 'complete'); }} disabled={busyId === row.id}>
                      {tr('common.complete', 'Complete')}
                    </button>
                    <button class="btn danger" onclick={(e) => { e.stopPropagation(); setStatus(row, 'cancel'); }} disabled={busyId === row.id}>
                      {tr('common.cancel', 'Cancel')}
                    </button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

{#if detailOpen && activeRow}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeDetail();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') closeDetail();
    }}
  >
    <div class="modal">
      <div class="modal-head">
        <h2>{$t('admin.network.installations.details_title') || 'Installation Details'}</h2>
        <button class="btn ghost" onclick={closeDetail}>✕</button>
      </div>

      <div class="meta-grid">
        <div><strong>{$t('common.customer') || 'Customer'}:</strong> {activeRow.customer_name || activeRow.customer_id}</div>
        <div><strong>{$t('common.location') || 'Location'}:</strong> {activeRow.location_label || activeRow.location_id}</div>
        <div><strong>{$t('common.package') || 'Package'}:</strong> {activeRow.package_name || '-'}</div>
        <div><strong>{$t('common.status') || 'Status'}:</strong> <span class={statusClass(activeRow.status)}>{activeRow.status}</span></div>
      </div>

      {#if $can('manage', 'work_orders')}
        <div class="form-grid">
          <label>
            {$t('common.assignee') || 'Assignee'}
            <select bind:value={formAssignee} disabled={busyId === activeRow.id || !canReadTeam}>
              <option value="">-</option>
              {#each team as member}
                <option value={member.user_id}>{member.name || member.email}</option>
              {/each}
            </select>
          </label>
          <label>
            {$t('common.schedule') || 'Schedule'}
            <input type="datetime-local" bind:value={formSchedule} disabled={busyId === activeRow.id} />
          </label>
        </div>

        <fieldset class="checklist">
          <legend>{$t('admin.network.installations.checklist') || 'Installation Checklist'}</legend>
          <label><input type="checkbox" bind:checked={checkCable} /> Cable installed</label>
          <label><input type="checkbox" bind:checked={checkOnt} /> ONT installed</label>
          <label><input type="checkbox" bind:checked={checkPppoe} /> PPPoE configured</label>
          <label><input type="checkbox" bind:checked={checkSpeed} /> Speed test passed</label>
        </fieldset>

        <label class="notes">
          {$t('common.notes') || 'Notes'}
          <textarea rows="5" bind:value={formNotes} placeholder={$t('admin.network.installations.notes_placeholder') || 'Technician notes and onsite findings'}></textarea>
        </label>

        <div class="modal-actions">
          <button class="btn ghost" onclick={savePlan} disabled={busyId === activeRow.id}>
            {$t('admin.network.installations.save_plan') || 'Save Plan'}
          </button>
          {#if activeRow.status === 'pending'}
            <button class="btn" onclick={() => activeRow && setStatus(activeRow, 'start', formNotes)} disabled={busyId === activeRow.id}>
              {$t('common.start') || 'Start'}
            </button>
          {/if}
          {#if activeRow.status !== 'completed' && activeRow.status !== 'cancelled'}
            <button class="btn success" onclick={() => activeRow && setStatus(activeRow, 'complete', formNotes)} disabled={busyId === activeRow.id}>
              {$t('common.complete') || 'Complete'}
            </button>
            <button class="btn danger" onclick={() => activeRow && setStatus(activeRow, 'cancel', formNotes)} disabled={busyId === activeRow.id}>
              {$t('common.cancel') || 'Cancel'}
            </button>
          {/if}
        </div>
        {#if !canReadTeam}
          <p class="helper-text">{$t('common.no_permission') || 'You do not have permission to view this page.'}</p>
        {/if}
      {/if}

      {#if activeRow.notes}
        <div class="history">
          <h3>{$t('admin.network.installations.history') || 'Latest Notes'}</h3>
          <pre>{activeRow.notes}</pre>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .page-content {
    display: grid;
    gap: 12px;
    padding: 28px;
    max-width: 1460px;
    margin: 0 auto;
  }
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
  }
  .stat-card {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    background: var(--bg-card);
    padding: 14px;
  }
  .stat-card.warning {
    box-shadow: 0 0 0 1px rgba(245, 158, 11, 0.16) inset;
  }
  .stat-card.info {
    box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.14) inset;
  }
  .stat-card.success {
    box-shadow: 0 0 0 1px rgba(34, 197, 94, 0.15) inset;
  }
  .stat-label {
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .stat-value {
    margin-top: 8px;
    font-size: 1.7rem;
    font-weight: 950;
    color: var(--text-primary);
  }
  .filters-wrap {
    margin-bottom: 2px;
  }
  .control {
    min-width: 180px;
  }
  .control-wide {
    min-width: 320px;
    flex: 1 1 340px;
  }
  .control label {
    display: block;
    margin-bottom: 6px;
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    color: var(--text-secondary);
  }
  .search-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 0 10px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-card);
    color: var(--text-secondary);
    margin-bottom: 0;
  }
  .input,
  .control :global(select.input) {
    width: 100%;
    min-height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-card);
    color: var(--text-primary);
    padding: 8px 10px;
  }
  .search-wrap input {
    border: 0;
    background: transparent;
    width: 100%;
    padding: 8px 0;
    outline: none;
    color: var(--text-primary);
  }
  .card {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    background: var(--bg-card);
    padding: 16px;
  }
  .muted {
    color: var(--text-secondary);
  }
  .table-wrap {
    overflow: auto;
    border: 1px solid var(--border-color);
    border-radius: 18px;
    background: var(--bg-card);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.2);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    min-width: 980px;
  }
  th,
  td {
    padding: 12px;
    border-bottom: 1px solid var(--border-color);
    text-align: left;
    vertical-align: middle;
  }
  th {
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .btn {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--color-primary);
    color: white;
    font-weight: 800;
    padding: 8px 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }
  .btn.success {
    border-color: rgba(34, 197, 94, 0.28);
    background: rgba(34, 197, 94, 0.14);
    color: rgba(34, 197, 94, 1);
  }
  .btn.danger {
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.14);
    color: rgba(239, 68, 68, 1);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .status {
    display: inline-flex;
    border-radius: 999px;
    border: 1px solid #374157;
    padding: 2px 10px;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .status.pending {
    border-color: #6a5a2b;
    color: #f6c65f;
  }
  .status.progress {
    border-color: #2f5d96;
    color: #7eb4ff;
  }
  .status.completed {
    border-color: #256e43;
    color: #59d091;
  }
  .status.cancelled {
    border-color: #7f2c2c;
    color: #f18989;
  }
  .clickable {
    cursor: pointer;
  }
  .clickable:hover {
    background: color-mix(in srgb, var(--bg-hover), transparent 30%);
  }
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(3, 8, 20, 0.66);
    display: grid;
    place-items: center;
    padding: 20px;
    z-index: 1000;
  }
  .modal {
    width: min(900px, 100%);
    max-height: calc(100vh - 40px);
    overflow: auto;
    border-radius: 14px;
    background: #0b1221;
    border: 1px solid #283149;
    padding: 16px;
    display: grid;
    gap: 14px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }
  .modal-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .modal h2 {
    margin: 0;
    font-size: 1.2rem;
  }
  .meta-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px 14px;
    font-size: 0.95rem;
  }
  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  label {
    display: grid;
    gap: 6px;
    font-size: 0.92rem;
  }
  input[type='datetime-local'],
  textarea {
    background: #0f1626;
    color: var(--text, #e9efff);
    border: 1px solid #2d3650;
    border-radius: 8px;
    padding: 8px;
  }
  .checklist {
    border: 1px solid #2d3650;
    border-radius: 10px;
    padding: 10px;
    display: grid;
    gap: 8px;
  }
  .checklist label {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .notes textarea {
    resize: vertical;
    min-height: 110px;
  }
  .helper-text {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.85rem;
  }
  .modal-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .history {
    border-top: 1px dashed #33405d;
    padding-top: 10px;
  }
  .history h3 {
    margin: 0 0 8px;
    font-size: 0.95rem;
  }
  .history pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
    font-size: 0.9rem;
    color: #b8c7e8;
  }
  @media (max-width: 800px) {
    .page-content {
      padding: 16px;
    }
    .stats-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .meta-grid,
    .form-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
