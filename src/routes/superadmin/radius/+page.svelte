<script lang="ts">
  import { api } from '$lib/api/client';
  import type {
    ManagedRadiusMappingPayload,
    ManagedRadiusSecretValue,
    ManagedRadiusServerPayload,
    SuperadminManagedRadiusMapping,
    SuperadminManagedRadiusServer,
    SuperadminManagedRadiusUser,
    Tenant,
  } from '$lib/api/types';
  import MappingFormModal from '$lib/components/superadmin/radius/MappingFormModal.svelte';
  import MappingSecretDialog from '$lib/components/superadmin/radius/MappingSecretDialog.svelte';
  import ServerFormModal from '$lib/components/superadmin/radius/ServerFormModal.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import { toast } from '$lib/stores/toast';
  import {
    buildManagedRadiusRouterOsCli,
    filterManagedRadiusMappings,
  } from '$lib/utils/managedRadiusControlPlane';
  import { t } from 'svelte-i18n';
  import { onMount } from 'svelte';

  type RouterOption = {
    id: string;
    tenant_id?: string | null;
    name?: string | null;
    host?: string | null;
  };

  const DEFAULT_SERVER_FORM = (): ManagedRadiusServerPayload => ({
    tenant_id: '',
    name: '',
    db_host: '',
    db_port: 5432,
    db_name: 'radius',
    db_user: 'radius',
    db_password: '',
    is_active: true,
  });

  const DEFAULT_MAPPING_FORM = (): ManagedRadiusMappingPayload => ({
    tenant_id: '',
    radius_server_id: '',
    router_id: '',
    nas_name: '',
    nas_ip_or_cidr: '',
    shortname: '',
    shared_secret: '',
    is_active: true,
  });

  let tenants = $state<Tenant[]>([]);
  let routers = $state<RouterOption[]>([]);
  let servers = $state<SuperadminManagedRadiusServer[]>([]);
  let mappings = $state<SuperadminManagedRadiusMapping[]>([]);
  let users = $state<SuperadminManagedRadiusUser[]>([]);

  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state('');

  let serverSearch = $state('');
  let serverStatusFilter = $state<'all' | 'active' | 'inactive'>('all');

  let mappingSearch = $state('');
  let mappingTenantFilter = $state('all');
  let mappingServerFilter = $state('all');
  let mappingStatusFilter = $state<'all' | 'active' | 'inactive'>('all');

  let userSearch = $state('');
  let tenantFilter = $state('all');
  let routerFilter = $state('all');
  let userStatusFilter = $state<'all' | 'provisioned' | 'not_provisioned'>('all');

  let showServerModal = $state(false);
  let savingServer = $state(false);
  let editingServerId = $state<string | null>(null);
  let serverForm = $state<ManagedRadiusServerPayload>(DEFAULT_SERVER_FORM());

  let showMappingModal = $state(false);
  let savingMapping = $state(false);
  let editingMappingId = $state<string | null>(null);
  let mappingForm = $state<ManagedRadiusMappingPayload>(DEFAULT_MAPPING_FORM());

  let showSecretDialog = $state(false);
  let secretDialogMode = $state<'reveal' | 'rotate'>('reveal');
  let secretDialogLoading = $state(false);
  let secretDialogDraft = $state('');
  let secretDialogRevealed = $state('');
  let secretDialogMapping = $state<SuperadminManagedRadiusMapping | null>(null);
  let mappingSecretCache = $state<Record<string, ManagedRadiusSecretValue>>({});

  onMount(() => {
    void loadData();
  });

  async function loadData(opts: { silent?: boolean } = {}) {
    if (opts.silent) refreshing = true;
    else loading = true;

    error = '';
    try {
      const [tenantRes, serverRes, mappingRes, userRes, routerRes] = await Promise.all([
        api.superadmin.listTenants(),
        api.superadmin.listManagedRadiusServers(),
        api.superadmin.listManagedRadiusMappings(),
        api.superadmin.listManagedRadiusUsers(),
        api.mikrotik.routers.list().catch(() => []),
      ]);

      tenants = tenantRes.data || [];
      servers = serverRes.data || [];
      mappings = mappingRes.data || [];
      users = userRes.data || [];
      routers = (routerRes || []) as RouterOption[];
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

  function generateSecret(length = 32) {
    const alphabet = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789';
    if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
      const bytes = new Uint8Array(length);
      crypto.getRandomValues(bytes);
      return Array.from(bytes, (byte) => alphabet[byte % alphabet.length]).join('');
    }

    return Array.from({ length }, () => alphabet[Math.floor(Math.random() * alphabet.length)]).join(
      '',
    );
  }

  function resetServerForm() {
    serverForm = DEFAULT_SERVER_FORM();
    editingServerId = null;
  }

  function resetMappingForm() {
    mappingForm = DEFAULT_MAPPING_FORM();
    editingMappingId = null;
  }

  function openCreateServerModal() {
    resetServerForm();
    showServerModal = true;
  }

  function openEditServerModal(server: SuperadminManagedRadiusServer) {
    editingServerId = server.id;
    serverForm = {
      tenant_id: server.tenant_id,
      name: server.name,
      db_host: server.db_host,
      db_port: server.db_port,
      db_name: server.db_name,
      db_user: '',
      db_password: '',
      is_active: server.is_active,
    };
    showServerModal = true;
  }

  async function submitServerForm() {
    if (!serverForm.tenant_id || !serverForm.name || !serverForm.db_host || !serverForm.db_name) {
      toast.error($t('superadmin.radius.toasts.server_validation') || 'Complete the server form first');
      return;
    }

    savingServer = true;
    try {
      const payload: ManagedRadiusServerPayload = {
        ...serverForm,
        db_password: serverForm.db_password?.trim() ? serverForm.db_password : null,
        db_user: serverForm.db_user?.trim() || 'radius',
        db_port: Number(serverForm.db_port) || 5432,
      };

      if (editingServerId) {
        await api.superadmin.updateManagedRadiusServer(editingServerId, payload);
        toast.success($t('superadmin.radius.toasts.server_updated') || 'Managed RADIUS server updated');
      } else {
        if (!payload.db_password) {
          toast.error(
            $t('superadmin.radius.toasts.server_password_required') || 'Database password is required for new servers',
          );
          return;
        }
        await api.superadmin.createManagedRadiusServer(payload);
        toast.success($t('superadmin.radius.toasts.server_created') || 'Managed RADIUS server created');
      }

      showServerModal = false;
      resetServerForm();
      await loadData({ silent: true });
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to save server');
    } finally {
      savingServer = false;
    }
  }

  async function toggleServerActive(server: SuperadminManagedRadiusServer) {
    try {
      await api.superadmin.setManagedRadiusServerActive(server.id, server.tenant_id, !server.is_active);
      toast.success(
        !server.is_active
          ? $t('superadmin.radius.toasts.server_activated') || 'Server activated'
          : $t('superadmin.radius.toasts.server_deactivated') || 'Server deactivated',
      );
      await loadData({ silent: true });
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to change server state');
    }
  }

  function openCreateMappingModal() {
    resetMappingForm();
    showMappingModal = true;
  }

  function openEditMappingModal(mapping: SuperadminManagedRadiusMapping) {
    editingMappingId = mapping.id;
    mappingForm = {
      tenant_id: mapping.tenant_id,
      radius_server_id: mapping.radius_server_id,
      router_id: mapping.router_id,
      nas_name: mapping.nas_name,
      nas_ip_or_cidr: mapping.nas_ip_or_cidr,
      shortname: mapping.shortname || '',
      shared_secret: '',
      is_active: mapping.is_active,
    };
    showMappingModal = true;
  }

  async function submitMappingForm() {
    if (
      !mappingForm.tenant_id ||
      !mappingForm.radius_server_id ||
      !mappingForm.router_id ||
      !mappingForm.nas_name ||
      !mappingForm.nas_ip_or_cidr
    ) {
      toast.error($t('superadmin.radius.toasts.mapping_validation') || 'Complete the mapping form first');
      return;
    }

    savingMapping = true;
    try {
      const payload: ManagedRadiusMappingPayload = {
        ...mappingForm,
        shortname: mappingForm.shortname?.trim() || null,
        shared_secret: mappingForm.shared_secret?.trim() || null,
      };

      if (editingMappingId) {
        await api.superadmin.updateManagedRadiusMapping(editingMappingId, payload);
        toast.success($t('superadmin.radius.toasts.mapping_updated') || 'NAS mapping updated');
      } else {
        await api.superadmin.createManagedRadiusMapping(payload);
        toast.success($t('superadmin.radius.toasts.mapping_created') || 'NAS mapping created');
      }

      showMappingModal = false;
      resetMappingForm();
      await loadData({ silent: true });
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to save mapping');
    } finally {
      savingMapping = false;
    }
  }

  async function toggleMappingActive(mapping: SuperadminManagedRadiusMapping) {
    try {
      await api.superadmin.setManagedRadiusMappingActive(
        mapping.id,
        mapping.tenant_id,
        !mapping.is_active,
      );
      toast.success(
        !mapping.is_active
          ? $t('superadmin.radius.toasts.mapping_activated') || 'Mapping activated'
          : $t('superadmin.radius.toasts.mapping_deactivated') || 'Mapping deactivated',
      );
      await loadData({ silent: true });
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to change mapping state');
    }
  }

  function openSecretDialog(mapping: SuperadminManagedRadiusMapping, mode: 'reveal' | 'rotate') {
    secretDialogMode = mode;
    secretDialogMapping = mapping;
    secretDialogDraft = '';
    secretDialogRevealed = mappingSecretCache[mapping.id]?.shared_secret || '';
    showSecretDialog = true;
  }

  function generateSecretForMappingForm() {
    mappingForm.shared_secret = generateSecret();
  }

  function generateSecretForDialog() {
    secretDialogDraft = generateSecret();
  }

  async function submitSecretDialog() {
    if (!secretDialogMapping) return;

    secretDialogLoading = true;
    try {
      let result: ManagedRadiusSecretValue;

      if (secretDialogMode === 'rotate') {
        result = await api.superadmin.rotateManagedRadiusMappingSecret(
          secretDialogMapping.id,
          secretDialogMapping.tenant_id,
          secretDialogDraft || null,
        );
        toast.success($t('superadmin.radius.toasts.secret_rotated') || 'Shared secret rotated');
      } else {
        result = await api.superadmin.revealManagedRadiusMappingSecret(
          secretDialogMapping.id,
          secretDialogMapping.tenant_id,
        );
        toast.success($t('superadmin.radius.toasts.secret_revealed') || 'Shared secret loaded');
      }

      mappingSecretCache = {
        ...mappingSecretCache,
        [secretDialogMapping.id]: result,
      };
      secretDialogRevealed = result.shared_secret;

      if (secretDialogMode === 'rotate') {
        await loadData({ silent: true });
      }
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to handle secret');
    } finally {
      secretDialogLoading = false;
    }
  }

  async function copyText(value: string, successMessage: string) {
    try {
      await navigator.clipboard.writeText(value);
      toast.success(successMessage);
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to copy text');
    }
  }

  async function copyMappingCli(mapping: SuperadminManagedRadiusMapping) {
    try {
      let secret = mappingSecretCache[mapping.id]?.shared_secret;
      if (!secret) {
        const response = await api.superadmin.revealManagedRadiusMappingSecret(
          mapping.id,
          mapping.tenant_id,
        );
        mappingSecretCache = {
          ...mappingSecretCache,
          [mapping.id]: response,
        };
        secret = response.shared_secret;
      }

      const cli = buildManagedRadiusRouterOsCli(
        mapping.radius_host,
        secret,
        mapping.auth_port,
        mapping.acct_port,
      );
      await copyText(cli, $t('superadmin.radius.toasts.cli_copied') || 'RouterOS CLI copied');
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to copy RouterOS CLI');
    }
  }

  function focusMappingsForServer(server: SuperadminManagedRadiusServer) {
    mappingTenantFilter = server.tenant_id;
    mappingServerFilter = server.id;
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

  const mappingTenantOptions = $derived.by(() =>
    tenants
      .slice()
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((tenant) => ({ id: tenant.id, name: tenant.name })),
  );

  const mappingServerOptions = $derived.by(() =>
    servers
      .filter((server) => mappingTenantFilter === 'all' || server.tenant_id === mappingTenantFilter)
      .map((server) => ({ id: server.id, name: `${server.name} (${server.tenant_name})` })),
  );

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

  const filteredMappings = $derived.by(() => {
    const base = filterManagedRadiusMappings(mappings, {
      tenantId: mappingTenantFilter === 'all' ? '' : mappingTenantFilter,
      serverId: mappingServerFilter === 'all' ? '' : mappingServerFilter,
      search: mappingSearch,
    });

    return base.filter((mapping) => {
      if (mappingStatusFilter === 'all') return true;
      return mappingStatusFilter === 'active' ? mapping.is_active : !mapping.is_active;
    });
  });

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
    mappings: mappings.length,
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

    <div class="hero-actions">
      <button class="btn btn-secondary" type="button" onclick={openCreateServerModal}>
        {$t('superadmin.radius.actions.new_server') || 'New server'}
      </button>
      <button class="btn btn-primary" type="button" onclick={openCreateMappingModal}>
        {$t('superadmin.radius.actions.new_mapping') || 'New mapping'}
      </button>
      <button class="refresh-btn" onclick={() => loadData({ silent: true })} disabled={refreshing}>
        {#if refreshing}
          {$t('common.loading') || 'Loading...'}
        {:else}
          {$t('superadmin.radius.refresh') || 'Refresh'}
        {/if}
      </button>
    </div>
  </div>

  {#if loading}
    <div class="state-card">{$t('superadmin.radius.loading') || 'Loading managed RADIUS observability...'}</div>
  {:else if error}
    <div class="state-card error">{error}</div>
  {:else}
    <div class="stats-grid">
      <StatsCard title={$t('superadmin.radius.stats.servers') || 'Servers'} value={stats.servers} icon="server" />
      <StatsCard title={$t('superadmin.radius.stats.mappings') || 'NAS Mappings'} value={stats.mappings} icon="network" color="success" />
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
                <th>{$t('superadmin.radius.columns.database') || 'Database'}</th>
                <th>{$t('superadmin.radius.columns.status') || 'Status'}</th>
                <th>{$t('superadmin.radius.columns.routers') || 'Routers'}</th>
                <th>{$t('superadmin.radius.columns.updated') || 'Updated'}</th>
                <th>{$t('superadmin.radius.columns.actions') || 'Actions'}</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredServers as server}
                <tr>
                  <td><div class="primary">{server.name}</div></td>
                  <td>{server.tenant_name}</td>
                  <td>{server.host}</td>
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
                  <td>
                    <div class="row-actions">
                      <button class="btn-link" type="button" onclick={() => openEditServerModal(server)}>
                        {$t('superadmin.radius.actions.edit') || 'Edit'}
                      </button>
                      <button class="btn-link" type="button" onclick={() => toggleServerActive(server)}>
                        {server.is_active
                          ? $t('superadmin.radius.actions.deactivate') || 'Deactivate'
                          : $t('superadmin.radius.actions.activate') || 'Activate'}
                      </button>
                      <button class="btn-link" type="button" onclick={() => focusMappingsForServer(server)}>
                        {$t('superadmin.radius.actions.view_mappings') || 'View mappings'}
                      </button>
                    </div>
                  </td>
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
          <h2>{$t('superadmin.radius.sections.mappings') || 'NAS Mappings'}</h2>
          <p>{filteredMappings.length} / {mappings.length}</p>
        </div>
        <div class="filters filters-wide">
          <input bind:value={mappingSearch} placeholder={$t('superadmin.radius.filters.search_mappings') || 'Search mappings...'} />
          <select bind:value={mappingTenantFilter}>
            <option value="all">{$t('superadmin.radius.filters.all_tenants') || 'All tenants'}</option>
            {#each mappingTenantOptions as tenant}
              <option value={tenant.id}>{tenant.name}</option>
            {/each}
          </select>
          <select bind:value={mappingServerFilter}>
            <option value="all">{$t('superadmin.radius.filters.all_servers') || 'All servers'}</option>
            {#each mappingServerOptions as server}
              <option value={server.id}>{server.name}</option>
            {/each}
          </select>
          <select bind:value={mappingStatusFilter}>
            <option value="all">{$t('superadmin.radius.filters.all_statuses') || 'All statuses'}</option>
            <option value="active">{$t('superadmin.radius.filters.active') || 'Active'}</option>
            <option value="inactive">{$t('superadmin.radius.filters.inactive') || 'Inactive'}</option>
          </select>
        </div>
      </div>

      {#if filteredMappings.length === 0}
        <div class="empty-state">
          <strong>{$t('superadmin.radius.empty.mappings_title') || 'No NAS mappings yet'}</strong>
          <span>{$t('superadmin.radius.empty.mappings_subtitle') || 'Create a tenant-scoped server and map a router to start issuing copy-ready MikroTik CLI.'}</span>
        </div>
      {:else}
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{$t('superadmin.radius.columns.tenant') || 'Tenant'}</th>
                <th>{$t('superadmin.radius.columns.server') || 'Server'}</th>
                <th>{$t('superadmin.radius.columns.router') || 'Router'}</th>
                <th>{$t('superadmin.radius.columns.nas') || 'NAS'}</th>
                <th>{$t('superadmin.radius.columns.secret') || 'Secret'}</th>
                <th>{$t('superadmin.radius.columns.status') || 'Status'}</th>
                <th>{$t('superadmin.radius.columns.updated') || 'Updated'}</th>
                <th>{$t('superadmin.radius.columns.actions') || 'Actions'}</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredMappings as mapping}
                <tr>
                  <td>{mapping.tenant_name}</td>
                  <td>
                    <div class="primary">{mapping.server_name}</div>
                    <div class="muted">{mapping.radius_host}:{mapping.auth_port}/{mapping.acct_port}</div>
                  </td>
                  <td>{mapping.router_name || ($t('superadmin.radius.labels.unknown_router') || 'Unknown router')}</td>
                  <td>
                    <div class="primary">{mapping.nas_name}</div>
                    <div class="muted">{mapping.nas_ip_or_cidr}</div>
                    {#if mapping.shortname}
                      <div class="muted">{mapping.shortname}</div>
                    {/if}
                  </td>
                  <td><code>{mapping.shared_secret_masked}</code></td>
                  <td>
                    <span class="badge" class:good={mapping.is_active} class:muted={!mapping.is_active}>
                      {mapping.is_active
                        ? $t('superadmin.radius.status.active') || 'Active'
                        : $t('superadmin.radius.status.inactive') || 'Inactive'}
                    </span>
                  </td>
                  <td>{formatDateTime(mapping.updated_at)}</td>
                  <td>
                    <div class="row-actions wrap">
                      <button class="btn-link" type="button" onclick={() => openEditMappingModal(mapping)}>
                        {$t('superadmin.radius.actions.edit') || 'Edit'}
                      </button>
                      <button class="btn-link" type="button" onclick={() => openSecretDialog(mapping, 'reveal')}>
                        {$t('superadmin.radius.actions.reveal_secret') || 'Reveal secret'}
                      </button>
                      <button class="btn-link" type="button" onclick={() => openSecretDialog(mapping, 'rotate')}>
                        {$t('superadmin.radius.actions.rotate_secret') || 'Rotate secret'}
                      </button>
                      <button class="btn-link" type="button" onclick={() => copyMappingCli(mapping)}>
                        {$t('superadmin.radius.actions.copy_cli') || 'Copy CLI'}
                      </button>
                      <button class="btn-link" type="button" onclick={() => toggleMappingActive(mapping)}>
                        {mapping.is_active
                          ? $t('superadmin.radius.actions.deactivate') || 'Deactivate'
                          : $t('superadmin.radius.actions.activate') || 'Activate'}
                      </button>
                    </div>
                  </td>
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

<ServerFormModal
  bind:show={showServerModal}
  loading={savingServer}
  isEditing={Boolean(editingServerId)}
  bind:server={serverForm}
  {tenants}
  onSubmit={submitServerForm}
/>

<MappingFormModal
  bind:show={showMappingModal}
  loading={savingMapping}
  isEditing={Boolean(editingMappingId)}
  bind:mapping={mappingForm}
  {tenants}
  {servers}
  {routers}
  onGenerateSecret={generateSecretForMappingForm}
  onSubmit={submitMappingForm}
/>

<MappingSecretDialog
  bind:show={showSecretDialog}
  loading={secretDialogLoading}
  mode={secretDialogMode}
  mappingLabel={secretDialogMapping
    ? `${secretDialogMapping.server_name} / ${secretDialogMapping.router_name || secretDialogMapping.nas_name}`
    : ''}
  maskedSecret={secretDialogMapping?.shared_secret_masked || ''}
  revealedSecret={secretDialogRevealed}
  bind:secretDraft={secretDialogDraft}
  onGenerate={generateSecretForDialog}
  onSubmit={submitSecretDialog}
/>

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

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    justify-content: flex-end;
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
    min-width: 1080px;
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

  .muted {
    color: var(--text-secondary);
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

  .muted.badge,
  .badge.muted {
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

  code {
    display: inline-block;
    padding: 0.35rem 0.55rem;
    border-radius: 10px;
    background: var(--bg-tertiary);
    word-break: break-all;
  }

  .row-actions {
    display: flex;
    gap: 0.6rem;
    align-items: center;
  }

  .row-actions.wrap {
    flex-wrap: wrap;
  }

  .btn-link {
    background: none;
    border: none;
    padding: 0;
    color: var(--color-primary);
    cursor: pointer;
    font-weight: 600;
  }

  @media (max-width: 900px) {
    .hero,
    .panel-head {
      flex-direction: column;
    }

    .filters,
    .filters-wide,
    .hero-actions {
      width: 100%;
      justify-content: stretch;
    }

    .filters input,
    .filters select,
    .refresh-btn,
    .hero-actions .btn {
      width: 100%;
    }
  }
</style>
