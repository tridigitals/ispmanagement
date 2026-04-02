<script lang="ts">
  import { api } from '$lib/api/client';
  import type { SuperadminManagedRadiusServer, SuperadminManagedRadiusUser } from '$lib/api/types';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import { t } from 'svelte-i18n';
  import { onMount } from 'svelte';

  let servers = $state<SuperadminManagedRadiusServer[]>([]);
  let users = $state<SuperadminManagedRadiusUser[]>([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state('');

  let serverSearch = $state('');
  let serverStatusFilter = $state<'all' | 'active' | 'inactive'>('all');

  let userSearch = $state('');
  let tenantFilter = $state('all');
  let routerFilter = $state('all');
  let userStatusFilter = $state<'all' | 'provisioned' | 'not_provisioned'>('all');

  onMount(() => {
    void loadData();
  });

  async function loadData(opts: { silent?: boolean } = {}) {
    if (opts.silent) refreshing = true;
    else loading = true;

    error = '';
    try {
      const [serverRes, userRes] = await Promise.all([
        api.superadmin.listManagedRadiusServers(),
        api.superadmin.listManagedRadiusUsers(),
      ]);

      servers = serverRes.data || [];
      users = userRes.data || [];
    } catch (err: any) {
      console.error('Failed to load superadmin managed RADIUS data', err);
      error = err?.message || String(err);
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  function normalized(value: string | null | undefined) {
    return String(value || '')
      .trim()
      .toLowerCase();
  }

  function formatDateTime(value: string | null | undefined) {
    if (!value) return $t('superadmin.radius.labels.never') || 'Never';
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(date);
  }

  function userStatus(user: SuperadminManagedRadiusUser) {
    if (user.radius_present) return 'provisioned';
    return 'not_provisioned';
  }

  function userBadgeTone(user: SuperadminManagedRadiusUser) {
    if (user.radius_present) return 'good';
    if (user.radius_last_error) return 'danger';
    return 'warn';
  }

  const tenantOptions = $derived.by(() => {
    const names = [...new Set(users.map((user) => user.tenant_name).filter(Boolean))];
    return names.sort((a, b) => a.localeCompare(b));
  });

  const routerOptions = $derived.by(() => {
    const names = [
      ...new Set(
        users.map((user) => user.router_name || $t('superadmin.radius.labels.unknown_router') || 'Unknown router'),
      ),
    ];
    return names.sort((a, b) => a.localeCompare(b));
  });

  const filteredServers = $derived.by(() =>
    servers.filter((server) => {
      const q = normalized(serverSearch);
      const matchesSearch =
        !q ||
        normalized(server.name).includes(q) ||
        normalized(server.tenant_name).includes(q) ||
        normalized(server.host).includes(q) ||
        normalized(server.db_host).includes(q);

      const matchesStatus =
        serverStatusFilter === 'all' ||
        (serverStatusFilter === 'active' ? server.is_active : !server.is_active);

      return matchesSearch && matchesStatus;
    }),
  );

  const filteredUsers = $derived.by(() =>
    users.filter((user) => {
      const q = normalized(userSearch);
      const routerName =
        user.router_name || ($t('superadmin.radius.labels.unknown_router') || 'Unknown router');
      const matchesSearch =
        !q ||
        normalized(user.username).includes(q) ||
        normalized(user.radius_identity).includes(q) ||
        normalized(user.tenant_name).includes(q) ||
        normalized(routerName).includes(q);

      const matchesTenant = tenantFilter === 'all' || user.tenant_name === tenantFilter;
      const matchesRouter = routerFilter === 'all' || routerName === routerFilter;
      const matchesStatus = userStatusFilter === 'all' || userStatus(user) === userStatusFilter;

      return matchesSearch && matchesTenant && matchesRouter && matchesStatus;
    }),
  );

  const stats = $derived.by(() => ({
    servers: servers.length,
    routers: servers.reduce((sum, server) => sum + server.router_count, 0),
    users: users.length,
    outOfSync: users.filter((user) => !user.radius_present || user.radius_last_error).length,
  }));
</script>

<div class="page-shell">
  <div class="hero">
    <div>
      <h1>{$t('superadmin.radius.title') || 'Managed RADIUS'}</h1>
      <p>
        {$t('superadmin.radius.subtitle') ||
          'Observe global RADIUS infrastructure and provisioned PPPoE users across tenants.'}
      </p>
    </div>
    <button class="refresh-btn" onclick={() => loadData({ silent: true })} disabled={refreshing}>
      {#if refreshing}
        {$t('common.loading') || 'Loading...'}
      {:else}
        {$t('superadmin.radius.refresh') || 'Refresh'}
      {/if}
    </button>
  </div>

  {#if loading}
    <div class="state-card">{$t('superadmin.radius.loading') || 'Loading managed RADIUS observability...'}</div>
  {:else if error}
    <div class="state-card error">{error}</div>
  {:else}
    <div class="stats-grid">
      <StatsCard title={$t('superadmin.radius.stats.servers') || 'Servers'} value={stats.servers} icon="server" />
      <StatsCard title={$t('superadmin.radius.stats.routers') || 'Mapped Routers'} value={stats.routers} icon="database" color="success" />
      <StatsCard title={$t('superadmin.radius.stats.users') || 'Users'} value={stats.users} icon="users" />
      <StatsCard title={$t('superadmin.radius.stats.out_of_sync') || 'Needs Attention'} value={stats.outOfSync} icon="activity" color="warning" />
    </div>

    <section class="panel">
      <div class="panel-head">
        <div>
          <h2>{$t('superadmin.radius.sections.servers') || 'Servers'}</h2>
          <p>{filteredServers.length} / {servers.length}</p>
        </div>
        <div class="filters">
          <input bind:value={serverSearch} placeholder={$t('superadmin.radius.filters.search_servers') || 'Search servers...'} />
          <select bind:value={serverStatusFilter}>
            <option value="all">{$t('superadmin.radius.filters.all_statuses') || 'All statuses'}</option>
            <option value="active">{$t('superadmin.radius.filters.active') || 'Active'}</option>
            <option value="inactive">{$t('superadmin.radius.filters.inactive') || 'Inactive'}</option>
          </select>
        </div>
      </div>

      {#if filteredServers.length === 0}
        <div class="empty-state">
          <strong>{$t('superadmin.radius.empty.servers_title') || 'No managed RADIUS servers yet'}</strong>
          <span>{$t('superadmin.radius.empty.servers_subtitle') || 'Managed RADIUS infrastructure will appear here after tenants configure servers and NAS mappings.'}</span>
        </div>
      {:else}
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{$t('superadmin.radius.columns.server') || 'Server'}</th>
                <th>{$t('superadmin.radius.columns.tenant') || 'Tenant'}</th>
                <th>{$t('superadmin.radius.columns.host') || 'Host'}</th>
                <th>{$t('superadmin.radius.columns.ports') || 'Ports'}</th>
                <th>{$t('superadmin.radius.columns.database') || 'Database'}</th>
                <th>{$t('superadmin.radius.columns.status') || 'Status'}</th>
                <th>{$t('superadmin.radius.columns.routers') || 'Routers'}</th>
                <th>{$t('superadmin.radius.columns.updated') || 'Updated'}</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredServers as server}
                <tr>
                  <td>
                    <div class="primary">{server.name}</div>
                  </td>
                  <td>{server.tenant_name}</td>
                  <td>
                    <div class="primary">{server.host}</div>
                  </td>
                  <td>{server.auth_port}/{server.acct_port}</td>
                  <td>{server.db_host}:{server.db_port}/{server.db_name}</td>
                  <td>
                    <span class="badge" class:good={server.is_active} class:muted={!server.is_active}>
                      {server.is_active
                        ? $t('superadmin.radius.status.active') || 'Active'
                        : $t('superadmin.radius.status.inactive') || 'Inactive'}
                    </span>
                  </td>
                  <td>{server.router_count}</td>
                  <td>{formatDateTime(server.updated_at)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    <section class="panel">
      <div class="panel-head">
        <div>
          <h2>{$t('superadmin.radius.sections.users') || 'Users'}</h2>
          <p>{filteredUsers.length} / {users.length}</p>
        </div>
        <div class="filters filters-wide">
          <input bind:value={userSearch} placeholder={$t('superadmin.radius.filters.search_users') || 'Search users...'} />
          <select bind:value={tenantFilter}>
            <option value="all">{$t('superadmin.radius.filters.all_tenants') || 'All tenants'}</option>
            {#each tenantOptions as tenantName}
              <option value={tenantName}>{tenantName}</option>
            {/each}
          </select>
          <select bind:value={routerFilter}>
            <option value="all">{$t('superadmin.radius.filters.all_routers') || 'All routers'}</option>
            {#each routerOptions as routerName}
              <option value={routerName}>{routerName}</option>
            {/each}
          </select>
          <select bind:value={userStatusFilter}>
            <option value="all">{$t('superadmin.radius.filters.all_users') || 'All users'}</option>
            <option value="provisioned">{$t('superadmin.radius.filters.provisioned') || 'Provisioned'}</option>
            <option value="not_provisioned">{$t('superadmin.radius.filters.not_provisioned') || 'Not provisioned'}</option>
          </select>
        </div>
      </div>

      {#if filteredUsers.length === 0}
        <div class="empty-state">
          <strong>{$t('superadmin.radius.empty.users_title') || 'No managed RADIUS users yet'}</strong>
          <span>{$t('superadmin.radius.empty.users_subtitle') || 'Managed-RADIUS-backed PPPoE users will appear here after tenant admins apply them.'}</span>
        </div>
      {:else}
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{$t('superadmin.radius.columns.tenant') || 'Tenant'}</th>
                <th>{$t('superadmin.radius.columns.router') || 'Router'}</th>
                <th>{$t('superadmin.radius.columns.username') || 'Username'}</th>
                <th>{$t('superadmin.radius.columns.identity') || 'RADIUS Identity'}</th>
                <th>{$t('superadmin.radius.columns.profile') || 'Profile'}</th>
                <th>{$t('superadmin.radius.columns.status') || 'Status'}</th>
                <th>{$t('superadmin.radius.columns.last_sync') || 'Last Sync'}</th>
                <th>{$t('superadmin.radius.columns.last_error') || 'Last Error'}</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredUsers as user}
                <tr>
                  <td>{user.tenant_name}</td>
                  <td>{user.router_name || ($t('superadmin.radius.labels.unknown_router') || 'Unknown router')}</td>
                  <td><div class="primary">{user.username}</div></td>
                  <td>{user.radius_identity || user.username}</td>
                  <td>{user.router_profile_name || ($t('superadmin.radius.labels.none') || 'None')}</td>
                  <td>
                    <span class="badge" class:good={userBadgeTone(user) === 'good'} class:warn={userBadgeTone(user) === 'warn'} class:danger={userBadgeTone(user) === 'danger'}>
                      {#if user.radius_present}
                        {$t('superadmin.radius.status.provisioned') || 'Provisioned'}
                      {:else if user.radius_last_error}
                        {$t('superadmin.radius.status.needs_attention') || 'Needs attention'}
                      {:else}
                        {$t('superadmin.radius.status.not_provisioned') || 'Not provisioned'}
                      {/if}
                    </span>
                  </td>
                  <td>{formatDateTime(user.radius_last_sync_at)}</td>
                  <td class="error-text">{user.radius_last_error || ($t('superadmin.radius.labels.none') || 'None')}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .page-shell {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1500px;
    margin: 0 auto;
    display: grid;
    gap: 1.5rem;
  }

  .hero {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .hero h1 {
    margin: 0 0 0.35rem;
    font-size: clamp(1.5rem, 2.5vw, 2rem);
  }

  .hero p {
    margin: 0;
    color: var(--text-secondary);
    max-width: 760px;
  }

  .refresh-btn,
  .filters input,
  .filters select {
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-primary);
    color: var(--text-primary);
    min-height: 42px;
  }

  .refresh-btn {
    padding: 0 1rem;
    cursor: pointer;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }

  .panel,
  .state-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    padding: 1rem;
    box-shadow: var(--shadow-sm);
  }

  .state-card.error {
    color: var(--color-danger, #dc2626);
  }

  .panel-head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .panel-head h2 {
    margin: 0 0 0.25rem;
  }

  .panel-head p {
    margin: 0;
    color: var(--text-secondary);
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  .filters input,
  .filters select {
    padding: 0 0.85rem;
  }

  .filters-wide input {
    min-width: 220px;
  }

  .table-wrap {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    min-width: 960px;
  }

  th,
  td {
    text-align: left;
    padding: 0.9rem 0.75rem;
    border-bottom: 1px solid var(--border-color);
    vertical-align: top;
  }

  th {
    color: var(--text-secondary);
    font-weight: 600;
    font-size: 0.9rem;
  }

  .primary {
    font-weight: 600;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    padding: 0.3rem 0.65rem;
    border-radius: 999px;
    font-size: 0.82rem;
    font-weight: 600;
  }

  .good {
    background: rgba(16, 185, 129, 0.14);
    color: #059669;
  }

  .warn {
    background: rgba(245, 158, 11, 0.14);
    color: #d97706;
  }

  .danger {
    background: rgba(239, 68, 68, 0.14);
    color: #dc2626;
  }

  .muted {
    background: rgba(148, 163, 184, 0.14);
    color: var(--text-secondary);
  }

  .empty-state {
    display: grid;
    gap: 0.35rem;
    padding: 0.5rem 0;
    color: var(--text-secondary);
  }

  .error-text {
    max-width: 320px;
    white-space: normal;
    word-break: break-word;
  }

  @media (max-width: 900px) {
    .hero,
    .panel-head {
      flex-direction: column;
    }

    .filters,
    .filters-wide {
      width: 100%;
      justify-content: stretch;
    }

    .filters input,
    .filters select,
    .refresh-btn {
      width: 100%;
    }
  }
</style>
