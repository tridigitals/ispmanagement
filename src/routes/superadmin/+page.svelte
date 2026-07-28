<script lang="ts">
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { systemHealthCache, type SystemHealth } from '$lib/stores/systemHealth';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';

  // ── State ──────────────────────────────────────────────────────────
  let loading = $state(true);
  let health = $state<SystemHealth | null>(null);

  let tenantTotal = $state(0);
  let tenantActive = $state(0);
  let userTotal = $state(0);
  let pendingApprovals = $state(0);
  let overdueInvoices = $state(0);
  let overdueInvoiceAmount = $state(0);
  let lastBackupAt = $state<string | null>(null);

  // ── Loaders ────────────────────────────────────────────────────────
  onMount(async () => {
    await loadDashboard();
  });

  async function loadDashboard() {
    loading = true;
    try {
      const [tenantsRes, usersRes, approvalsRes, invoicesRes, backupsRes, healthRes] =
        await Promise.all([
          api.superadmin.listTenants().catch(() => null),
          api.users.list(1, 1).catch(() => null),
          api.superadmin.listPendingApprovals().catch(() => null),
          api.payment.listAllInvoices().catch(() => null),
          api.backup.list({ scope: 'all' }).catch(() => null),
          api.superadmin.getSystemHealth().catch(() => null),
        ]);

      const tenants = tenantsRes?.data || [];
      tenantTotal = tenantsRes?.total ?? tenants.length;
      tenantActive = tenants.filter((x: any) => x.is_active).length;

      userTotal = usersRes?.total ?? 0;
      pendingApprovals = approvalsRes?.total ?? 0;

      const now = Date.now();
      const overdue = (invoicesRes || []).filter((inv: any) => {
        if (inv.status !== 'pending') return false;
        const due = new Date(inv.due_date || inv.created_at).getTime();
        return Number.isFinite(due) && due < now;
      });
      overdueInvoices = overdue.length;
      overdueInvoiceAmount = overdue.reduce(
        (sum: number, inv: any) => sum + (Number(inv.amount) || 0),
        0,
      );

      const latestBackup = (backupsRes || [])
        .map((b: any) => new Date(b.created_at || 0).getTime())
        .filter((ms) => Number.isFinite(ms) && ms > 0)
        .sort((a, b) => b - a)[0];
      lastBackupAt = latestBackup ? new Date(latestBackup).toISOString() : null;

      if (healthRes) {
        health = healthRes;
        systemHealthCache.set({ health: healthRes, fetchedAt: Date.now() });
      }
    } catch (e) {
      console.error('Failed to load superadmin dashboard', e);
    } finally {
      loading = false;
    }
  }

  // ── Derived ────────────────────────────────────────────────────────
  function backupAgeHours(): number | null {
    if (!lastBackupAt) return null;
    const ms = Date.now() - new Date(lastBackupAt).getTime();
    return Math.round(ms / 3_600_000);
  }

  let backupStale = $derived.by(() => {
    const h = backupAgeHours();
    return h === null || h > 24;
  });

  function formatIDR(n: number): string {
    return 'Rp ' + new Intl.NumberFormat('id-ID', { maximumFractionDigits: 0 }).format(n);
  }

  function formatRelative(iso: string): string {
    const diffH = Math.round((Date.now() - new Date(iso).getTime()) / 3_600_000);
    if (diffH < 1) return $t('superadmin.dashboard.time.just_now');
    if (diffH < 24)
      return $t('superadmin.dashboard.time.hours_ago', { values: { count: diffH } });
    const d = Math.round(diffH / 24);
    return $t('superadmin.dashboard.time.days_ago', { values: { count: d } });
  }

  let dbOk = $derived(health?.database?.is_connected ?? false);
  let dbLatency = $derived(
    health?.request_metrics ? Math.round(health.request_metrics.avg_response_time_ms) : null,
  );

  type AttentionItem = {
    key: string;
    icon: string;
    title: string;
    desc: string;
    action: string;
    href: string;
    tone: 'rose' | 'amber' | 'info';
  };

  let attentionItems = $derived.by<AttentionItem[]>(() => {
    const items: AttentionItem[] = [];
    if (pendingApprovals > 0) {
      items.push({
        key: 'approvals',
        icon: 'user-check',
        title: $t('superadmin.dashboard.attention.approvals_title', {
          values: { count: pendingApprovals },
        }),
        desc: $t('superadmin.dashboard.attention.approvals_desc'),
        action: $t('superadmin.dashboard.attention.review'),
        href: '/superadmin/registration-approvals',
        tone: 'info',
      });
    }
    if (overdueInvoices > 0) {
      items.push({
        key: 'invoices',
        icon: 'file-text',
        title: $t('superadmin.dashboard.attention.invoices_title', {
          values: { count: overdueInvoices },
        }),
        desc: $t('superadmin.dashboard.attention.invoices_desc', {
          values: { amount: formatIDR(overdueInvoiceAmount) },
        }),
        action: $t('superadmin.dashboard.attention.follow_up'),
        href: '/superadmin/invoices',
        tone: 'amber',
      });
    }
    if (backupStale) {
      items.push({
        key: 'backup',
        icon: 'archive',
        title: $t('superadmin.dashboard.attention.backup_title'),
        desc: lastBackupAt
          ? $t('superadmin.dashboard.attention.backup_desc_last', {
              values: { when: formatRelative(lastBackupAt) },
            })
          : $t('superadmin.dashboard.attention.backup_desc_none'),
        action: $t('superadmin.dashboard.attention.run'),
        href: '/superadmin/backups',
        tone: 'rose',
      });
    }
    return items;
  });
