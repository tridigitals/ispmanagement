<script lang="ts">
  import { api } from '$lib/api/client';
  import type {
    ManagedRadiusAssignmentPayload,
    ManagedRadiusMappingPayload,
    ManagedRadiusSecretValue,
    SuperadminManagedRadiusAssignment,
    SuperadminManagedRadiusMapping,
    SuperadminManagedRadiusRuntimeStatus,
    SuperadminManagedRadiusSession,
    SuperadminManagedRadiusServer,
    SuperadminManagedRadiusUser,
    Tenant,
  } from '$lib/api/types';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import ManagedRadiusFilterToolbar from '$lib/components/superadmin/radius/ManagedRadiusFilterToolbar.svelte';
  import MobileOverflowActions from '$lib/components/ui/MobileOverflowActions.svelte';
  import ResponsiveTabs from '$lib/components/ui/ResponsiveTabs.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { toast } from '$lib/stores/toast';
  import {
    buildManagedRadiusTabs,
    buildManagedRadiusRouterOsCli,
    filterManagedRadiusMappings,
    type ManagedRadiusTabId,
  } from '$lib/utils/managedRadiusControlPlane';
  import {
    formatManagedRadiusSessionOctets,
    getManagedRadiusSessionBadgeTone,
    getManagedRadiusSessionStatus,
    getManagedRadiusUserAttentionCount,
    getManagedRadiusUserBadgeTone,
    getManagedRadiusUserStatus,
  } from './superadminRadiusStatus';
  import { loadSuperadminRadiusDialogs } from './superadminRadiusPageModules';
  import { t } from 'svelte-i18n';
  import { onMount } from 'svelte';

  type RouterOption = {
    id: string;
    tenant_id?: string | null;
    name?: string | null;
    host?: string | null;
  };

  const DEFAULT_ASSIGNMENT_FORM = (): ManagedRadiusAssignmentPayload => ({
    tenant_id: '',
    radius_endpoint_id: '',
    is_active: true,
  });

  const DEFAULT_MAPPING_FORM = (): ManagedRadiusMappingPayload => ({
    tenant_id: '',
    radius_endpoint_id: '',
    router_id: '',
    nas_name: '',
    nas_ip_or_cidr: '',
    shortname: '',
    shared_secret: '',
    is_active: true,
  });

  let tenants = $state<Tenant[]>([]);
  let routers = $state<RouterOption[]>([]);
  let runtimeStatus = $state<SuperadminManagedRadiusRuntimeStatus | null>(null);
  let servers = $state<SuperadminManagedRadiusServer[]>([]);
  let assignments = $state<SuperadminManagedRadiusAssignment[]>([]);
  let mappings = $state<SuperadminManagedRadiusMapping[]>([]);
  let users = $state<SuperadminManagedRadiusUser[]>([]);
  let sessions = $state<SuperadminManagedRadiusSession[]>([]);

  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let activeTab = $state<ManagedRadiusTabId>('assignments');
  let isMobile = $state(false);

  let assignmentSearch = $state('');
  let assignmentTenantFilter = $state('all');
  let assignmentStatusFilter = $state<'all' | 'active' | 'inactive'>('all');
  let assignmentFiltersOpen = $state(false);

  let mappingSearch = $state('');
  let mappingTenantFilter = $state('all');
  let mappingServerFilter = $state('all');
  let mappingStatusFilter = $state<'all' | 'active' | 'inactive'>('all');
  let mappingFiltersOpen = $state(false);

  let userSearch = $state('');
  let tenantFilter = $state('all');
  let routerFilter = $state('all');
  let userStatusFilter = $state<'all' | 'provisioned' | 'not_provisioned'>('all');
  let userFiltersOpen = $state(false);
  let sessionSearch = $state('');
  let sessionTenantFilter = $state('all');
  let sessionRouterFilter = $state('all');
  let sessionFiltersOpen = $state(false);

  let showAssignmentModal = $state(false);
  let savingAssignment = $state(false);
  let editingAssignmentId = $state<string | null>(null);
  let assignmentForm = $state<ManagedRadiusAssignmentPayload>(DEFAULT_ASSIGNMENT_FORM());

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
  let radiusDialogsLoading = $state(false);
  let AssignmentFormModalComponent = $state<any>(null);
  let MappingFormModalComponent = $state<any>(null);
  let MappingSecretDialogComponent = $state<any>(null);

  onMount(() => {
    const mq = window.matchMedia('(max-width: 900px)');
    const updateViewport = () => {
      isMobile = mq.matches;
    };
    updateViewport();
    mq.addEventListener('change', updateViewport);
    void loadData();
    return () => {
      mq.removeEventListener('change', updateViewport);
    };
  });

  async function ensureSuperadminRadiusDialogsLoaded() {
    if (
      AssignmentFormModalComponent &&
      MappingFormModalComponent &&
      MappingSecretDialogComponent
    ) {
      return;
    }
    if (radiusDialogsLoading) return;

    radiusDialogsLoading = true;
    try {
      const modules = await loadSuperadminRadiusDialogs();
      AssignmentFormModalComponent = modules.AssignmentFormModalComponent;
      MappingFormModalComponent = modules.MappingFormModalComponent;
      MappingSecretDialogComponent = modules.MappingSecretDialogComponent;
    } catch (err: any) {
      toast.error(err?.message || 'Failed to load managed RADIUS dialogs');
    } finally {
      radiusDialogsLoading = false;
    }
  }

  async function loadData(opts: { silent?: boolean } = {}) {
    if (opts.silent) refreshing = true;
    else loading = true;

    error = '';
    try {
      const [tenantRes, runtimeRes, serverRes, assignmentRes, mappingRes, userRes, sessionRes, routerRes] = await Promise.all([
        api.superadmin.listTenants(),
        api.superadmin.getManagedRadiusRuntimeStatus(),
        api.superadmin.listManagedRadiusServers(),
        api.superadmin.listManagedRadiusAssignments(),
        api.superadmin.listManagedRadiusMappings(),
        api.superadmin.listManagedRadiusUsers(),
        api.superadmin.listManagedRadiusSessions(),
        api.mikrotik.routers.list().catch(() => []),
      ]);

      tenants = tenantRes.data || [];
      runtimeStatus = runtimeRes || null;
      servers = serverRes.data || [];
      assignments = assignmentRes.data || [];
      mappings = mappingRes.data || [];
      users = userRes.data || [];
      sessions = sessionRes.data || [];
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
    return getManagedRadiusUserStatus(user);
  }

  function userBadgeTone(user: SuperadminManagedRadiusUser) {
    return getManagedRadiusUserBadgeTone(user);
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

  function resetAssignmentForm() {
    assignmentForm = DEFAULT_ASSIGNMENT_FORM();
    editingAssignmentId = null;
  }

  function resetMappingForm() {
    mappingForm = DEFAULT_MAPPING_FORM();
    editingMappingId = null;
  }

  function resetAssignmentFilters() {
    assignmentSearch = '';
    assignmentTenantFilter = 'all';
    assignmentStatusFilter = 'all';
  }

  function resetMappingFilters() {
    mappingSearch = '';
    mappingTenantFilter = 'all';
    mappingServerFilter = 'all';
    mappingStatusFilter = 'all';
  }

  function resetUserFilters() {
    userSearch = '';
    tenantFilter = 'all';
    routerFilter = 'all';
    userStatusFilter = 'all';
  }

  function resetSessionFilters() {
    sessionSearch = '';
    sessionTenantFilter = 'all';
    sessionRouterFilter = 'all';
  }

  function openCreateAssignmentModal() {
    resetAssignmentForm();
    if (!assignmentForm.radius_endpoint_id) {
      assignmentForm.radius_endpoint_id =
        servers.find((server) => server.is_default)?.id || servers[0]?.id || '';
    }
    showAssignmentModal = true;
  }

  function openEditAssignmentModal(assignment: SuperadminManagedRadiusAssignment) {
    editingAssignmentId = assignment.id;
    assignmentForm = {
      tenant_id: assignment.tenant_id,
      radius_endpoint_id: assignment.radius_endpoint_id,
      is_active: assignment.is_active,
    };
    showAssignmentModal = true;
  }

  async function submitAssignmentForm() {
    if (!assignmentForm.tenant_id || !assignmentForm.radius_endpoint_id) {
      toast.error(
        $t('superadmin.radius.toasts.assignment_validation') || 'Complete the assignment form first',
      );
      return;
    }

    savingAssignment = true;
    try {
      if (editingAssignmentId) {
        await api.superadmin.updateManagedRadiusAssignment(editingAssignmentId, assignmentForm);
        toast.success(
          $t('superadmin.radius.toasts.assignment_updated') || 'Tenant assignment updated',
        );
      } else {
        await api.superadmin.createManagedRadiusAssignment(assignmentForm);
        toast.success(
          $t('superadmin.radius.toasts.assignment_created') || 'Tenant assignment created',
        );
      }

      showAssignmentModal = false;
      resetAssignmentForm();
      await loadData({ silent: true });
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to save assignment');
    } finally {
      savingAssignment = false;
    }
  }

  async function toggleAssignmentActive(assignment: SuperadminManagedRadiusAssignment) {
    try {
      await api.superadmin.setManagedRadiusAssignmentActive(
        assignment.id,
        assignment.tenant_id,
        !assignment.is_active,
      );
      toast.success(
        !assignment.is_active
          ? $t('superadmin.radius.toasts.assignment_activated') || 'Assignment activated'
          : $t('superadmin.radius.toasts.assignment_deactivated') || 'Assignment deactivated',
      );
      await loadData({ silent: true });
    } catch (err: any) {
      toast.error(err?.message || String(err) || 'Failed to change assignment state');
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
      radius_endpoint_id: mapping.radius_endpoint_id,
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
      !mappingForm.radius_endpoint_id ||
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

  const runtimeStatusTone = $derived.by(() => {
    if (!runtimeStatus?.enabled) return 'warn';
    return runtimeStatus.running ? 'good' : 'danger';
  });

  const runtimeStatusLabel = $derived.by(() => {
    if (!runtimeStatus?.enabled) {
      return $t('superadmin.radius.runtime.disabled') || 'Disabled';
    }

    return runtimeStatus.running
      ? $t('superadmin.radius.runtime.running') || 'Running'
      : $t('superadmin.radius.runtime.stopped') || 'Stopped';
  });

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
      .filter((server) =>
        mappingTenantFilter === 'all'
          ? true
          : assignments.some(
              (assignment) =>
                assignment.tenant_id === mappingTenantFilter &&
                assignment.radius_endpoint_id === server.id,
            ),
      )
      .map((server) => ({ id: server.id, name: server.name })),
  );

  const assignmentPrimaryFilterOptions = $derived.by(() => [
    {
      value: 'all',
      label: $t('superadmin.radius.filters.all_tenants') || 'All tenants',
    },
    ...mappingTenantOptions.map((tenant) => ({
      value: tenant.id,
      label: tenant.name,
    })),
  ]);

  const mappingPrimaryFilterOptions = $derived.by(() => [
    {
      value: 'all',
      label: $t('superadmin.radius.filters.all_tenants') || 'All tenants',
    },
    ...mappingTenantOptions.map((tenant) => ({
      value: tenant.id,
      label: tenant.name,
    })),
  ]);

  const userPrimaryFilterOptions = $derived.by(() => [
    {
      value: 'all',
      label: $t('superadmin.radius.filters.all_tenants') || 'All tenants',
    },
    ...tenantOptions.map((tenantName) => ({
      value: tenantName,
      label: tenantName,
    })),
  ]);

  const sessionPrimaryFilterOptions = $derived.by(() => [
    {
      value: 'all',
      label: $t('superadmin.radius.filters.all_tenants') || 'All tenants',
    },
    ...tenantOptions.map((tenantName) => ({
      value: tenantName,
      label: tenantName,
    })),
  ]);

  const assignmentAdvancedFilterCount = $derived.by(() =>
    assignmentStatusFilter === 'all' ? 0 : 1,
  );

  const mappingAdvancedFilterCount = $derived.by(() => {
    let count = 0;
    if (mappingServerFilter !== 'all') count += 1;
    if (mappingStatusFilter !== 'all') count += 1;
    return count;
  });

  const userAdvancedFilterCount = $derived.by(() => {
    let count = 0;
    if (routerFilter !== 'all') count += 1;
    if (userStatusFilter !== 'all') count += 1;
    return count;
  });

  const sessionAdvancedFilterCount = $derived.by(() =>
    sessionRouterFilter === 'all' ? 0 : 1,
  );

  const filteredAssignments = $derived.by(() =>
    assignments.filter((assignment) => {
      const q = normalized(assignmentSearch);
      const matchesSearch =
        !q ||
        normalized(assignment.tenant_name).includes(q) ||
        normalized(assignment.endpoint_name).includes(q) ||
        normalized(assignment.radius_host).includes(q);

      const matchesTenant =
        assignmentTenantFilter === 'all' || assignment.tenant_id === assignmentTenantFilter;
      const matchesStatus =
        assignmentStatusFilter === 'all' ||
        (assignmentStatusFilter === 'active' ? assignment.is_active : !assignment.is_active);

      return matchesSearch && matchesTenant && matchesStatus;
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

  const filteredSessions = $derived.by(() =>
    sessions.filter((session) => {
      const q = normalized(sessionSearch);
      const routerName =
        session.router_name || ($t('superadmin.radius.labels.unknown_router') || 'Unknown router');
      const identity = session.radius_identity || session.username;
      const matchesSearch =
        !q ||
        normalized(session.username).includes(q) ||
        normalized(identity).includes(q) ||
        normalized(session.tenant_name).includes(q) ||
        normalized(routerName).includes(q) ||
        normalized(session.acct_session_id).includes(q) ||
        normalized(session.status_type).includes(q);

      const matchesTenant =
        sessionTenantFilter === 'all' || session.tenant_name === sessionTenantFilter;
      const matchesRouter =
        sessionRouterFilter === 'all' || routerName === sessionRouterFilter;

      return matchesSearch && matchesTenant && matchesRouter;
    }),
  );

  const stats = $derived.by(() => ({
    assignments: assignments.length,
    mappings: mappings.length,
    users: users.length,
    sessions: sessions.length,
    outOfSync: getManagedRadiusUserAttentionCount(users),
  }));

  const assignmentColumns = $derived.by(() => [
    { key: 'tenant', label: $t('superadmin.radius.columns.tenant') || 'Tenant' },
    { key: 'server', label: $t('superadmin.radius.columns.server') || 'Server' },
    { key: 'status', label: $t('superadmin.radius.columns.status') || 'Status' },
    { key: 'routers', label: $t('superadmin.radius.columns.routers') || 'Routers', align: 'right' as const },
    { key: 'updated', label: $t('superadmin.radius.columns.updated') || 'Updated' },
    { key: 'actions', label: $t('superadmin.radius.columns.actions') || 'Actions', width: '220px' },
  ]);

  const mappingColumns = $derived.by(() => [
    { key: 'tenant', label: $t('superadmin.radius.columns.tenant') || 'Tenant' },
    { key: 'server', label: $t('superadmin.radius.columns.server') || 'Server' },
    { key: 'router', label: $t('superadmin.radius.columns.router') || 'Router' },
    { key: 'nas', label: $t('superadmin.radius.columns.nas') || 'NAS' },
    { key: 'secret', label: $t('superadmin.radius.columns.secret') || 'Secret' },
    { key: 'status', label: $t('superadmin.radius.columns.status') || 'Status' },
    { key: 'updated', label: $t('superadmin.radius.columns.updated') || 'Updated' },
    { key: 'actions', label: $t('superadmin.radius.columns.actions') || 'Actions', width: '320px' },
  ]);

  const userColumns = $derived.by(() => [
    { key: 'tenant', label: $t('superadmin.radius.columns.tenant') || 'Tenant' },
    { key: 'router', label: $t('superadmin.radius.columns.router') || 'Router' },
    { key: 'username', label: $t('superadmin.radius.columns.username') || 'Username' },
    { key: 'identity', label: $t('superadmin.radius.columns.identity') || 'RADIUS Identity' },
    { key: 'profile', label: $t('superadmin.radius.columns.profile') || 'Profile' },
    { key: 'status', label: $t('superadmin.radius.columns.status') || 'Status' },
    { key: 'last_sync', label: $t('superadmin.radius.columns.last_sync') || 'Last Sync' },
    { key: 'last_error', label: $t('superadmin.radius.columns.last_error') || 'Last Error' },
  ]);

  const sessionColumns = $derived.by(() => [
    { key: 'tenant', label: $t('superadmin.radius.columns.tenant') || 'Tenant' },
    { key: 'router', label: $t('superadmin.radius.columns.router') || 'Router' },
    { key: 'username', label: $t('superadmin.radius.columns.username') || 'Username' },
    { key: 'identity', label: $t('superadmin.radius.columns.identity') || 'RADIUS Identity' },
    { key: 'status', label: $t('superadmin.radius.columns.status') || 'Status' },
    { key: 'ip_address', label: $t('superadmin.radius.columns.ip_address') || 'Framed IP' },
    { key: 'session_id', label: $t('superadmin.radius.columns.session_id') || 'Session ID' },
    { key: 'updated', label: $t('superadmin.radius.columns.updated') || 'Updated' },
  ]);

  const tabs = $derived.by(() => buildManagedRadiusTabs(stats, activeTab));
  const radiusActionItems = $derived.by(() => [
    {
      id: 'new-assignment',
      label: $t('superadmin.radius.actions.new_assignment') || 'New assignment',
      icon: 'layers',
    },
    {
      id: 'new-mapping',
      label: $t('superadmin.radius.actions.new_mapping') || 'New mapping',
      icon: 'network',
      tone: 'primary' as const,
    },
    {
      id: 'refresh',
      label: refreshing
        ? $t('common.loading') || 'Loading...'
        : $t('superadmin.radius.refresh') || 'Refresh',
      icon: 'refresh-cw',
      disabled: refreshing,
    },
  ]);
  const radiusTabItems = $derived.by(() =>
    tabs.map((tab) => ({
      id: tab.id,
      label:
        tab.id === 'assignments'
          ? $t('superadmin.radius.sections.assignments') || 'Tenant Assignments'
          : tab.id === 'mappings'
            ? $t('superadmin.radius.sections.mappings') || 'NAS Mappings'
            : tab.id === 'sessions'
              ? $t('superadmin.radius.sections.sessions') || 'Sessions'
              : $t('superadmin.radius.sections.users') || 'Users',
      count: tab.count,
    })),
  );

  $effect(() => {
    if (!showAssignmentModal && !showMappingModal && !showSecretDialog) return;
    void ensureSuperadminRadiusDialogsLoaded();
  });

  function handleRadiusActionSelect(actionId: string) {
    if (actionId === 'new-assignment') {
      openCreateAssignmentModal();
      return;
    }
    if (actionId === 'new-mapping') {
      openCreateMappingModal();
      return;
    }
    if (actionId === 'refresh') {
      void loadData({ silent: true });
    }
  }
</script>

<div class="page-shell">
  <div class="hero">
    <div>
      <h1>{$t('superadmin.radius.title')}</h1>
      <p>
        {$t('superadmin.radius.subtitle')}
      </p>
      {#if runtimeStatus}
        <div class="runtime-banner">
          <span class="badge" class:good={runtimeStatusTone === 'good'} class:warn={runtimeStatusTone === 'warn'} class:danger={runtimeStatusTone === 'danger'}>
            {runtimeStatusLabel}
          </span>
          <span class="runtime-meta">
            {$t('superadmin.radius.runtime.endpoint')}:
            <strong>{runtimeStatus.advertised_host}:{runtimeStatus.auth_port}/{runtimeStatus.acct_port}</strong>
          </span>
          <span class="runtime-meta">
            {$t('superadmin.radius.runtime.authenticator')}:
            <strong>
              {runtimeStatus.require_message_authenticator
                ? $t('superadmin.radius.runtime.required') || 'Required'
                : $t('superadmin.radius.runtime.optional') || 'Optional'}
            </strong>
          </span>
        </div>
      {/if}
    </div>

    <div class="hero-actions">
      <MobileOverflowActions
        items={radiusActionItems}
        primaryIds={['new-mapping', 'refresh']}
        {isMobile}
        on:select={(event) => handleRadiusActionSelect(event.detail)}
      />
    </div>
  </div>

  {#if loading}
    <div class="state-card">{$t('superadmin.radius.loading')}</div>
  {:else if error}
    <div class="state-card error">{error}</div>
  {:else}
    <div class="stats-grid">
      <StatsCard title={$t('superadmin.radius.stats.assignments')} value={stats.assignments} icon="layers" color="info" />
      <StatsCard title={$t('superadmin.radius.stats.mappings')} value={stats.mappings} icon="network" color="success" />
      <StatsCard title={$t('superadmin.radius.stats.users')} value={stats.users} icon="users" />
      <StatsCard title={$t('superadmin.radius.stats.sessions')} value={stats.sessions} icon="activity" color="info" />
      <StatsCard title={$t('superadmin.radius.stats.out_of_sync')} value={stats.outOfSync} icon="activity" color="warning" />
    </div>

    <section class="panel">
      <ResponsiveTabs
        items={radiusTabItems}
        bind:activeId={activeTab}
        {isMobile}
        priorityCount={3}
        ariaLabel={$t('superadmin.radius.title')}
      />

      {#if activeTab === 'assignments'}
        <ManagedRadiusFilterToolbar
          title={$t('superadmin.radius.sections.assignments')}
          countLabel={`${filteredAssignments.length} / ${assignments.length}`}
          bind:searchQuery={assignmentSearch}
          searchPlaceholder={$t('superadmin.radius.filters.search_assignments')}
          bind:primaryFilterValue={assignmentTenantFilter}
          primaryFilterOptions={assignmentPrimaryFilterOptions}
          primaryFilterAriaLabel={$t('superadmin.radius.filters.all_tenants')}
          bind:filterPanelOpen={assignmentFiltersOpen}
          activeFilterCount={assignmentAdvancedFilterCount}
          onReset={resetAssignmentFilters}
        >
          {#snippet advancedFilters()}
            <div class="advanced-grid">
              <div class="advanced-field">
                <label for="assignment-status-filter"
                  >{$t('superadmin.radius.filters.all_statuses')}</label
                >
                <select id="assignment-status-filter" bind:value={assignmentStatusFilter}>
                  <option value="all">
                    {$t('superadmin.radius.filters.all_statuses')}
                  </option>
                  <option value="active">
                    {$t('superadmin.radius.filters.active')}
                  </option>
                  <option value="inactive">
                    {$t('superadmin.radius.filters.inactive')}
                  </option>
                </select>
              </div>
            </div>
          {/snippet}
        </ManagedRadiusFilterToolbar>

        {#if filteredAssignments.length === 0}
          <div class="empty-state">
            <strong>{$t('superadmin.radius.empty.assignments_title')}</strong>
            <span>{$t('superadmin.radius.empty.assignments_subtitle')}</span>
          </div>
        {:else}
          <div class="table-wrap">
            <Table columns={assignmentColumns} data={filteredAssignments} keyField="id" pagination={true} pageSize={10} mobileView="card">
              {#snippet cell({ item, key })}
                {#if key === 'tenant'}
                  {item.tenant_name}
                {:else if key === 'server'}
                  <div class="primary">{item.endpoint_name}</div>
                  <div class="muted">{item.radius_host}:{item.auth_port}/{item.acct_port}</div>
                {:else if key === 'status'}
                  <span class="badge" class:good={item.is_active} class:muted={!item.is_active}>
                    {item.is_active
                      ? $t('superadmin.radius.status.active') || 'Active'
                      : $t('superadmin.radius.status.inactive') || 'Inactive'}
                  </span>
                {:else if key === 'routers'}
                  {item.router_count}
                {:else if key === 'updated'}
                  {formatDateTime(item.updated_at)}
                {:else if key === 'actions'}
                  <div class="row-actions">
                    <button class="btn-link" type="button" onclick={() => openEditAssignmentModal(item)}>
                      {$t('superadmin.radius.actions.edit')}
                    </button>
                    <button class="btn-link" type="button" onclick={() => toggleAssignmentActive(item)}>
                      {item.is_active
                        ? $t('superadmin.radius.actions.deactivate') || 'Deactivate'
                        : $t('superadmin.radius.actions.activate') || 'Activate'}
                    </button>
                  </div>
                {/if}
              {/snippet}
            </Table>
          </div>
        {/if}
      {:else if activeTab === 'mappings'}
        <ManagedRadiusFilterToolbar
          title={$t('superadmin.radius.sections.mappings')}
          countLabel={`${filteredMappings.length} / ${mappings.length}`}
          bind:searchQuery={mappingSearch}
          searchPlaceholder={$t('superadmin.radius.filters.search_mappings')}
          bind:primaryFilterValue={mappingTenantFilter}
          primaryFilterOptions={mappingPrimaryFilterOptions}
          primaryFilterAriaLabel={$t('superadmin.radius.filters.all_tenants')}
          bind:filterPanelOpen={mappingFiltersOpen}
          activeFilterCount={mappingAdvancedFilterCount}
          onReset={resetMappingFilters}
        >
          {#snippet advancedFilters()}
            <div class="advanced-grid advanced-grid-wide">
              <div class="advanced-field">
                <label for="mapping-server-filter"
                  >{$t('superadmin.radius.filters.all_servers')}</label
                >
                <select id="mapping-server-filter" bind:value={mappingServerFilter}>
                  <option value="all">
                    {$t('superadmin.radius.filters.all_servers')}
                  </option>
                  {#each mappingServerOptions as server}
                    <option value={server.id}>{server.name}</option>
                  {/each}
                </select>
              </div>

              <div class="advanced-field">
                <label for="mapping-status-filter"
                  >{$t('superadmin.radius.filters.all_statuses')}</label
                >
                <select id="mapping-status-filter" bind:value={mappingStatusFilter}>
                  <option value="all">
                    {$t('superadmin.radius.filters.all_statuses')}
                  </option>
                  <option value="active">
                    {$t('superadmin.radius.filters.active')}
                  </option>
                  <option value="inactive">
                    {$t('superadmin.radius.filters.inactive')}
                  </option>
                </select>
              </div>
            </div>
          {/snippet}
        </ManagedRadiusFilterToolbar>

        {#if filteredMappings.length === 0}
          <div class="empty-state">
            <strong>{$t('superadmin.radius.empty.mappings_title')}</strong>
            <span>{$t('superadmin.radius.empty.mappings_subtitle')}</span>
          </div>
        {:else}
          <div class="table-wrap">
            <Table columns={mappingColumns} data={filteredMappings} keyField="id" pagination={true} pageSize={10} mobileView="card">
              {#snippet cell({ item, key })}
                {#if key === 'tenant'}
                  {item.tenant_name}
                {:else if key === 'server'}
                  <div class="primary">{item.endpoint_name}</div>
                  <div class="muted">{item.radius_host}:{item.auth_port}/{item.acct_port}</div>
                {:else if key === 'router'}
                  {item.router_name || ($t('superadmin.radius.labels.unknown_router') || 'Unknown router')}
                {:else if key === 'nas'}
                  <div class="primary">{item.nas_name}</div>
                  <div class="muted">{item.nas_ip_or_cidr}</div>
                  {#if item.shortname}
                    <div class="muted">{item.shortname}</div>
                  {/if}
                {:else if key === 'secret'}
                  <code>{item.shared_secret_masked}</code>
                {:else if key === 'status'}
                  <span class="badge" class:good={item.is_active} class:muted={!item.is_active}>
                    {item.is_active
                      ? $t('superadmin.radius.status.active') || 'Active'
                      : $t('superadmin.radius.status.inactive') || 'Inactive'}
                  </span>
                {:else if key === 'updated'}
                  {formatDateTime(item.updated_at)}
                {:else if key === 'actions'}
                  <div class="row-actions wrap">
                    <button class="btn-link" type="button" onclick={() => openEditMappingModal(item)}>
                      {$t('superadmin.radius.actions.edit')}
                    </button>
                    <button class="btn-link" type="button" onclick={() => openSecretDialog(item, 'reveal')}>
                      {$t('superadmin.radius.actions.reveal_secret')}
                    </button>
                    <button class="btn-link" type="button" onclick={() => openSecretDialog(item, 'rotate')}>
                      {$t('superadmin.radius.actions.rotate_secret')}
                    </button>
                    <button class="btn-link" type="button" onclick={() => copyMappingCli(item)}>
                      {$t('superadmin.radius.actions.copy_cli')}
                    </button>
                    <button class="btn-link" type="button" onclick={() => toggleMappingActive(item)}>
                      {item.is_active
                        ? $t('superadmin.radius.actions.deactivate') || 'Deactivate'
                        : $t('superadmin.radius.actions.activate') || 'Activate'}
                    </button>
                  </div>
                {/if}
              {/snippet}
            </Table>
          </div>
        {/if}
        {:else if activeTab === 'users'}
        <ManagedRadiusFilterToolbar
          title={$t('superadmin.radius.sections.users')}
          countLabel={`${filteredUsers.length} / ${users.length}`}
          bind:searchQuery={userSearch}
          searchPlaceholder={$t('superadmin.radius.filters.search_users')}
          bind:primaryFilterValue={tenantFilter}
          primaryFilterOptions={userPrimaryFilterOptions}
          primaryFilterAriaLabel={$t('superadmin.radius.filters.all_tenants')}
          bind:filterPanelOpen={userFiltersOpen}
          activeFilterCount={userAdvancedFilterCount}
          onReset={resetUserFilters}
        >
          {#snippet advancedFilters()}
            <div class="advanced-grid advanced-grid-wide">
              <div class="advanced-field">
                <label for="user-router-filter"
                  >{$t('superadmin.radius.filters.all_routers')}</label
                >
                <select id="user-router-filter" bind:value={routerFilter}>
                  <option value="all">
                    {$t('superadmin.radius.filters.all_routers')}
                  </option>
                  {#each routerOptions as routerName}
                    <option value={routerName}>{routerName}</option>
                  {/each}
                </select>
              </div>

              <div class="advanced-field">
                <label for="user-status-filter"
                  >{$t('superadmin.radius.filters.all_users')}</label
                >
                <select id="user-status-filter" bind:value={userStatusFilter}>
                  <option value="all">
                    {$t('superadmin.radius.filters.all_users')}
                  </option>
                  <option value="provisioned">
                    {$t('superadmin.radius.filters.provisioned')}
                  </option>
                  <option value="not_provisioned">
                    {$t('superadmin.radius.filters.not_provisioned')}
                  </option>
                </select>
              </div>
            </div>
          {/snippet}
        </ManagedRadiusFilterToolbar>

        {#if filteredUsers.length === 0}
          <div class="empty-state">
            <strong>{$t('superadmin.radius.empty.users_title')}</strong>
            <span>{$t('superadmin.radius.empty.users_subtitle')}</span>
          </div>
        {:else}
          <div class="table-wrap">
            <Table columns={userColumns} data={filteredUsers} keyField="id" pagination={true} pageSize={10} mobileView="card">
              {#snippet cell({ item, key })}
                {#if key === 'tenant'}
                  {item.tenant_name}
                {:else if key === 'router'}
                  {item.router_name || ($t('superadmin.radius.labels.unknown_router') || 'Unknown router')}
                {:else if key === 'username'}
                  <div class="primary">{item.username}</div>
                {:else if key === 'identity'}
                  {item.radius_identity || item.username}
                {:else if key === 'profile'}
                  {item.router_profile_name || ($t('superadmin.radius.labels.none') || 'None')}
                {:else if key === 'status'}
                  <span class="badge" class:good={userBadgeTone(item) === 'good'} class:warn={userBadgeTone(item) === 'warn'} class:danger={userBadgeTone(item) === 'danger'}>
                    {#if item.is_provisioned}
                      {$t('superadmin.radius.status.provisioned')}
                    {:else if item.provisioning_error}
                      {$t('superadmin.radius.status.needs_attention')}
                    {:else}
                      {$t('superadmin.radius.status.not_provisioned')}
                    {/if}
                  </span>
                {:else if key === 'last_sync'}
                  {formatDateTime(item.provisioned_at)}
                {:else if key === 'last_error'}
                  <span class="error-text">
                    {item.provisioning_error || ($t('superadmin.radius.labels.none') || 'None')}
                  </span>
                {/if}
              {/snippet}
            </Table>
          </div>
        {/if}
      {:else}
        <ManagedRadiusFilterToolbar
          title={$t('superadmin.radius.sections.sessions')}
          countLabel={`${filteredSessions.length} / ${sessions.length}`}
          bind:searchQuery={sessionSearch}
          searchPlaceholder={$t('superadmin.radius.filters.search_sessions')}
          bind:primaryFilterValue={sessionTenantFilter}
          primaryFilterOptions={sessionPrimaryFilterOptions}
          primaryFilterAriaLabel={$t('superadmin.radius.filters.all_tenants')}
          bind:filterPanelOpen={sessionFiltersOpen}
          activeFilterCount={sessionAdvancedFilterCount}
          onReset={resetSessionFilters}
        >
          {#snippet advancedFilters()}
            <div class="advanced-grid">
              <div class="advanced-field">
                <label for="session-router-filter"
                  >{$t('superadmin.radius.filters.all_routers')}</label
                >
                <select id="session-router-filter" bind:value={sessionRouterFilter}>
                  <option value="all">
                    {$t('superadmin.radius.filters.all_routers')}
                  </option>
                  {#each routerOptions as routerName}
                    <option value={routerName}>{routerName}</option>
                  {/each}
                </select>
              </div>
            </div>
          {/snippet}
        </ManagedRadiusFilterToolbar>

        {#if filteredSessions.length === 0}
          <div class="empty-state">
            <strong>{$t('superadmin.radius.empty.sessions_title')}</strong>
            <span>{$t('superadmin.radius.empty.sessions_subtitle')}</span>
          </div>
        {:else}
          <div class="table-wrap">
            <Table columns={sessionColumns} data={filteredSessions} keyField="id" pagination={true} pageSize={10} mobileView="card">
              {#snippet cell({ item, key })}
                {#if key === 'tenant'}
                  {item.tenant_name}
                {:else if key === 'router'}
                  {item.router_name || ($t('superadmin.radius.labels.unknown_router') || 'Unknown router')}
                {:else if key === 'username'}
                  <div class="primary">{item.username}</div>
                {:else if key === 'identity'}
                  {item.radius_identity || item.username}
                {:else if key === 'status'}
                  <span
                    class="badge"
                    class:good={getManagedRadiusSessionBadgeTone(item) === 'good'}
                    class:muted={getManagedRadiusSessionBadgeTone(item) === 'muted'}
                  >
                    {getManagedRadiusSessionStatus(item) === 'online'
                      ? $t('superadmin.radius.status.online') || 'Online'
                      : $t('superadmin.radius.status.offline') || 'Offline'}
                  </span>
                  <div class="muted">{item.status_type}</div>
                {:else if key === 'ip_address'}
                  {item.framed_ip_address || ($t('superadmin.radius.labels.none') || 'None')}
                {:else if key === 'session_id'}
                  <code>{item.acct_session_id}</code>
                  <div class="muted">
                    RX {formatManagedRadiusSessionOctets(item.input_octets)} / TX {formatManagedRadiusSessionOctets(item.output_octets)}
                  </div>
                {:else if key === 'updated'}
                  {formatDateTime(item.last_update_at || item.updated_at)}
                {/if}
              {/snippet}
            </Table>
          </div>
        {/if}
      {/if}
    </section>
  {/if}
</div>

{#if AssignmentFormModalComponent}
  <AssignmentFormModalComponent
    bind:show={showAssignmentModal}
    loading={savingAssignment}
    isEditing={Boolean(editingAssignmentId)}
    bind:assignment={assignmentForm}
    {tenants}
    {servers}
    onSubmit={submitAssignmentForm}
  />
{/if}

{#if MappingFormModalComponent}
  <MappingFormModalComponent
    bind:show={showMappingModal}
    loading={savingMapping}
    isEditing={Boolean(editingMappingId)}
    bind:mapping={mappingForm}
    {tenants}
    {assignments}
    {routers}
    onGenerateSecret={generateSecretForMappingForm}
    onSubmit={submitMappingForm}
  />
{/if}

{#if MappingSecretDialogComponent}
  <MappingSecretDialogComponent
    bind:show={showSecretDialog}
    loading={secretDialogLoading}
    mode={secretDialogMode}
    mappingLabel={secretDialogMapping
      ? `${secretDialogMapping.endpoint_name} / ${secretDialogMapping.router_name || secretDialogMapping.nas_name}`
      : ''}
    maskedSecret={secretDialogMapping?.shared_secret_masked || ''}
    revealedSecret={secretDialogRevealed}
    bind:secretDraft={secretDialogDraft}
    onGenerate={generateSecretForDialog}
    onSubmit={submitSecretDialog}
  />
{/if}

<style>
  .page-shell {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1500px;
    margin: 0 auto;
    display: grid;
    gap: 1.5rem;
    overflow-x: hidden;
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

  .runtime-banner {
    margin-top: 0.9rem;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 0.75rem;
    align-items: center;
  }

  .runtime-meta {
    color: var(--text-secondary);
    font-size: 0.95rem;
    word-break: break-all;
  }

  .runtime-meta strong {
    color: var(--text-primary);
    font-weight: 600;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    justify-content: flex-end;
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
    border-radius: var(--radius-lg);
    padding: 1rem;
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }

  .state-card.error {
    color: var(--color-danger, #dc2626);
  }

  .advanced-grid {
    display: grid;
    grid-template-columns: minmax(0, 280px);
    gap: 0.75rem;
  }

  .advanced-grid-wide {
    grid-template-columns: repeat(2, minmax(0, 240px));
  }

  .advanced-field {
    display: grid;
    gap: 0.35rem;
  }

  .advanced-field label {
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .advanced-field select {
    min-height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-primary);
    color: var(--text-primary);
    padding: 0 0.85rem;
    outline: none;
  }

  .advanced-field select:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .table-wrap {
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
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
    .hero {
      flex-direction: column;
    }

    .page-shell {
      padding: 16px;
      gap: 1rem;
    }

    .hero p {
      max-width: none;
    }

    .hero-actions {
      width: 100%;
      justify-content: stretch;
    }

    .hero-actions :global(.btn),
    .hero-actions :global(button) {
      flex: 1;
    }

    .stats-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0.75rem;
    }

    .panel {
      padding: 0.85rem;
    }

    .advanced-grid,
    .advanced-grid-wide {
      grid-template-columns: 1fr;
    }

    .row-actions {
      flex-wrap: wrap;
      justify-content: flex-end;
      gap: 0.45rem 0.75rem;
    }

    .error-text {
      max-width: none;
    }
  }

  @media (max-width: 560px) {
    .page-shell {
      padding: 12px;
      gap: 0.75rem;
    }

    .stats-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0.6rem;
    }

    .panel {
      padding: 0.65rem;
    }

    .hero h1 {
      font-size: 1.25rem;
    }

    code {
      max-width: 100%;
      white-space: normal;
    }
  }
</style>