</script>

<div class="sa-dash fade-in">
  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>{$t('superadmin.dashboard.loading')}</p>
    </div>
  {:else}
    <!-- ── Page header (pola konsisten) ── -->
    <div class="page-head">
      <div>
        <div class="crumbs">
          {$t('superadmin.dashboard.crumbs.root')}
          <span class="crumb-sep">›</span>
          <b>{$t('superadmin.dashboard.crumbs.dashboard')}</b>
        </div>
        <h1>{$t('superadmin.dashboard.welcome')}</h1>
        <p class="subtitle">{$t('superadmin.dashboard.subtitle')}</p>
      </div>
      <div class="head-actions">
        <button class="btn ghost" onclick={loadDashboard}>
          <Icon name="refresh-cw" size={14} />
          {$t('superadmin.dashboard.actions.refresh')}
        </button>
        <button class="btn primary" onclick={() => goto('/superadmin/tenants')}>
          <Icon name="plus" size={14} />
          {$t('superadmin.dashboard.actions.new_tenant')}
        </button>
      </div>
    </div>

    <!-- ── Health strip ── -->
    <div class="health-strip">
      <div class="h-item">
        <div class="h-dot" class:ok={dbOk} class:warn={!dbOk}></div>
        <div>
          <b>{$t('superadmin.dashboard.health.database')}</b>
          <small>
            {#if dbOk}
              {dbLatency !== null
                ? $t('superadmin.dashboard.health.latency_ms', { values: { ms: dbLatency } })
                : $t('superadmin.dashboard.health.connected')}
            {:else}
              {$t('superadmin.dashboard.health.down')}
            {/if}
          </small>
        </div>
      </div>
      <div class="h-item">
        <div class="h-dot" class:ok={health !== null} class:warn={health === null}></div>
        <div>
          <b>{$t('superadmin.dashboard.health.sessions')}</b>
          <small>
            {#if health}
              {$t('superadmin.dashboard.health.sessions_value', {
                values: { count: health.active_sessions },
              })}
            {:else}
              {$t('common.loading')}
            {/if}
          </small>
        </div>
      </div>
      <div class="h-item">
        <div class="h-dot" class:ok={!backupStale} class:warn={backupStale}></div>
        <div>
          <b>{$t('superadmin.dashboard.health.backup')}</b>
          <small>
            {#if lastBackupAt}
              {formatRelative(lastBackupAt)}
            {:else}
              {$t('superadmin.dashboard.health.backup_never')}
            {/if}
          </small>
        </div>
      </div>
      <button class="h-item h-link" onclick={() => goto('/superadmin/system')}>
        <div class="h-dot ok"></div>
        <div>
          <b>{$t('superadmin.dashboard.health.details')}</b>
          <small>{$t('superadmin.dashboard.health.details_hint')}</small>
        </div>
        <Icon name="chevron-right" size={14} />
      </button>
    </div>

    <!-- ── Stat cards ── -->
    <div class="stats-grid" aria-label={$t('superadmin.dashboard.stats_aria')}>
      <button class="stat c-cyan" onclick={() => goto('/superadmin/tenants')}>
        <div class="stat-top">
          <div class="stat-ic"><Icon name="database" size={18} /></div>
        </div>
        <div class="stat-val">{tenantTotal}</div>
        <div class="stat-lbl">{$t('superadmin.dashboard.stats.tenants')}</div>
      </button>

      <button class="stat c-emerald" onclick={() => goto('/superadmin/tenants')}>
        <div class="stat-top">
          <div class="stat-ic"><Icon name="check-circle" size={18} /></div>
          <span class="trend up">
            {$t('superadmin.dashboard.stats.active_pct', {
              values: {
                pct: tenantTotal > 0 ? Math.round((tenantActive / tenantTotal) * 100) : 0,
              },
            })}
          </span>
        </div>
        <div class="stat-val">{tenantActive}</div>
        <div class="stat-lbl">{$t('superadmin.dashboard.stats.active_tenants')}</div>
      </button>

      <button class="stat c-indigo" onclick={() => goto('/superadmin/users')}>
        <div class="stat-top">
          <div class="stat-ic"><Icon name="users" size={18} /></div>
        </div>
        <div class="stat-val">{userTotal.toLocaleString('id-ID')}</div>
        <div class="stat-lbl">{$t('superadmin.dashboard.stats.users')}</div>
      </button>

      <button class="stat c-amber" onclick={() => goto('/superadmin/registration-approvals')}>
        <div class="stat-top">
          <div class="stat-ic"><Icon name="user-check" size={18} /></div>
          {#if pendingApprovals > 0}
            <span class="trend warn">
              {$t('superadmin.dashboard.stats.needs_review')}
            </span>
          {/if}
        </div>
        <div class="stat-val">{pendingApprovals}</div>
        <div class="stat-lbl">{$t('superadmin.dashboard.stats.pending_approvals')}</div>
      </button>
    </div>

    <!-- ── Perlu Perhatian ── -->
    {#if attentionItems.length > 0}
      <div class="panel">
        <div class="panel-head">
          <h2>{$t('superadmin.dashboard.attention.title')}</h2>
          <span class="count-badge">{attentionItems.length}</span>
        </div>
        <div class="attention-list">
          {#each attentionItems as item (item.key)}
            <div class="attention-row">
              <div class="attention-ic {item.tone}">
                <Icon name={item.icon} size={18} />
              </div>
              <div class="attention-body">
                <b>{item.title}</b>
                <small>{item.desc}</small>
              </div>
              <button class="btn small" onclick={() => goto(item.href)}>{item.action}</button>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- ── Quick actions ── -->
    <div class="section-header">
      <h2>{$t('superadmin.dashboard.quick_actions.title')}</h2>
    </div>

    <div class="actions-grid">
      <button class="action-card" onclick={() => goto('/superadmin/tenants')}>
        <div class="action-icon accent-cyan"><Icon name="database" size={18} /></div>
        <h3>{$t('superadmin.dashboard.quick_actions.tenants.title')}</h3>
        <p>{$t('superadmin.dashboard.quick_actions.tenants.desc')}</p>
      </button>

      <button class="action-card" onclick={() => goto('/superadmin/users')}>
        <div class="action-icon accent-indigo"><Icon name="users" size={18} /></div>
        <h3>{$t('superadmin.dashboard.quick_actions.users.title')}</h3>
        <p>{$t('superadmin.dashboard.quick_actions.users.desc')}</p>
      </button>

      <button class="action-card" onclick={() => goto('/superadmin/radius')}>
        <div class="action-icon accent-cyan"><Icon name="server" size={18} /></div>
        <h3>{$t('superadmin.radius.title')}</h3>
        <p>{$t('superadmin.radius.subtitle')}</p>
      </button>

      <button class="action-card" onclick={() => goto('/superadmin/audit-logs')}>
        <div class="action-icon accent-emerald"><Icon name="activity" size={18} /></div>
        <h3>{$t('superadmin.dashboard.quick_actions.audit.title')}</h3>
        <p>{$t('superadmin.dashboard.quick_actions.audit.desc')}</p>
      </button>

      <button class="action-card" onclick={() => goto('/superadmin/settings')}>
        <div class="action-icon accent-amber"><Icon name="settings" size={18} /></div>
        <h3>{$t('superadmin.dashboard.quick_actions.settings.title')}</h3>
        <p>{$t('superadmin.dashboard.quick_actions.settings.desc')}</p>
      </button>
    </div>
  {/if}
</div>

<style>
  .sa-dash {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1280px;
    margin: 0 auto;
    color: var(--text-primary);
    --accent-emerald: #10b981;
    --accent-cyan: #22d3ee;
    --accent-indigo: #6366f1;
    --accent-amber: #f59e0b;
    --accent-rose: #f43f5e;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2.5rem 1rem;
    gap: 0.75rem;
    color: var(--text-secondary);
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* ── Page header ── */
  .page-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 22px;
    flex-wrap: wrap;
  }

  .crumbs {
    font-size: 0.75rem;
    color: var(--text-secondary);
    opacity: 0.75;
    margin-bottom: 6px;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .crumbs b {
    font-weight: 500;
    opacity: 1;
  }

  .page-head h1 {
    font-size: 1.45rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.875rem;
    margin: 2px 0 0;
  }

  .head-actions {
    display: flex;
    gap: 10px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    transition: all 0.15s;
  }

  .btn:hover {
    border-color: var(--color-primary);
  }

  .btn.primary {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: #0b0d14;
  }

  .btn.primary:hover {
    filter: brightness(1.1);
  }

  .btn.ghost {
    background: transparent;
  }

  .btn.small {
    padding: 6px 14px;
    font-size: 0.78rem;
  }

  /* ── Health strip ── */
  .health-strip {
    display: flex;
    gap: 12px;
    margin-bottom: 24px;
    flex-wrap: wrap;
  }

  .h-item {
    flex: 1;
    min-width: 180px;
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 13px 16px;
    color: var(--text-primary);
    text-align: left;
  }

  button.h-item {
    cursor: pointer;
    transition: border-color 0.15s;
  }

  button.h-item:hover {
    border-color: var(--color-primary);
  }

  .h-link :global(svg) {
    margin-left: auto;
    color: var(--text-secondary);
  }

  .h-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
    background: var(--text-secondary);
  }

  .h-dot.ok {
    background: var(--accent-emerald);
    box-shadow: 0 0 0 4px rgba(16, 185, 129, 0.15);
  }

  .h-dot.warn {
    background: var(--accent-amber);
    box-shadow: 0 0 0 4px rgba(245, 158, 11, 0.15);
  }

  .h-item b {
    font-size: 0.82rem;
    display: block;
  }

  .h-item small {
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  /* ── Stat cards (aksen garis atas, bukan blok warna) ── */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 14px;
    margin-bottom: 26px;
  }

  .stat {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 18px;
    position: relative;
    overflow: hidden;
    cursor: pointer;
    transition: all 0.18s;
    color: var(--text-primary);
    text-align: left;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
  }

  .stat::before {
    content: '';
    position: absolute;
    inset: 0 0 auto;
    height: 3px;
  }

  .stat.c-cyan::before {
    background: var(--accent-cyan);
  }

  .stat.c-emerald::before {
    background: var(--accent-emerald);
  }

  .stat.c-indigo::before {
    background: var(--accent-indigo);
  }

  .stat.c-amber::before {
    background: var(--accent-amber);
  }

  .stat:hover {
    transform: translateY(-2px);
    border-color: var(--color-primary);
  }

  .stat-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    min-height: 38px;
  }

  .stat-ic {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .c-cyan .stat-ic {
    color: var(--accent-cyan);
  }

  .c-emerald .stat-ic {
    color: var(--accent-emerald);
  }

  .c-indigo .stat-ic {
    color: #a5b4fc;
  }

  .c-amber .stat-ic {
    color: var(--accent-amber);
  }

  .trend {
    font-size: 0.72rem;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 99px;
  }

  .trend.up {
    background: rgba(16, 185, 129, 0.12);
    color: var(--accent-emerald);
  }

  .trend.warn {
    background: rgba(245, 158, 11, 0.12);
    color: var(--accent-amber);
  }

  .stat-val {
    font-size: 1.7rem;
    font-weight: 800;
    letter-spacing: -0.03em;
  }

  .stat-lbl {
    color: var(--text-secondary);
    font-size: 0.8rem;
    margin-top: 2px;
  }

  /* ── Panel Perlu Perhatian ── */
  .panel {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
    margin-bottom: 24px;
  }

  .panel-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .panel-head h2 {
    font-size: 1rem;
    font-weight: 700;
    margin: 0;
  }

  .count-badge {
    background: rgba(245, 158, 11, 0.14);
    color: var(--accent-amber);
    font-size: 0.72rem;
    font-weight: 700;
    padding: 2px 9px;
    border-radius: 99px;
  }

  .attention-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 13px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .attention-row:last-child {
    border-bottom: none;
  }

  .attention-ic {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }

  .attention-ic.info {
    background: rgba(34, 211, 238, 0.1);
    color: var(--accent-cyan);
  }

  .attention-ic.amber {
    background: rgba(245, 158, 11, 0.1);
    color: var(--accent-amber);
  }

  .attention-ic.rose {
    background: rgba(244, 63, 94, 0.1);
    color: var(--accent-rose);
  }

  .attention-body {
    flex: 1;
    min-width: 0;
  }

  .attention-body b {
    font-size: 0.875rem;
    display: block;
  }

  .attention-body small {
    color: var(--text-secondary);
    font-size: 0.78rem;
  }

  /* ── Quick actions ── */
  .section-header {
    margin: 0 0 1rem 0;
  }

  .section-header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 750;
    letter-spacing: -0.01em;
  }

  .actions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1rem;
  }

  .action-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s;
    color: var(--text-primary);
  }

  .action-card:hover {
    transform: translateY(-2px);
    border-color: var(--color-primary);
  }

  .action-icon {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.08);
    margin-bottom: 0.85rem;
    background: rgba(255, 255, 255, 0.03);
  }

  .accent-emerald {
    color: var(--accent-emerald);
  }

  .accent-cyan {
    color: var(--accent-cyan);
  }

  .accent-indigo {
    color: #a5b4fc;
  }

  .accent-amber {
    color: var(--accent-amber);
  }

  .action-card h3 {
    margin: 0 0 0.35rem 0;
    font-size: 1rem;
    font-weight: 750;
  }

  .action-card p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.92rem;
    line-height: 1.35;
  }

  @media (max-width: 768px) {
    .stats-grid {
      grid-template-columns: 1fr 1fr;
      gap: 0.75rem;
    }

    .actions-grid {
      grid-template-columns: 1fr;
    }

    .page-head {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
