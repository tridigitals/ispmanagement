<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can, token, user } from '$lib/stores/auth';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';
  import {
    api,
    type AuditLog,
    type CustomerSubscriptionView,
    type FileRecord,
    type IspPackageRouterMappingView,
    type InstallationWorkOrderView,
    type ManagedRadiusRouterSetup,
    type PppoeAccountPublic,
    type TeamMember,
    type WorkOrderRescheduleRequestView,
  } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import { formatDateTime } from '$lib/utils/date';
  import {
    getInstallationInternetTestTargetOptions,
    getInstallationInternetTestTargetHint,
    normalizeInstallationInternetTestTarget,
    resolveInstallationInternetTestRouterId,
    type InstallationInternetTestTarget,
  } from '$lib/utils/installationInternetTest';
  import { shouldAllowInstallationInvoiceCreation } from '$lib/utils/installationInvoice';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import InstallationCableMap from '$lib/components/network/InstallationCableMap.svelte';

  let loading = $state(true);
  let busyId = $state<string | null>(null);
  let creatingInvoiceId = $state<string | null>(null);
  let rows = $state<InstallationWorkOrderView[]>([]);
  let team = $state<TeamMember[]>([]);
  let includeClosed = $state(false);
  let search = $state('');
  let statusFilter = $state('all');

  let detailOpen = $state(false);
  let activeRow = $state<InstallationWorkOrderView | null>(null);
  let cancelDialogOpen = $state(false);
  let cancelTarget = $state<InstallationWorkOrderView | null>(null);
  let cancelReason = $state('');
  let formAssignee = $state('');
  let formSchedule = $state('');
  let formNotes = $state('');
  let checkCable = $state(false);
  let checkOnt = $state(false);
  let checkPppoe = $state(false);
  let checkSpeed = $state(false);
  let showCableMapDrawer = $state(false);
  let installationPhotos = $state<FileRecord[]>([]);
  let uploadingPhotos = $state(false);
  let installationSubscription = $state<CustomerSubscriptionView | null>(null);
  let installationPppoeAccount = $state<PppoeAccountPublic | null>(null);
  let installationPppoeMappings = $state<IspPackageRouterMappingView[]>([]);
  let installationManagedRadiusSetup = $state<ManagedRadiusRouterSetup | null>(null);
  let installationManagedRadiusLoadError = $state('');
  let loadingInstallationPppoe = $state(false);
  let savingInstallationPppoe = $state(false);
  let installationPppoeUsername = $state('');
  let installationPppoePassword = $state('');
  let installationPppoeComment = $state('');
  let installationPppoeTarget = $state<InstallationInternetTestTarget>('router');
  let onsiteFocusIndex = $state<number | null>(null);
  let canManageWorkOrders = $derived($can('manage', 'work_orders'));
  let canReadAuditLogs = $derived($can('read', 'audit_logs'));
  let currentUserId = $derived(($user?.id || '').trim());
  let timelineLoading = $state(false);
  let timelineRows = $state<AuditLog[]>([]);
  let rescheduleLoading = $state(false);
  let rescheduleRequest = $state<WorkOrderRescheduleRequestView | null>(null);
  let rescheduleDecisionBusy = $state(false);
  let rescheduleDecisionNotes = $state('');
  let rescheduleOverrideAt = $state('');
  let isAdminOwner = $derived.by(() => {
    const globalRole = `${$user?.role || ''}`.trim().toLowerCase();
    const tenantRole = `${($user as any)?.tenant_role || ''}`.trim().toLowerCase();
    return (
      !!$user &&
      (($user as any)?.is_super_admin === true ||
        globalRole === 'owner' ||
        globalRole === 'admin' ||
        tenantRole === 'owner' ||
        tenantRole === 'admin')
    );
  });
  let canReviewReschedule = $derived.by(
    () => !!activeRow && canManageWorkOrders && (isAdminOwner || isAssignedToCurrentUser(activeRow)),
  );
  const CANCEL_REASON_MIN = 10;
  const INSTALLATION_REFRESH_SIGNAL_KEY = 'nm_installation_work_order_refresh';
  let lastHandledRefreshSignalTs = $state(0);

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
  const assignableTeam = $derived.by(() =>
    team
      .filter((member) => member.is_active)
      .sort((a, b) =>
        `${a.name || a.email}`.toLowerCase().localeCompare(`${b.name || b.email}`.toLowerCase()),
      ),
  );
  const assigneeOptions = $derived.by(() => {
    const options = assignableTeam.map((member) => ({
      value: member.user_id,
      label: `${member.name || member.email} (${member.role_name || member.role || '-'})`,
    }));
    if (formAssignee && !options.some((option) => option.value === formAssignee)) {
      const current = team.find((member) => member.user_id === formAssignee);
      options.unshift({
        value: formAssignee,
        label: current
          ? `${current.name || current.email} (${current.role_name || current.role || '-'})`
          : formAssignee,
      });
    }
    return options;
  });

  onMount(() => {
    if (!$can('read', 'work_orders') && !$can('manage', 'work_orders')) {
      goto('/unauthorized');
      return;
    }
    void loadAll();

    const onStorage = (event: StorageEvent) => {
      if (event.key !== INSTALLATION_REFRESH_SIGNAL_KEY || !event.newValue) return;
      void maybeHandleRefreshSignal(event.newValue, false);
    };
    const onFocus = () => {
      const raw = localStorage.getItem(INSTALLATION_REFRESH_SIGNAL_KEY);
      if (!raw) return;
      void maybeHandleRefreshSignal(raw, true);
    };
    const onMessage = (event: MessageEvent) => {
      const data = event.data as any;
      if (!data || typeof data !== 'object') return;
      if (data.type !== 'nm_work_order_updated') return;
      if (!activeRow || data.work_order_id !== activeRow.id) return;
      void loadAll();
      toast.success(
        tr(
          'admin.network.installations.cable_route_synced',
          'Cable route update synced from topology map.',
        ),
      );
    };
    window.addEventListener('storage', onStorage);
    window.addEventListener('focus', onFocus);
    window.addEventListener('message', onMessage);

    return () => {
      window.removeEventListener('storage', onStorage);
      window.removeEventListener('focus', onFocus);
      window.removeEventListener('message', onMessage);
    };
  });

  onDestroy(() => {
    // onMount cleanup handles event listener removal.
  });

  async function maybeHandleRefreshSignal(raw: string, showToast: boolean) {
    let payload: { work_order_id?: string; ts?: number } | null = null;
    try {
      payload = JSON.parse(raw);
    } catch {
      return;
    }
    if (!payload?.work_order_id || !payload?.ts) return;
    if (payload.ts <= lastHandledRefreshSignalTs) return;
    if (!activeRow || payload.work_order_id !== activeRow.id) return;

    lastHandledRefreshSignalTs = payload.ts;
    await loadAll();
    if (showToast) {
      toast.success(
        tr(
          'admin.network.installations.cable_route_synced',
          'Cable route update synced from topology map.',
        ),
      );
    }
  }

  async function loadAll() {
    loading = true;
    try {
      const [workOrders, members] = await Promise.all([
        api.workOrders.list({ include_closed: includeClosed, limit: 300 }),
        canManageWorkOrders ? api.workOrders.assignees().catch(() => [] as TeamMember[]) : Promise.resolve([] as TeamMember[]),
      ]);
      rows = workOrders;
      team = members;

      // Keep detail modal in sync with latest server state (including reschedule requests)
      if (detailOpen && activeRow) {
        const refreshed = workOrders.find((x) => x.id === activeRow?.id) || null;
        activeRow = refreshed;
        if (refreshed) {
          formAssignee = refreshed.assigned_to || formAssignee;
          formSchedule = refreshed.scheduled_at ? toLocalInputValue(refreshed.scheduled_at) : formSchedule;
          void loadRescheduleRequest(refreshed.id);
        } else {
          rescheduleRequest = null;
        }
      }
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load installation work orders');
    } finally {
      loading = false;
    }
  }

  function isUnassigned(row: InstallationWorkOrderView) {
    return !row.assigned_to || row.assigned_to.trim().length === 0;
  }

  function isAssignedToCurrentUser(row: InstallationWorkOrderView) {
    if (!currentUserId) return false;
    return (row.assigned_to || '').trim() === currentUserId;
  }

  function canOperateRow(row: InstallationWorkOrderView) {
    return isAdminOwner || isAssignedToCurrentUser(row);
  }

  function canTakeRow(row: InstallationWorkOrderView) {
    return canManageWorkOrders && row.status === 'pending' && isUnassigned(row);
  }

  function canReleaseRow(row: InstallationWorkOrderView) {
    return canManageWorkOrders && isAdminOwner && row.status === 'pending' && !isUnassigned(row);
  }

  async function claimWorkOrder(row: InstallationWorkOrderView) {
    if (!canTakeRow(row)) return;
    busyId = row.id;
    try {
      await api.workOrders.claim(row.id);
      toast.success(tr('admin.network.installations.claim_ok', 'Work order taken'));
      await loadAll();
      if (activeRow?.id === row.id) {
        const refreshed = rows.find((x) => x.id === row.id);
        if (refreshed) openDetail(refreshed);
      }
    } catch (e: any) {
      toast.error(e?.message || tr('admin.network.installations.claim_fail', 'Failed to take work order'));
    } finally {
      busyId = null;
    }
  }

  async function releaseWorkOrder(row: InstallationWorkOrderView) {
    if (!canReleaseRow(row)) return;
    busyId = row.id;
    try {
      await api.workOrders.release(row.id);
      toast.success(tr('admin.network.installations.release_ok', 'Assignee released'));
      await loadAll();
      if (activeRow?.id === row.id) {
        const refreshed = rows.find((x) => x.id === row.id);
        if (refreshed) openDetail(refreshed);
      }
    } catch (e: any) {
      toast.error(
        e?.message || tr('admin.network.installations.release_fail', 'Failed to release assignee'),
      );
    } finally {
      busyId = null;
    }
  }

  function hasValidCancelReason(notes?: string) {
    return (notes || '').trim().length >= CANCEL_REASON_MIN;
  }

  async function setStatus(
    row: InstallationWorkOrderView,
    action: 'start' | 'complete' | 'cancel' | 'reopen',
    notes?: string,
  ): Promise<boolean> {
    if (action === 'start' && !isPlanReady(row.assigned_to || '', row.scheduled_at || '')) {
      toast.error(tr('admin.network.installations.plan_required', 'Set assignee and schedule before starting.'));
      return false;
    }
    if (action === 'cancel' && !hasValidCancelReason(notes)) {
      toast.error(
        tr(
          'admin.network.installations.cancel_reason_required',
          `Cancellation reason is required (minimum ${CANCEL_REASON_MIN} characters).`,
        ),
      );
      return false;
    }
    busyId = row.id;
    try {
      if (action === 'start') await api.workOrders.start(row.id, notes);
      if (action === 'complete') await api.workOrders.complete(row.id, notes);
      if (action === 'cancel') await api.workOrders.cancel(row.id, notes);
      if (action === 'reopen') await api.workOrders.reopen(row.id, notes);

      const shouldRevealClosed = (action === 'complete' || action === 'cancel') && !includeClosed;
      if (shouldRevealClosed) {
        includeClosed = true;
      }

      toast.success(tr(`admin.network.installations.${action}_ok`, 'Updated'));
      await loadAll();

      if (activeRow?.id === row.id) {
        const refreshed = rows.find((x) => x.id === row.id);
        if (refreshed) {
          openDetail(refreshed);
        } else {
          closeDetail();
        }
      }

      if (shouldRevealClosed) {
        toast.info(
          tr(
            'admin.network.installations.closed_revealed',
            'Work order moved to closed list. Closed filter is now visible.',
          ),
        );
      }
      return true;
    } catch (e: any) {
      toast.error(e?.message || 'Update failed');
      return false;
    } finally {
      busyId = null;
    }
  }

  function openCancelDialog(row: InstallationWorkOrderView) {
    cancelTarget = row;
    cancelReason = '';
    cancelDialogOpen = true;
  }

  function closeCancelDialog() {
    cancelDialogOpen = false;
    cancelTarget = null;
    cancelReason = '';
  }

  async function confirmCancelFromDialog() {
    if (!cancelTarget) return;
    const ok = await setStatus(cancelTarget, 'cancel', cancelReason);
    if (ok) closeCancelDialog();
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
    formNotes = extractEditableNotes(row.notes);
    detailOpen = true;
    const checklist = parseChecklistStateFromNotes(row.notes);
    checkCable = checklist.cable;
    checkOnt = checklist.ont;
    checkPppoe = checklist.pppoe;
    checkSpeed = checklist.speed;
    showCableMapDrawer = false;
    installationPhotos = parsePhotoIdsFromNotes(row.notes).map((id, index) => ({
      id,
      tenant_id: '',
      name: `photo-${index + 1}`,
      original_name: `photo-${index + 1}`,
      path: '',
      size: 0,
      content_type: 'image/*',
      uploaded_by: null,
      created_at: '',
      updated_at: '',
    }));
    onsiteFocusIndex = null;
    rescheduleRequest = null;
    rescheduleLoading = false;
    rescheduleDecisionNotes = '';
    rescheduleOverrideAt = '';
    void loadWorkOrderTimeline(row.id);
    void loadRescheduleRequest(row.id);
    void loadInstallationPppoeContext(row);
  }

  function closeDetail() {
    detailOpen = false;
    activeRow = null;
    formAssignee = '';
    formSchedule = '';
    formNotes = '';
    timelineRows = [];
    timelineLoading = false;
    rescheduleLoading = false;
    rescheduleRequest = null;
    rescheduleDecisionBusy = false;
    rescheduleDecisionNotes = '';
    rescheduleOverrideAt = '';
    installationPhotos = [];
    uploadingPhotos = false;
    showCableMapDrawer = false;
    installationSubscription = null;
    installationPppoeAccount = null;
    installationPppoeMappings = [];
    installationManagedRadiusSetup = null;
    installationManagedRadiusLoadError = '';
    loadingInstallationPppoe = false;
    savingInstallationPppoe = false;
    installationPppoeUsername = '';
    installationPppoePassword = '';
    installationPppoeComment = '';
    installationPppoeTarget = 'router';
    onsiteFocusIndex = null;
  }

  async function loadWorkOrderTimeline(workOrderId: string) {
    if (!canReadAuditLogs || !workOrderId) {
      timelineRows = [];
      return;
    }
    timelineLoading = true;
    try {
      const res = await api.audit.listTenant(1, 30, {
        resource: 'installation_work_orders',
        resource_id: workOrderId,
      });
      timelineRows = (res?.data || []).filter((log) =>
        `${log.action || ''}`.toUpperCase().startsWith('WORK_ORDER_'),
      );
    } catch {
      timelineRows = [];
    } finally {
      timelineLoading = false;
    }
  }

  async function loadRescheduleRequest(workOrderId: string) {
    if (!canManageWorkOrders || !workOrderId) {
      rescheduleRequest = null;
      return;
    }
    rescheduleLoading = true;
    try {
      rescheduleRequest = await api.workOrders.getRescheduleRequest(workOrderId);
      rescheduleOverrideAt = toLocalInputValue(rescheduleRequest?.requested_schedule_at || '');
    } catch {
      rescheduleRequest = null;
    } finally {
      rescheduleLoading = false;
    }
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

  function parseChecklistStateFromNotes(notes: string | null | undefined) {
    const raw = String(notes || '');
    const hasChecked = (label: string) => {
      const escaped = label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      return new RegExp(`\\[x\\]\\s+${escaped}`, 'i').test(raw);
    };
    return {
      cable: hasChecked('Cable installed'),
      ont: hasChecked('ONT installed'),
      pppoe: hasChecked('PPPoE configured'),
      speed: hasChecked('Speed test passed'),
    };
  }

  function stripGeneratedInstallationSections(notes: string | null | undefined) {
    const raw = String(notes || '').replace(/\r\n/g, '\n');
    return raw
      .replace(
        /(?:^|\n)Installation checklist:\n(?:\[[xX ]\]\s.*(?:\n|$)){1,8}/g,
        '\n',
      )
      .replace(/(?:^|\n)Installation photos:\n(?:- .*(?:\n|$))+/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

  function extractEditableNotes(notes: string | null | undefined) {
    return stripGeneratedInstallationSections(notes);
  }

  function buildPersistedInstallationNotes() {
    const extra = stripGeneratedInstallationSections(formNotes);
    const checklist = buildChecklistNote();
    const photos = buildInstallationPhotosNote();
    return [extra, checklist, photos]
      .filter((part) => part && part.trim().length > 0)
      .join('\n\n');
  }

  async function loadInstallationPppoeContext(row: InstallationWorkOrderView) {
    loadingInstallationPppoe = true;
    installationSubscription = null;
    installationPppoeAccount = null;
    installationPppoeMappings = [];
    installationManagedRadiusSetup = null;
    installationManagedRadiusLoadError = '';
    installationPppoeUsername = '';
    installationPppoePassword = '';
    installationPppoeComment = '';
    try {
      const [subRes, pppoeRes] = await Promise.all([
        api.customers.subscriptions.list(row.customer_id, { page: 1, per_page: 200 }),
        api.pppoe.accounts.list({
          customer_id: row.customer_id,
          location_id: row.location_id,
          page: 1,
          per_page: 50,
        }),
      ]);
      const subscription =
        ((subRes?.data || []) as CustomerSubscriptionView[]).find((item) => item.id === row.subscription_id) ||
        null;
      installationSubscription = subscription;
      installationPppoeAccount =
        (((pppoeRes?.data || []) as PppoeAccountPublic[]).find(
          (item) => item.location_id === row.location_id,
        ) || null);

      const explicitRouterId =
        subscription?.router_id || row.router_id || installationPppoeAccount?.router_id || '';
      const packageId = subscription?.package_id || row.package_id || installationPppoeAccount?.package_id || '';
      let routerId = explicitRouterId;
      if (routerId) {
        installationPppoeMappings = await api.ispPackages.routerMappings.list({
          router_id: routerId,
        });
      } else if (packageId) {
        installationPppoeMappings = await api.ispPackages.routerMappings.list();
        routerId = resolveInstallationInternetTestRouterId({
          explicitRouterId,
          packageId,
          mappings: installationPppoeMappings,
        });
      }

      if (routerId) {
        try {
          installationManagedRadiusSetup = await api.mikrotik.routers.managedRadiusSetup(routerId);
          installationManagedRadiusLoadError = '';
        } catch {
          installationManagedRadiusSetup = null;
          installationManagedRadiusLoadError = tr(
            'admin.network.installations.managed_radius_load_failed',
            'Managed RADIUS setup could not be loaded. Check permissions or router setup.',
          );
        }
      }

      if (installationPppoeAccount) {
        installationPppoeUsername = installationPppoeAccount.username || '';
        installationPppoeComment = installationPppoeAccount.comment || '';
      }

      const nextTargetOptions = getInstallationInternetTestTargetOptions({
        routerId,
        managedRadiusConfigured: installationManagedRadiusSetup?.configured,
      });
      installationPppoeTarget = normalizeInstallationInternetTestTarget(
        installationPppoeAccount?.account_source === 'managed_radius' ? 'managed_radius' : 'router',
        nextTargetOptions,
      );
    } catch (e: any) {
      toast.error(e?.message || 'Failed to prepare PPPoE installation form');
    } finally {
      loadingInstallationPppoe = false;
    }
  }

  const installationPppoeMapping = $derived.by(() => {
    const subscription = installationSubscription;
    const packageId =
      subscription?.package_id || activeRow?.package_id || installationPppoeAccount?.package_id || '';
    if (!packageId) return null;
    const routerId =
      subscription?.router_id || activeRow?.router_id || installationPppoeAccount?.router_id || '';
    if (routerId) {
      return (
        installationPppoeMappings.find(
          (item) => item.router_id === routerId && item.package_id === packageId,
        ) || null
      );
    }
    const packageMatches = installationPppoeMappings.filter((item) => item.package_id === packageId);
    if (packageMatches.length === 1) return packageMatches[0];
    return null;
  });

  const installationPppoeRouterLabel = $derived.by(
    () =>
      installationSubscription?.router_name ||
      activeRow?.router_name ||
      installationPppoeMapping?.router_name ||
      installationSubscription?.router_id ||
      activeRow?.router_id ||
      installationPppoeAccount?.router_id ||
      '-',
  );

  const installationPppoeTargetOptions = $derived.by(() =>
    getInstallationInternetTestTargetOptions({
      routerId:
        resolveInstallationInternetTestRouterId({
          explicitRouterId:
            installationSubscription?.router_id ||
            activeRow?.router_id ||
            installationPppoeAccount?.router_id ||
            installationPppoeMapping?.router_id ||
            '',
          packageId:
            installationSubscription?.package_id ||
            activeRow?.package_id ||
            installationPppoeAccount?.package_id ||
            installationPppoeMapping?.package_id ||
            '',
          mappings: installationPppoeMappings,
        }),
      managedRadiusConfigured: installationManagedRadiusSetup?.configured,
    }),
  );

  $effect(() => {
    installationPppoeTarget = normalizeInstallationInternetTestTarget(
      installationPppoeTarget,
      installationPppoeTargetOptions,
    );
  });

  const installationPppoeTargetLabel = $derived.by(
    () =>
      installationPppoeTargetOptions.find((option) => option.value === installationPppoeTarget)?.label ||
      'Router',
  );

  const installationManagedRadiusHint = $derived.by(() =>
    getInstallationInternetTestTargetHint({
      managedRadiusConfigured: installationManagedRadiusSetup?.configured,
      managedRadiusLoadError: installationManagedRadiusLoadError,
      planUpgradeRequired: installationManagedRadiusSetup?.plan_upgrade_required,
      tenantHasActiveAssignment: installationManagedRadiusSetup?.tenant_has_active_assignment,
      canCreateMapping: installationManagedRadiusSetup?.can_create_mapping,
      defaultServerAvailable: installationManagedRadiusSetup?.default_server_available,
    }),
  );

  const installationPppoeTargetSummary = $derived.by(() => {
    if (installationPppoeTarget === 'managed_radius') {
      return (
        installationManagedRadiusSetup?.server_name ||
        installationManagedRadiusSetup?.assignment_server_name ||
        installationManagedRadiusSetup?.radius_host ||
        tr('admin.network.installations.target_managed_radius', 'Managed RADIUS')
      );
    }
    return installationPppoeRouterLabel;
  });

  async function saveInstallationPppoe() {
    const row = activeRow;
    const subscription = installationSubscription;
    const mapping = installationPppoeMapping;
    const routerId =
      subscription?.router_id || row?.router_id || installationPppoeAccount?.router_id || mapping?.router_id || '';
    const packageId =
      subscription?.package_id || row?.package_id || installationPppoeAccount?.package_id || mapping?.package_id || '';
    if (!row) {
      toast.error('Work order context is not ready yet');
      return;
    }
    if (!routerId) {
      toast.error('Internet service does not have router assigned yet');
      return;
    }
    if (!mapping?.router_profile_name) {
      toast.error('Router profile mapping for this internet package is not configured');
      return;
    }
    if (!installationPppoeUsername.trim()) {
      toast.error(tr('admin.network.installations.username_required', 'Username is required'));
      return;
    }
    if (!installationPppoeAccount && !installationPppoePassword) {
      toast.error(tr('admin.network.installations.password_required', 'Password is required'));
      return;
    }
    if (installationPppoeTarget === 'managed_radius' && !installationManagedRadiusSetup?.configured) {
      toast.error(
        tr(
          'admin.network.installations.managed_radius_not_configured',
          'Managed RADIUS is not configured for this router yet',
        ),
      );
      return;
    }

    savingInstallationPppoe = true;
    try {
      let account = installationPppoeAccount;
      const trimmedComment = installationPppoeComment.trim() || null;
      const selectedSource = installationPppoeTarget;

      if (!account) {
        account = await api.pppoe.accounts.create({
          router_id: routerId,
          customer_id: row.customer_id,
          location_id: row.location_id,
          username: installationPppoeUsername.trim(),
          password: installationPppoePassword,
          package_id: packageId || null,
          router_profile_name: mapping.router_profile_name,
          address_pool: mapping.address_pool ?? null,
          comment: trimmedComment,
          account_source: selectedSource,
        });
        toast.success(
          selectedSource === 'managed_radius'
            ? tr(
                'admin.network.installations.test_account_created_radius',
                'Test account created for RADIUS',
              )
            : tr('admin.network.installations.pppoe_created', 'PPPoE account created'),
        );
      } else {
        account = await api.pppoe.accounts.update(account.id, {
          username: installationPppoeUsername.trim(),
          password: installationPppoePassword || undefined,
          package_id: packageId || null,
          router_profile_name: mapping.router_profile_name,
          address_pool: mapping.address_pool ?? null,
          comment: trimmedComment,
          account_source: selectedSource,
        });
        toast.success(
          tr('admin.network.installations.test_account_updated', 'Test account updated'),
        );
      }

      installationPppoeAccount = account;
      installationPppoePassword = '';
      const applied = await api.pppoe.accounts.apply(account.id);
      installationPppoeAccount = applied;
      checkPppoe = true;
      await savePlan();
      toast.success(
        selectedSource === 'managed_radius'
          ? tr(
              'admin.network.installations.test_account_applied_radius',
              'Test account applied to Managed RADIUS',
            )
          : tr('admin.network.installations.pppoe_applied', 'PPPoE account applied to router'),
      );
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save test account');
    } finally {
      savingInstallationPppoe = false;
    }
  }

  async function applyInstallationPppoe() {
    if (!installationPppoeAccount) return;
    savingInstallationPppoe = true;
    try {
      const applied = await api.pppoe.accounts.apply(installationPppoeAccount.id);
      installationPppoeAccount = applied;
      checkPppoe = true;
      await savePlan();
      toast.success(
        installationPppoeAccount.account_source === 'managed_radius'
          ? tr(
              'admin.network.installations.test_account_applied_radius',
              'Test account applied to Managed RADIUS',
            )
          : tr('admin.network.installations.pppoe_applied', 'PPPoE account applied to router'),
      );
    } catch (e: any) {
      toast.error(e?.message || 'Failed to apply PPPoE account');
    } finally {
      savingInstallationPppoe = false;
    }
  }

  function parsePhotoIdsFromNotes(notes: string | null | undefined): string[] {
    if (!notes) return [];
    const ids = new Set<string>();
    const regex = /\/storage\/files\/([0-9a-fA-F-]{8,})\/content/g;
    let match: RegExpExecArray | null = null;
    while ((match = regex.exec(notes)) !== null) {
      if (match[1]) ids.add(match[1]);
    }
    return Array.from(ids);
  }

  function getStorageContentUrl(fileId: string) {
    const API_BASE = getApiBaseUrl();
    const authParam = $token ? `?token=${encodeURIComponent($token)}` : '';
    return `${API_BASE}/storage/files/${fileId}/content${authParam}`;
  }

  function buildInstallationPhotosNote() {
    if (installationPhotos.length === 0) return '';
    const lines = installationPhotos.map((file) => {
      const url = getStorageContentUrl(file.id);
      const name = file.original_name || file.name || file.id;
      return `- ${name}: ${url}`;
    });
    return `Installation photos:\n${lines.join('\n')}`;
  }

  async function uploadInstallationPhotos(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const files = Array.from(input.files || []);
    if (files.length === 0) return;

    uploadingPhotos = true;
    try {
      for (const file of files) {
        if (!file.type.startsWith('image/')) continue;
        const uploaded = await api.storage.uploadFile(file);
        if (!installationPhotos.some((x) => x.id === uploaded.id)) {
          installationPhotos = [...installationPhotos, uploaded];
        }
      }
      toast.success(
        tr('admin.network.installations.photos_uploaded', 'Installation photos uploaded'),
      );
    } catch (e: any) {
      toast.error(
        e?.message ||
          tr(
            'admin.network.installations.photos_upload_failed',
            'Failed to upload installation photos',
          ),
      );
    } finally {
      uploadingPhotos = false;
      input.value = '';
    }
  }

  function removeInstallationPhoto(fileId: string) {
    installationPhotos = installationPhotos.filter((x) => x.id !== fileId);
  }

  function openCableDesigner() {
    showCableMapDrawer = true;
  }

  async function handleCableMapSaved() {
    await loadAll();
  }

  const onsiteTaskDefs = [
    { key: 'cable', title: 'Cable installed', desc: 'Drop cable + termination complete.' },
    { key: 'ont', title: 'ONT installed', desc: 'Power and signal indicator normal.' },
    { key: 'pppoe', title: 'PPPoE configured', desc: 'Account applied and authenticated.' },
    { key: 'speed', title: 'Speed test passed', desc: 'Measured speed meets package threshold.' },
  ] as const;

  function getOnsiteTaskChecked(index: number) {
    if (index === 0) return checkCable;
    if (index === 1) return checkOnt;
    if (index === 2) return checkPppoe;
    return checkSpeed;
  }

  function setOnsiteTaskChecked(index: number, checked: boolean) {
    if (index === 0) checkCable = checked;
    else if (index === 1) checkOnt = checked;
    else if (index === 2) checkPppoe = checked;
    else checkSpeed = checked;
  }

  async function savePlan() {
    const row = activeRow;
    if (!row) return;
    const assigned_to = formAssignee.trim();
    if (!assigned_to) {
      toast.error(tr('admin.network.installations.assign_required', 'Choose assignee first'));
      return;
    }
    busyId = row.id;
    try {
      const note = buildPersistedInstallationNotes();
      await api.workOrders.assign(row.id, {
        assigned_to,
        scheduled_at: formSchedule ? new Date(formSchedule).toISOString() : undefined,
        notes: note,
      });
      toast.success(tr('admin.network.installations.assigned', 'Assigned'));
      await loadAll();
      const refreshed = rows.find((x) => x.id === row.id);
      if (refreshed) openDetail(refreshed);
    } catch (e: any) {
      toast.error(e?.message || 'Assign failed');
    } finally {
      busyId = null;
    }
  }

  function isPlanReady(assignee: string, schedule: string) {
    return assignee.trim().length > 0 && schedule.trim().length > 0;
  }

  function hasAssignee(assignee: string) {
    return assignee.trim().length > 0;
  }

  const checklistDoneCount = $derived(
    [checkCable, checkOnt, checkPppoe, checkSpeed].filter(Boolean).length,
  );
  const checklistTotal = 4;
  const onsiteAutoIndex = $derived.by(() => {
    const idx = onsiteTaskDefs.findIndex((_, i) => !getOnsiteTaskChecked(i));
    return idx >= 0 ? idx : checklistTotal - 1;
  });
  const onsiteActiveIndex = $derived.by(() => {
    const candidate = onsiteFocusIndex ?? onsiteAutoIndex;
    return Math.max(0, Math.min(checklistTotal - 1, candidate));
  });
  const onsiteActiveTask = $derived.by(() => onsiteTaskDefs[onsiteActiveIndex]);
  const isClosedState = $derived(activeRow?.status === 'completed' || activeRow?.status === 'cancelled');
  const canCompleteActive = $derived(activeRow?.status === 'in_progress' && checklistDoneCount === checklistTotal);
  const canSaveAssignStep = $derived(activeRow?.status === 'pending' && hasAssignee(formAssignee));
  const canSaveScheduleStep = $derived(activeRow?.status === 'pending' && isPlanReady(formAssignee, formSchedule));
  const canStartActive = $derived(
    activeRow?.status === 'pending' && isPlanReady(formAssignee, formSchedule),
  );
  const effectiveStep = $derived.by(() => {
    if (!activeRow) return 1;
    if (activeRow.status === 'completed' || activeRow.status === 'cancelled') return 4;
    if (activeRow.status === 'in_progress' && checklistDoneCount === checklistTotal) return 4;
    if (activeRow.status === 'in_progress') return 3;
    if (!hasAssignee(formAssignee)) return 1;
    return 2;
  });
  const isAwaitingFirstPayment = $derived.by(() => {
    if (!activeRow) return false;
    return (
      activeRow.status === 'completed' &&
      (activeRow.subscription_status === 'pending_installation' ||
        activeRow.subscription_status === 'suspended') &&
      !activeRow.subscription_starts_at
    );
  });
  const isGraceActive = $derived.by(
    () => activeRow?.status === 'completed' && activeRow.subscription_status === 'grace_active',
  );
  const canCreateMissingInvoice = $derived.by(() =>
    shouldAllowInstallationInvoiceCreation({
      workOrderStatus: activeRow?.status,
      subscriptionStatus: activeRow?.subscription_status,
      hasCustomerPackageInvoice: activeRow?.has_customer_package_invoice,
    }),
  );
  const subscriptionStatusLabel = $derived.by(() =>
    formatSubscriptionStatus(activeRow?.subscription_status || ''),
  );
  const subscriptionGraceDeadlineLabel = $derived.by(() =>
    activeRow?.subscription_grace_until ? formatDateTime(activeRow.subscription_grace_until) : '-',
  );
  const currentFocusTitle = $derived.by(() => {
    if (!activeRow) return '';
    if (activeRow.status === 'completed') {
      return isGraceActive
        ? tr('admin.network.installations.focus_grace_title', 'Service is temporarily active')
        : tr('admin.network.installations.focus_done_title', 'Installation is completed');
    }
    if (activeRow.status === 'cancelled') {
      return tr('admin.network.installations.focus_cancelled_title', 'Work order is cancelled');
    }
    if (activeRow.status === 'pending' && effectiveStep === 1) {
      return tr('admin.network.installations.focus_assign_title', 'Assign technician');
    }
    if (activeRow.status === 'pending' && effectiveStep === 2) {
      return tr('admin.network.installations.focus_schedule_title', 'Lock installation schedule');
    }
    if (activeRow.status === 'in_progress' && onsiteActiveTask?.key === 'pppoe') {
      return tr('admin.network.installations.focus_test_title', 'Test customer internet access');
    }
    if (activeRow.status === 'in_progress' && effectiveStep === 4) {
      return tr('admin.network.installations.focus_finish_title', 'Finish and confirm service outcome');
    }
    return tr('admin.network.installations.focus_onsite_title', 'Complete onsite installation tasks');
  });
  const currentFocusHint = $derived.by(() => {
    if (!activeRow) return '';
    if (activeRow.status === 'completed') {
      return isGraceActive
        ? tr('admin.network.installations.focus_grace_hint', 'Customer can use the service for now. Billing will auto-suspend it if the first invoice remains unpaid after the deadline.')
        : tr('admin.network.installations.focus_done_hint', 'No technician action is required unless billing or reopen follow-up is needed.');
    }
    if (activeRow.status === 'cancelled') {
      return tr('admin.network.installations.focus_cancelled_hint', 'This request stays closed until an admin decides to reopen it.');
    }
    if (activeRow.status === 'pending' && effectiveStep === 1) {
      return tr('admin.network.installations.focus_assign_hint', 'Choose who will own this visit so the rest of the flow stays on one technician.');
    }
    if (activeRow.status === 'pending' && effectiveStep === 2) {
      return tr('admin.network.installations.focus_schedule_hint', 'Set the exact visit time before starting onsite work.');
    }
    if (activeRow.status === 'in_progress' && onsiteActiveTask?.key === 'pppoe') {
      return tr('admin.network.installations.focus_test_hint', 'Use the package mapping values automatically, then verify the account really connects.');
    }
    if (activeRow.status === 'in_progress' && effectiveStep === 4) {
      return tr('admin.network.installations.focus_finish_hint', 'Once completed, the service will either enter grace-active or become fully active depending on payment state.');
    }
    return tr('admin.network.installations.focus_onsite_hint', 'Move through one onsite task at a time so handover and proof stay clean.');
  });

  async function startFromDetail() {
    if (!activeRow) return;
    if (!isPlanReady(formAssignee, formSchedule)) {
      toast.error(tr('admin.network.installations.plan_required', 'Set assignee and schedule before starting.'));
      return;
    }
    await savePlan();
    const latest = rows.find((x) => x.id === activeRow?.id);
    if (latest) {
      await setStatus(latest, 'start', formNotes);
    }
  }

  async function completeFromDetail() {
    if (!activeRow) return;
    if (checklistDoneCount !== checklistTotal) {
      toast.error(tr('admin.network.installations.checklist_required', 'Complete all checklist items before activation.'));
      return;
    }
    await setStatus(activeRow, 'complete', formNotes);
  }

  async function saveAssignStep() {
    if (!canSaveAssignStep) {
      toast.error(tr('admin.network.installations.assign_required', 'Choose assignee first'));
      return;
    }
    await savePlan();
  }

  async function saveScheduleStep() {
    if (!canSaveScheduleStep) {
      toast.error(tr('admin.network.installations.schedule_required', 'Choose schedule first'));
      return;
    }
    await savePlan();
  }

  function resetToAssignStep() {
    formAssignee = '';
  }

  function assigneeLabel(userId: string) {
    if (!userId) return '-';
    const member = team.find((x) => x.user_id === userId);
    return member?.name || member?.email || userId;
  }

  function focusOnsiteStep(index: number) {
    onsiteFocusIndex = Math.max(0, Math.min(checklistTotal - 1, index));
  }

  function goPrevOnsiteStep() {
    focusOnsiteStep(onsiteActiveIndex - 1);
  }

  function goNextOnsiteStep() {
    focusOnsiteStep(onsiteActiveIndex + 1);
  }

  async function markActiveOnsiteStepDone() {
    setOnsiteTaskChecked(onsiteActiveIndex, true);
    await savePlan();
    if (onsiteActiveIndex < checklistTotal - 1) {
      focusOnsiteStep(onsiteActiveIndex + 1);
    }
  }

  function tr(key: string, fallback: string) {
    const value = $t(key);
    return value && value !== key ? value : fallback;
  }

  function formatSubscriptionStatus(status: string) {
    const normalized = status.trim().toLowerCase();
    if (normalized === 'grace_active') {
      return tr('admin.network.installations.status_grace_active', 'Grace Active');
    }
    if (normalized === 'pending_installation') {
      return tr('admin.network.installations.status_pending_installation', 'Pending Installation');
    }
    if (normalized === 'suspended') {
      return tr('common.suspended', 'Suspended');
    }
    if (normalized === 'active') {
      return tr('common.active', 'Active');
    }
    if (normalized === 'cancelled') {
      return tr('common.cancelled', 'Cancelled');
    }
    return status || '-';
  }

  async function createInvoiceFromDetail() {
    if (!activeRow || creatingInvoiceId) return;
    creatingInvoiceId = activeRow.id;
    try {
      const invoice = await api.payment.createInvoiceForInstallationWorkOrder(activeRow.id);
      toast.success(
        tr(
          'admin.network.installations.invoice_created',
          `Invoice created: ${invoice.invoice_number}`,
        ),
      );
      await loadAll();
      const refreshed = rows.find((x) => x.id === activeRow?.id);
      if (refreshed) {
        openDetail(refreshed);
      }
    } catch (e: any) {
      toast.error(
        e?.message || tr('admin.network.installations.invoice_create_failed', 'Failed to create invoice'),
      );
    } finally {
      creatingInvoiceId = null;
    }
  }

  async function approveRescheduleFromDetail() {
    if (!activeRow || !rescheduleRequest || !canReviewReschedule || rescheduleDecisionBusy) return;
    const rowId = activeRow.id;
    rescheduleDecisionBusy = true;
    try {
      await api.workOrders.approveReschedule(rowId, {
        scheduled_at: rescheduleOverrideAt
          ? new Date(rescheduleOverrideAt).toISOString()
          : undefined,
        notes: rescheduleDecisionNotes.trim() || undefined,
      });
      toast.success(
        tr(
          'admin.network.installations.reschedule_approved',
          'Reschedule request approved',
        ),
      );
      await loadAll();
      const refreshed = rows.find((x) => x.id === rowId);
      if (refreshed) openDetail(refreshed);
    } catch (e: any) {
      toast.error(e?.message || 'Failed to approve reschedule request');
    } finally {
      rescheduleDecisionBusy = false;
    }
  }

  async function rejectRescheduleFromDetail() {
    if (!activeRow || !rescheduleRequest || !canReviewReschedule || rescheduleDecisionBusy) return;
    const rowId = activeRow.id;
    const notes = rescheduleDecisionNotes.trim();
    if (notes.length < 5) {
      toast.error(
        tr(
          'admin.network.installations.reschedule_reject_reason_required',
          'Add rejection reason first',
        ),
      );
      return;
    }
    rescheduleDecisionBusy = true;
    try {
      await api.workOrders.rejectReschedule(rowId, { notes });
      toast.success(
        tr(
          'admin.network.installations.reschedule_rejected',
          'Reschedule request rejected',
        ),
      );
      await loadAll();
      const refreshed = rows.find((x) => x.id === rowId);
      if (refreshed) openDetail(refreshed);
    } catch (e: any) {
      toast.error(e?.message || 'Failed to reject reschedule request');
    } finally {
      rescheduleDecisionBusy = false;
    }
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
                    {#if canTakeRow(row)}
                      <button
                        class="btn ghost"
                        onclick={(e) => {
                          e.stopPropagation();
                          claimWorkOrder(row);
                        }}
                        disabled={busyId === row.id}
                      >
                        {tr('common.take', 'Take')}
                      </button>
                    {/if}
                    {#if canReleaseRow(row)}
                      <button
                        class="btn ghost"
                        onclick={(e) => {
                          e.stopPropagation();
                          releaseWorkOrder(row);
                        }}
                        disabled={busyId === row.id}
                      >
                        {tr('common.release', 'Release')}
                      </button>
                    {/if}
                    <button
                      class="btn"
                      onclick={(e) => {
                        e.stopPropagation();
                        setStatus(row, 'start');
                      }}
                      disabled={busyId === row.id || !canOperateRow(row) || !isPlanReady(row.assigned_to || '', row.scheduled_at || '')}
                    >
                      {tr('common.start', 'Start')}
                    </button>
                  {/if}
                  {#if $can('manage', 'work_orders') && isAdminOwner && row.status !== 'completed' && row.status !== 'cancelled'}
                    <button class="btn danger" onclick={(e) => { e.stopPropagation(); openCancelDialog(row); }} disabled={busyId === row.id}>
                      {tr('common.cancel', 'Cancel')}
                    </button>
                  {/if}
                  {#if $can('manage', 'work_orders') && row.status === 'cancelled'}
                    <button
                      class="btn ghost"
                      onclick={(e) => {
                        e.stopPropagation();
                        setStatus(row, 'reopen');
                      }}
                      disabled={busyId === row.id}
                    >
                      {tr('common.reopen', 'Reopen')}
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
        <h2>{tr('admin.network.installations.details_title', 'Installation Details')}</h2>
        <button class="btn ghost" onclick={closeDetail}>✕</button>
      </div>

      <div class="step-flow">
        {#if activeRow.status === 'in_progress'}
          <div class:active-step={true}>1. {tr('admin.network.installations.step_assign', 'Assign')}</div>
          <div class:active-step={true}>2. {tr('admin.network.installations.step_schedule', 'Schedule')}</div>
          <div class:done-step={checkCable} class:active-step={!checkCable && onsiteActiveIndex === 0}>
            3. Cable
          </div>
          <div class:done-step={checkOnt} class:active-step={!checkOnt && onsiteActiveIndex === 1}>
            4. ONT
          </div>
          <div class:done-step={checkPppoe} class:active-step={!checkPppoe && onsiteActiveIndex === 2}>
            5. PPPoE
          </div>
          <div class:done-step={checkSpeed} class:active-step={!checkSpeed && onsiteActiveIndex === 3}>
            6. Speed Test
          </div>
          <div class:active-step={checklistDoneCount === checklistTotal}>
            7. {tr('admin.network.installations.step_activate', 'Activate')}
          </div>
        {:else}
          <div class:active-step={effectiveStep >= 1}>1. {tr('admin.network.installations.step_assign', 'Assign')}</div>
          <div class:active-step={effectiveStep >= 2}>2. {tr('admin.network.installations.step_schedule', 'Schedule')}</div>
          <div class:active-step={effectiveStep >= 3}>3. {tr('admin.network.installations.step_onsite', 'On-site & Test')}</div>
          <div class:active-step={effectiveStep >= 4}>4. {tr('admin.network.installations.step_activate', 'Finish')}</div>
        {/if}
      </div>

      <div class="meta-grid">
        <article class="meta-item">
          <span class="meta-label">{tr('common.customer', 'Customer')}</span>
          <strong class="meta-value">{activeRow.customer_name || activeRow.customer_id}</strong>
        </article>
        <article class="meta-item">
          <span class="meta-label">{tr('common.location', 'Location')}</span>
          <strong class="meta-value">{activeRow.location_label || activeRow.location_id}</strong>
        </article>
        <article class="meta-item">
          <span class="meta-label">{tr('common.package', 'Package')}</span>
          <strong class="meta-value">{activeRow.package_name || '-'}</strong>
        </article>
        <article class="meta-item">
          <span class="meta-label">{tr('common.status', 'Status')}</span>
          <span class="meta-value"><span class={statusClass(activeRow.status)}>{activeRow.status}</span></span>
        </article>
        <article class="meta-item">
          <span class="meta-label">{tr('admin.network.installations.subscription_status', 'Service Status')}</span>
          <strong class="meta-value">{subscriptionStatusLabel}</strong>
        </article>
        <article class="meta-item">
          <span class="meta-label">{tr('common.assignee', 'Assignee')}</span>
          <strong class="meta-value">{activeRow.assigned_to_name || '-'}</strong>
        </article>
      </div>

      <section class:grace={isGraceActive} class="focus-panel">
        <div class="focus-copy">
          <span class="focus-kicker">{tr('admin.network.installations.focus_kicker', 'Current Focus')}</span>
          <strong>{currentFocusTitle}</strong>
          <p>{currentFocusHint}</p>
        </div>
        {#if activeRow.status === 'completed' && isGraceActive}
          <div class="focus-chip">
            <span>{tr('admin.network.installations.grace_deadline', 'Grace active until')}</span>
            <strong>{subscriptionGraceDeadlineLabel}</strong>
          </div>
        {:else if activeRow.status === 'in_progress'}
          <div class="focus-chip">
            <span>{tr('admin.network.installations.checklist', 'Installation Checklist')}</span>
            <strong>{checklistDoneCount}/{checklistTotal}</strong>
          </div>
        {/if}
      </section>

      {#if $can('manage', 'work_orders')}
        <section class="wizard-card">
          {#if activeRow.status === 'pending' && effectiveStep === 1}
            <h3>{tr('admin.network.installations.step_assign', 'Assign')}</h3>
            {#if isAdminOwner}
              <p class="step-help">{tr('admin.network.installations.step_assign_help', 'Choose technician first, then continue to scheduling.')}</p>
              <label>
                {tr('common.assignee', 'Assignee')}
                <Select2
                  bind:value={formAssignee}
                  options={assigneeOptions}
                  placeholder={tr('admin.network.installations.assignee_placeholder', 'Select assignee')}
                  searchPlaceholder={tr('common.search', 'Search')}
                  noResultsText={tr('common.no_results', 'No results')}
                  width="100%"
                  disabled={busyId === activeRow.id || !canManageWorkOrders}
                />
              </label>
              {#if canManageWorkOrders && assigneeOptions.length === 0}
                <p class="helper-text">
                  {tr(
                    'admin.network.installations.no_assignable_members',
                    'No eligible installers found. Only Admin/Technician or roles with installation permission are shown.',
                  )}
                </p>
              {/if}
              <label class="notes">
                {tr('common.notes', 'Notes')}
                <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
              </label>
              <div class="modal-actions">
                {#if canReleaseRow(activeRow)}
                  <button
                    class="btn ghost"
                    onclick={() => activeRow && releaseWorkOrder(activeRow)}
                    disabled={busyId === activeRow.id}
                  >
                    {tr('common.release', 'Release')}
                  </button>
                {/if}
                <button class="btn ghost" onclick={saveAssignStep} disabled={busyId === activeRow.id || !canSaveAssignStep}>
                  {tr('admin.network.installations.save_assign', 'Save Assignee')}
                </button>
              </div>
            {:else}
              <p class="step-help">{tr('admin.network.installations.step_take_help', 'Take this work order first, then continue to scheduling.')}</p>
              {#if isUnassigned(activeRow)}
                <div class="modal-actions">
                  <button class="btn ghost" onclick={() => activeRow && claimWorkOrder(activeRow)} disabled={busyId === activeRow.id}>
                    {tr('common.take', 'Take')}
                  </button>
                </div>
              {:else if isAssignedToCurrentUser(activeRow)}
                <p class="helper-text">{tr('admin.network.installations.already_taken_by_you', 'You already took this work order. Continue to Schedule step.')}</p>
              {:else}
                <p class="helper-text">{tr('admin.network.installations.taken_by_other', 'This work order has been taken by another technician.')}</p>
              {/if}
            {/if}
          {:else if activeRow.status === 'pending' && effectiveStep === 2}
            <h3>{tr('admin.network.installations.step_schedule', 'Schedule')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_schedule_help', 'Set installation date/time, then start work order.')}</p>
            {#if rescheduleLoading}
              <p class="helper-text">{tr('common.loading', 'Loading...')}</p>
            {:else if rescheduleRequest}
              <div class="reschedule-request-card">
                <div class="reschedule-request-head">
                  <strong>{tr('admin.network.installations.reschedule_pending_title', 'Pending reschedule request')}</strong>
                  <span>{formatDateTime(rescheduleRequest.created_at)}</span>
                </div>
                <div class="reschedule-request-grid">
                  <div>
                    <span>{tr('common.requested_by', 'Requested by')}</span>
                    <strong>{rescheduleRequest.requested_by_name || rescheduleRequest.requested_by_email || '-'}</strong>
                  </div>
                  <div>
                    <span>{tr('common.schedule', 'Schedule')}</span>
                    <strong>{formatDateTime(rescheduleRequest.requested_schedule_at)}</strong>
                  </div>
                </div>
                {#if rescheduleRequest.reason}
                  <p>{rescheduleRequest.reason}</p>
                {/if}
                {#if canReviewReschedule}
                  <div class="reschedule-decision-fields">
                    <label>
                      {tr('admin.network.installations.override_schedule_optional', 'Override schedule (optional)')}
                      <input type="datetime-local" bind:value={rescheduleOverrideAt} disabled={rescheduleDecisionBusy} />
                    </label>
                    <label>
                      {tr('common.notes', 'Notes')}
                      <textarea
                        rows="3"
                        bind:value={rescheduleDecisionNotes}
                        placeholder={tr('admin.network.installations.reschedule_decision_notes', 'Decision notes')}
                        disabled={rescheduleDecisionBusy}
                      ></textarea>
                    </label>
                  </div>
                  <div class="modal-actions">
                    <button class="btn ghost" type="button" onclick={approveRescheduleFromDetail} disabled={rescheduleDecisionBusy}>
                      {tr('common.approve', 'Approve')}
                    </button>
                    <button class="btn danger" type="button" onclick={rejectRescheduleFromDetail} disabled={rescheduleDecisionBusy}>
                      {tr('common.reject', 'Reject')}
                    </button>
                  </div>
                {/if}
              </div>
            {/if}
            <div class="assigned-summary">
              <span class="summary-label">{tr('common.assignee', 'Assignee')}</span>
              <strong>{assigneeLabel(formAssignee)}</strong>
              {#if isAdminOwner}
                <button class="btn ghost mini" type="button" onclick={resetToAssignStep}>{tr('common.edit', 'Edit')}</button>
              {/if}
            </div>
            <label>
              {tr('common.schedule', 'Schedule')}
              <input type="datetime-local" bind:value={formSchedule} disabled={busyId === activeRow.id} />
            </label>
            <label class="notes">
              {tr('common.notes', 'Notes')}
              <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
            </label>
            <div class="modal-actions">
              <button class="btn ghost" onclick={saveScheduleStep} disabled={busyId === activeRow.id || !canSaveScheduleStep}>
                {tr('admin.network.installations.save_schedule', 'Save Schedule')}
              </button>
              <button class="btn" onclick={startFromDetail} disabled={busyId === activeRow.id || !canStartActive}>
                {tr('common.start', 'Start')}
              </button>
            </div>
          {:else if activeRow.status === 'in_progress' && effectiveStep === 3}
            <h3>{tr('admin.network.installations.step_onsite', 'On-site')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_onsite_help', 'Complete physical installation, test internet access, then finish the visit.')}</p>
            {#if onsiteActiveTask.key === 'cable'}
              <div class="cable-designer-card">
                <div class="cable-designer-copy">
                  <strong>{tr('admin.network.installations.cable_route_title', 'Cable Route')}</strong>
                  <p>
                    {tr(
                      'admin.network.installations.cable_route_desc',
                      'Draw physical cable/link route in Topology Map and save it there.',
                    )}
                  </p>
                </div>
                <button class="btn ghost" type="button" onclick={openCableDesigner}>
                  <Icon name="map-pin" size={14} />
                  {tr('admin.network.installations.open_cable_designer', 'Draw Cable Route')}
                </button>
              </div>
              {#if showCableMapDrawer}
                <div class="cable-map-drawer">
                  <div class="cable-map-head">
                    <strong>{tr('admin.network.installations.cable_map_inline_title', 'Cable Route Designer')}</strong>
                    <button class="btn ghost mini" type="button" onclick={() => (showCableMapDrawer = false)}>
                      {tr('common.close', 'Close')}
                    </button>
                  </div>
                  <InstallationCableMap
                    workOrderId={activeRow.id}
                    customerId={activeRow.customer_id}
                    locationId={activeRow.location_id}
                    preferredTargetNodeId={activeRow.selected_node_id}
                    on:saved={handleCableMapSaved}
                  />
                </div>
              {/if}
            {/if}
            {#if onsiteActiveTask.key === 'pppoe'}
              <section class="pppoe-install-card">
                <div class="pppoe-install-head">
                  <div>
                    <strong>{tr('admin.network.installations.internet_test_title', 'Internet Test')}</strong>
                    <p>{tr('admin.network.installations.internet_test_help', 'Technician only enters username and password. Router, profile, and pool follow the active internet package mapping.')}</p>
                  </div>
                  {#if installationPppoeAccount}
                    <span class="status progress">{tr('admin.network.installations.internet_test_configured', 'Configured')}</span>
                  {/if}
                </div>

                {#if loadingInstallationPppoe}
                  <p class="helper-text">{tr('common.loading', 'Loading...')}</p>
                {:else if !installationSubscription}
                  <p class="helper-text">Subscription internet untuk work order ini belum ditemukan.</p>
                {:else}
                  <div class="form-grid two-col compact">
                    <label class="summary-field">
                      {tr('admin.network.installations.pppoe_username', 'Username')}
                      <input class="input" bind:value={installationPppoeUsername} placeholder="pppoe username" />
                    </label>
                    <label class="summary-field">
                      {tr('admin.network.installations.pppoe_password', 'Password')}
                      <input class="input" type="password" bind:value={installationPppoePassword} placeholder={installationPppoeAccount ? tr('admin.network.installations.password_keep_existing_placeholder', 'Leave blank to keep current password') : 'pppoe password'} />
                    </label>
                  </div>

                  {#if installationPppoeTargetOptions.length > 1}
                    <div class="field">
                      <div class="field-label">{tr('admin.network.installations.provision_to', 'Provision to')}</div>
                      <select class="input" bind:value={installationPppoeTarget}>
                        {#each installationPppoeTargetOptions as option (option.value)}
                          <option value={option.value} disabled={option.disabled}>
                            {option.label}
                          </option>
                        {/each}
                      </select>
                      {#if installationManagedRadiusHint}
                        <p class="helper-text">{tr(
                          installationManagedRadiusLoadError
                            ? 'admin.network.installations.managed_radius_load_failed'
                            : installationManagedRadiusSetup?.plan_upgrade_required
                              ? 'admin.network.installations.managed_radius_plan_required'
                              : installationManagedRadiusSetup?.tenant_has_active_assignment === false &&
                                  installationManagedRadiusSetup?.default_server_available
                                ? 'admin.network.installations.managed_radius_assignment_inactive'
                                : installationManagedRadiusSetup?.tenant_has_active_assignment &&
                                    installationManagedRadiusSetup?.can_create_mapping
                                  ? 'admin.network.installations.managed_radius_mapping_inactive'
                                  : 'admin.network.installations.managed_radius_not_configured',
                          installationManagedRadiusHint,
                        )}</p>
                      {/if}
                    </div>
                  {/if}

                  <label class="notes">
                    {tr('admin.network.installations.pppoe_comment', 'Comment')}
                    <input class="input" bind:value={installationPppoeComment} placeholder="Optional PPPoE comment" />
                  </label>

                  {#if installationManagedRadiusSetup?.configured}
                    <p class="helper-text">
                      {tr(
                        'admin.network.installations.managed_radius_ready_hint',
                        'Managed RADIUS is ready on this router. Technician can choose local router or RADIUS before applying.',
                      )}
                    </p>
                  {/if}

                  {#if installationPppoeAccount}
                    <div class="pppoe-existing">
                      <span>{tr('admin.network.installations.pppoe_existing', 'Existing PPPoE:')}</span>
                      <strong>{installationPppoeAccount.username}</strong>
                      <span>{installationPppoeAccount.account_source === 'managed_radius' ? 'RADIUS' : 'Router'}</span>
                    </div>
                  {/if}

                  <div class="test-outcome">
                    <span class:ok={!!installationPppoeAccount} class="test-state">
                      {installationPppoeAccount
                        ? installationPppoeAccount.account_source === 'managed_radius'
                          ? tr(
                              'admin.network.installations.radius_ready_state',
                              'RADIUS account is ready for live testing.',
                            )
                          : tr('admin.network.installations.test_ready_state', 'Router account is ready for live testing.')
                        : installationPppoeTarget === 'managed_radius'
                          ? tr(
                              'admin.network.installations.radius_pending_state',
                              'Create the account first, then test live connectivity through RADIUS.',
                            )
                          : tr('admin.network.installations.test_pending_state', 'Create the account first, then test live connectivity from the customer side.')}
                    </span>
                  </div>

                  <div class="modal-actions">
                    {#if !installationPppoeAccount}
                      <button
                        class="btn ghost"
                        type="button"
                        onclick={saveInstallationPppoe}
                        disabled={
                          savingInstallationPppoe ||
                          !(
                            installationSubscription?.router_id ||
                            activeRow?.router_id ||
                            installationPppoeMapping?.router_id
                          ) ||
                          !installationPppoeMapping?.router_profile_name ||
                          !installationPppoeUsername.trim() ||
                          !installationPppoePassword ||
                          (installationPppoeTarget === 'managed_radius' && !installationManagedRadiusSetup?.configured)
                        }
                      >
                        {savingInstallationPppoe
                          ? tr('common.loading', 'Loading...')
                          : installationPppoeTarget === 'managed_radius'
                            ? tr(
                                'admin.network.installations.create_apply_radius',
                                'Create & Apply to RADIUS',
                              )
                            : tr('admin.network.installations.create_and_test', 'Create & Test Connection')}
                      </button>
                    {:else}
                      <button
                        class="btn ghost"
                        type="button"
                        onclick={saveInstallationPppoe}
                        disabled={
                          savingInstallationPppoe ||
                          !installationPppoeUsername.trim() ||
                          (installationPppoeTarget === 'managed_radius' && !installationManagedRadiusSetup?.configured)
                        }
                      >
                        {savingInstallationPppoe
                          ? tr('common.loading', 'Loading...')
                          : installationPppoeTarget === 'managed_radius'
                            ? tr(
                                'admin.network.installations.save_reapply_radius',
                                'Save & Re-apply to RADIUS',
                              )
                            : tr(
                                'admin.network.installations.save_reapply_router',
                                'Save & Re-apply to Router',
                              )}
                      </button>
                      <button
                        class="btn ghost"
                        type="button"
                        onclick={applyInstallationPppoe}
                        disabled={savingInstallationPppoe}
                      >
                        {savingInstallationPppoe
                          ? tr('common.loading', 'Loading...')
                          : installationPppoeAccount.account_source === 'managed_radius'
                            ? tr(
                                'admin.network.installations.apply_existing_radius',
                                'Apply Existing to RADIUS',
                              )
                            : tr('admin.network.installations.apply_test', 'Apply Test to Router')}
                      </button>
                    {/if}
                  </div>
                {/if}
              </section>
            {/if}
            <fieldset class="checklist single-step">
              <legend>
                {tr('admin.network.installations.current_step', 'Current Step')}
                <span class="progress-inline">{onsiteActiveIndex + 1}/{checklistTotal}</span>
              </legend>
              <label class="check-item" class:is-done={getOnsiteTaskChecked(onsiteActiveIndex)}>
                <input
                  type="checkbox"
                  checked={getOnsiteTaskChecked(onsiteActiveIndex)}
                  onchange={(e) =>
                    setOnsiteTaskChecked(
                      onsiteActiveIndex,
                      (e.currentTarget as HTMLInputElement).checked,
                    )}
                />
                <span class="check-indicator">{getOnsiteTaskChecked(onsiteActiveIndex) ? '✓' : ''}</span>
                <span class="check-content">
                  <strong>{onsiteActiveTask.title}</strong>
                  <small>{onsiteActiveTask.desc}</small>
                </span>
              </label>
            </fieldset>

            <section class="photos-card">
              <div class="photos-head">
                <strong>{tr('admin.network.installations.photos_title', 'Installation Photos')}</strong>
                <label class="btn ghost upload-btn">
                  <Icon name="image" size={14} />
                  {uploadingPhotos
                    ? tr('common.loading', 'Loading...')
                    : tr('admin.network.installations.photos_add', 'Add Photos')}
                  <input
                    type="file"
                    accept="image/*"
                    multiple
                    onchange={uploadInstallationPhotos}
                    disabled={uploadingPhotos}
                  />
                </label>
              </div>

              {#if installationPhotos.length > 0}
                <div class="photo-grid">
                  {#each installationPhotos as file}
                    <article class="photo-item">
                      <a href={getStorageContentUrl(file.id)} target="_blank" rel="noreferrer">
                        <img
                          src={getStorageContentUrl(file.id)}
                          alt={file.original_name || file.name || 'Installation photo'}
                          loading="lazy"
                        />
                      </a>
                      <div class="photo-meta">
                        <span title={file.original_name || file.name || file.id}>
                          {file.original_name || file.name || file.id}
                        </span>
                        <button class="btn danger mini" type="button" onclick={() => removeInstallationPhoto(file.id)}>
                          {tr('common.remove', 'Remove')}
                        </button>
                      </div>
                    </article>
                  {/each}
                </div>
              {:else}
                <p class="helper-text">
                  {tr(
                    'admin.network.installations.photos_empty',
                    'No installation photos uploaded yet.',
                  )}
                </p>
              {/if}
            </section>
            <label class="notes">
              {tr('common.notes', 'Notes')}
              <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
            </label>
            <div class="modal-actions stage-actions">
              <button
                class="btn ghost"
                type="button"
                onclick={goPrevOnsiteStep}
                disabled={onsiteActiveIndex === 0}
              >
                {tr('common.previous', 'Previous')}
              </button>
              <button
                class="btn ghost"
                type="button"
                onclick={goNextOnsiteStep}
                disabled={onsiteActiveIndex >= checklistTotal - 1}
              >
                {tr('common.next', 'Next')}
              </button>
              <button
                class="btn"
                type="button"
                onclick={markActiveOnsiteStepDone}
                disabled={getOnsiteTaskChecked(onsiteActiveIndex)}
              >
                {tr('admin.network.installations.mark_done', 'Mark done')}
              </button>
              <button class="btn ghost" onclick={savePlan} disabled={busyId === activeRow.id}>
                {tr('admin.network.installations.save_plan', 'Save Plan')}
              </button>
            </div>
          {:else if activeRow.status === 'in_progress' && effectiveStep === 4}
            <h3>{tr('admin.network.installations.step_activate', 'Finish')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_active_help', 'Checklist complete. Finish installation to start the service state flow.')}</p>
            <div class="activation-ready">
              <div>{tr('admin.network.installations.checklist', 'Installation Checklist')}: <strong>{checklistDoneCount}/{checklistTotal}</strong></div>
              <div>{tr('common.schedule', 'Schedule')}: <strong>{activeRow.scheduled_at ? formatDateTime(activeRow.scheduled_at) : '-'}</strong></div>
            </div>
            <label class="notes">
              {tr('common.notes', 'Notes')}
              <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
            </label>
            <div class="modal-actions stage-actions">
              <button class="btn success" onclick={completeFromDetail} disabled={busyId === activeRow.id || !canCompleteActive}>
                {tr('common.complete', 'Complete')}
              </button>
            </div>
          {:else if isClosedState}
            <h3>{tr('admin.network.installations.final_state', 'Final State')}</h3>
            <p class="step-help">
              {activeRow.status === 'completed'
                ? isGraceActive
                  ? tr(
                      'admin.network.installations.final_grace_active',
                      'Installation is complete. Service is temporarily active during grace period.',
                    )
                  : isAwaitingFirstPayment
                  ? activeRow.has_customer_package_invoice
                    ? tr(
                        'admin.network.installations.final_waiting_payment_invoice_exists',
                        'Installation is complete. First invoice already exists and service is waiting payment before activation.',
                      )
                    : tr(
                        'admin.network.installations.final_waiting_payment',
                        'Installation is complete. Service is waiting first payment before activation.',
                      )
                  : tr('admin.network.installations.final_completed', 'Installation has been completed and service is active.')
                : tr('admin.network.installations.final_cancelled', 'Installation has been cancelled.')}
            </p>
            {#if activeRow.status === 'completed' && isGraceActive}
              <div class="activation-ready">
                <div>{tr('admin.network.installations.grace_deadline', 'Grace active until')}: <strong>{subscriptionGraceDeadlineLabel}</strong></div>
                <div>{tr('admin.network.installations.grace_followup', 'If the first invoice is still unpaid after this deadline, service will be suspended automatically.')}</div>
              </div>
            {/if}
            {#if canCreateMissingInvoice}
              <div class="modal-actions stage-actions">
                <button
                  class="btn ghost"
                  type="button"
                  onclick={createInvoiceFromDetail}
                  disabled={creatingInvoiceId === activeRow.id}
                >
                  <Icon name="file-plus" size={14} />
                  {creatingInvoiceId === activeRow.id
                    ? tr('common.loading', 'Loading...')
                    : tr('admin.network.installations.create_invoice', 'Create payment invoice')}
                </button>
              </div>
            {/if}
            {#if activeRow.status === 'cancelled'}
              <label class="notes">
                {tr('common.notes', 'Notes')}
                <textarea rows="3" bind:value={formNotes} placeholder={tr('admin.network.installations.reopen_notes', 'Optional note before reopening work order')}></textarea>
              </label>
              <div class="modal-actions">
                <button class="btn ghost" onclick={() => activeRow && setStatus(activeRow, 'reopen', formNotes)} disabled={busyId === activeRow.id}>
                  {tr('common.reopen', 'Reopen')}
                </button>
              </div>
            {/if}
          {/if}
        </section>
      {/if}

      {#if canReadAuditLogs}
        <div class="history">
          <h3>{tr('admin.network.installations.timeline', 'Work Order Timeline')}</h3>
          {#if timelineLoading}
            <p class="helper-text">{tr('common.loading', 'Loading...')}</p>
          {:else if timelineRows.length === 0}
            <p class="helper-text">{tr('common.no_data', 'No data')}</p>
          {:else}
            <div class="timeline-list">
              {#each timelineRows as log}
                <article class="timeline-item">
                  <div class="timeline-head">
                    <strong>{log.action}</strong>
                    <span>{formatDateTime(log.created_at)}</span>
                  </div>
                  <div class="timeline-meta">
                    <span>{log.user_name || log.user_email || log.user_id || '-'}</span>
                    {#if log.ip_address}
                      <span>{log.ip_address}</span>
                    {/if}
                  </div>
                  {#if log.details}
                    <p>{log.details}</p>
                  {/if}
                </article>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

{#if cancelDialogOpen && cancelTarget}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeCancelDialog();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') closeCancelDialog();
    }}
  >
    <div class="modal cancel-modal">
      <div class="modal-head">
        <h2>{tr('common.cancel', 'Cancel')} Work Order</h2>
        <button class="btn ghost" onclick={closeCancelDialog}>✕</button>
      </div>
      <p class="step-help">
        {tr(
          'admin.network.installations.cancel_reason_required',
          'Cancellation reason is required (minimum 10 characters).',
        )}
      </p>
      <div class="meta-grid">
        <div><strong>{tr('common.customer', 'Customer')}:</strong> {cancelTarget.customer_name || cancelTarget.customer_id}</div>
        <div><strong>{tr('common.location', 'Location')}:</strong> {cancelTarget.location_label || cancelTarget.location_id}</div>
      </div>
      <label class="notes">
        {tr('common.notes', 'Notes')}
        <textarea
          rows="4"
          bind:value={cancelReason}
          placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}
        ></textarea>
      </label>
      <div class="modal-actions">
        <button class="btn ghost" onclick={closeCancelDialog} disabled={busyId === cancelTarget.id}>
          {tr('common.close', 'Close')}
        </button>
        <button
          class="btn danger"
          onclick={confirmCancelFromDialog}
          disabled={busyId === cancelTarget.id || !hasValidCancelReason(cancelReason)}
        >
          {tr('common.cancel', 'Cancel')}
        </button>
      </div>
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
  .btn.mini {
    padding: 5px 9px;
    font-size: 0.76rem;
    border-radius: 10px;
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
  .cancel-modal {
    width: min(640px, 100%);
  }
  .modal-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    position: sticky;
    top: -16px;
    z-index: 5;
    margin: -16px -16px 0;
    padding: 16px;
    background: rgba(11, 18, 33, 0.94);
    backdrop-filter: blur(10px);
    border-bottom: 1px solid rgba(51, 65, 85, 0.72);
  }
  .modal h2 {
    margin: 0;
    font-size: 1.2rem;
  }
  .step-flow {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 8px;
  }
  .step-flow > div {
    border: 1px solid #334155;
    border-radius: 16px;
    padding: 10px 12px;
    color: #9fb0cc;
    font-size: 0.82rem;
    text-align: center;
    font-weight: 700;
  }
  .step-flow > div.active-step {
    border-color: rgba(99, 102, 241, 0.6);
    background: rgba(99, 102, 241, 0.14);
    color: #dbeafe;
  }
  .step-flow > div.done-step {
    border-color: rgba(34, 197, 94, 0.45);
    background: rgba(22, 101, 52, 0.22);
    color: #d1fae5;
  }
  .meta-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px 14px;
  }
  .focus-panel {
    border: 1px solid rgba(96, 165, 250, 0.28);
    border-radius: 14px;
    background:
      linear-gradient(135deg, rgba(30, 41, 59, 0.96), rgba(15, 23, 42, 0.94)),
      radial-gradient(circle at top right, rgba(59, 130, 246, 0.16), transparent 44%);
    padding: 14px 16px;
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }
  .focus-panel.grace {
    border-color: rgba(34, 197, 94, 0.34);
    background:
      linear-gradient(135deg, rgba(15, 37, 28, 0.96), rgba(11, 18, 33, 0.94)),
      radial-gradient(circle at top right, rgba(34, 197, 94, 0.18), transparent 44%);
  }
  .focus-copy {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .focus-kicker {
    color: #93c5fd;
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .focus-copy strong {
    color: #eff6ff;
    font-size: 1rem;
  }
  .focus-copy p {
    margin: 0;
    color: #c6d4ea;
    font-size: 0.88rem;
    line-height: 1.45;
    max-width: 62ch;
  }
  .focus-chip {
    min-width: 190px;
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 12px;
    background: rgba(15, 23, 42, 0.7);
    padding: 10px 12px;
    display: grid;
    gap: 4px;
  }
  .focus-chip span {
    color: #93c5fd;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 700;
  }
  .focus-chip strong {
    color: #f8fafc;
    font-size: 0.95rem;
  }
  .meta-item {
    border: 1px solid #2b3854;
    border-radius: 10px;
    background: #0f1728;
    padding: 10px 12px;
    display: grid;
    gap: 5px;
  }
  .meta-label {
    color: #9fb0cc;
    font-size: 0.75rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-weight: 700;
  }
  .meta-value {
    color: #e5edff;
    font-size: 0.96rem;
    font-weight: 800;
    min-height: 20px;
  }
  .wizard-card {
    border: 1px solid #2b3a5b;
    border-radius: 12px;
    background: #0e1729;
    padding: 16px;
    display: grid;
    gap: 12px;
  }
  .wizard-card h3 {
    margin: 0;
    font-size: 1rem;
  }
  .assigned-summary {
    border: 1px solid #334766;
    border-radius: 10px;
    padding: 10px 12px;
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    background: #0b1221;
  }
  .reschedule-request-card {
    border: 1px solid rgba(245, 158, 11, 0.38);
    border-radius: 10px;
    background: rgba(120, 53, 15, 0.18);
    padding: 12px;
    display: grid;
    gap: 10px;
  }
  .reschedule-request-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 0.84rem;
    color: #fbbf24;
  }
  .reschedule-request-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px 12px;
  }
  .reschedule-request-grid > div {
    display: grid;
    gap: 4px;
  }
  .reschedule-request-grid span {
    font-size: 0.75rem;
    color: #fcd34d;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 700;
  }
  .reschedule-request-grid strong {
    color: #fde68a;
    font-size: 0.92rem;
  }
  .reschedule-request-card p {
    margin: 0;
    font-size: 0.86rem;
    color: #fde68a;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .reschedule-decision-fields {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .reschedule-decision-fields label:last-child {
    grid-column: 1 / -1;
  }
  .summary-label {
    color: #9fb0cc;
    font-size: 0.8rem;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    font-weight: 700;
  }
  .step-help {
    margin: 0;
    font-size: 0.9rem;
    color: #9fb0cc;
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
  .checklist.single-step {
    padding: 12px;
  }
  .progress-inline {
    margin-left: 8px;
    font-size: 0.78rem;
    color: #93c5fd;
    font-weight: 700;
  }
  .activation-ready {
    border: 1px dashed #3b5276;
    border-radius: 12px;
    padding: 12px;
    display: grid;
    gap: 8px;
    color: #cfe0ff;
    font-size: 0.9rem;
    background: rgba(15, 23, 42, 0.52);
  }
  .checklist label {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .check-item {
    border: 1px solid #314261;
    background: #0f1728;
    border-radius: 10px;
    padding: 10px;
    cursor: pointer;
    gap: 10px !important;
    align-items: flex-start !important;
    transition: border-color 140ms ease, background 140ms ease;
  }
  .check-item:hover {
    border-color: #47608d;
    background: #111d33;
  }
  .check-item input[type='checkbox'] {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
    pointer-events: none;
  }
  .check-indicator {
    width: 22px;
    height: 22px;
    border-radius: 7px;
    border: 1px solid #496087;
    background: #0c1422;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: #0b1a32;
    font-weight: 900;
    line-height: 1;
  }
  .check-content {
    display: grid;
    gap: 3px;
    color: #d9e7ff;
  }
  .check-content strong {
    font-size: 0.96rem;
  }
  .check-content small {
    color: #9eb0cf;
    font-size: 0.8rem;
  }
  .check-item.is-done {
    border-color: rgba(34, 197, 94, 0.44);
    background: rgba(22, 101, 52, 0.2);
  }
  .check-item.is-done .check-indicator {
    border-color: rgba(34, 197, 94, 0.65);
    background: #22c55e;
    color: #06280f;
  }
  .check-item.is-done .check-content strong {
    color: #d1fadf;
  }
  .cable-designer-card {
    border: 1px solid #2d3f61;
    border-radius: 12px;
    background: linear-gradient(135deg, #0c162a, #101c31);
    padding: 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }
  .pppoe-install-card {
    border: 1px solid rgba(59, 130, 246, 0.26);
    border-radius: 14px;
    background:
      linear-gradient(180deg, rgba(11, 23, 41, 0.96), rgba(12, 18, 33, 0.98)),
      radial-gradient(circle at top right, rgba(59, 130, 246, 0.14), transparent 45%);
    padding: 14px;
    display: grid;
    gap: 12px;
  }
  .pppoe-install-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: flex-start;
  }
  .pppoe-install-head p {
    margin: 4px 0 0;
    color: #b7c8e7;
    font-size: 0.86rem;
    max-width: 58ch;
  }
  .pppoe-existing {
    border: 1px solid rgba(34, 197, 94, 0.22);
    border-radius: 12px;
    background: rgba(21, 128, 61, 0.12);
    padding: 10px 12px;
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    color: #d6f5e3;
  }
  .pppoe-existing span:first-child {
    color: #9fd7b2;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }
  .test-outcome {
    display: flex;
    justify-content: flex-start;
  }
  .test-state {
    border: 1px dashed rgba(148, 163, 184, 0.34);
    border-radius: 999px;
    padding: 7px 12px;
    color: #c7d3e7;
    font-size: 0.82rem;
    background: rgba(15, 23, 42, 0.45);
  }
  .test-state.ok {
    border-color: rgba(34, 197, 94, 0.38);
    color: #d4f7df;
    background: rgba(22, 101, 52, 0.2);
  }
  .cable-designer-copy {
    display: grid;
    gap: 4px;
  }
  .cable-designer-copy p {
    margin: 0;
    font-size: 0.85rem;
    color: #9fb0cc;
  }
  .cable-map-drawer {
    margin-top: 10px;
    border: 1px solid #2d3f61;
    border-radius: 10px;
    background: #0a1220;
    overflow: hidden;
  }
  .cable-map-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid #263655;
    background: #0b1629;
  }
  .cable-map-drawer :global(.icm-map) {
    border-top-left-radius: 0;
    border-top-right-radius: 0;
    border-left: 0;
    border-right: 0;
    border-bottom: 0;
  }
  .photos-card {
    border: 1px solid #2d3650;
    border-radius: 10px;
    padding: 10px;
    display: grid;
    gap: 10px;
    background: #0f1626;
  }
  .photos-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .upload-btn {
    position: relative;
    overflow: hidden;
  }
  .upload-btn input[type='file'] {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .photo-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: 8px;
  }
  .photo-item {
    border: 1px solid #2d3650;
    border-radius: 10px;
    background: #0b1221;
    overflow: hidden;
    display: grid;
    gap: 6px;
    padding: 6px;
  }
  .photo-item img {
    width: 100%;
    height: 92px;
    object-fit: cover;
    border-radius: 6px;
    border: 1px solid #2d3650;
    display: block;
    background: #0a1220;
  }
  .photo-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }
  .photo-meta span {
    min-width: 0;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.75rem;
    color: #b8c7e3;
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
  .stage-actions {
    position: sticky;
    bottom: -16px;
    z-index: 4;
    margin: 6px -16px -16px;
    padding: 14px 16px 16px;
    background: linear-gradient(180deg, rgba(11, 18, 33, 0), rgba(11, 18, 33, 0.92) 22%, rgba(11, 18, 33, 0.98));
    backdrop-filter: blur(8px);
    border-top: 1px solid rgba(51, 65, 85, 0.72);
  }
  .history {
    border-top: 1px dashed #33405d;
    padding-top: 10px;
  }
  .history h3 {
    margin: 0 0 8px;
    font-size: 0.95rem;
  }
  .timeline-list {
    display: grid;
    gap: 8px;
  }
  .timeline-item {
    border: 1px solid #2d3650;
    border-radius: 10px;
    padding: 10px;
    background: #0f1626;
    display: grid;
    gap: 4px;
  }
  .timeline-head {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: center;
  }
  .timeline-head strong {
    font-size: 0.9rem;
  }
  .timeline-head span {
    color: #9fb0cc;
    font-size: 0.78rem;
  }
  .timeline-meta {
    display: flex;
    gap: 10px;
    color: #9fb0cc;
    font-size: 0.78rem;
  }
  .timeline-item p {
    margin: 0;
    color: #c9d6ef;
    font-size: 0.85rem;
    white-space: pre-wrap;
    word-break: break-word;
  }
  @media (max-width: 800px) {
    .page-content {
      padding: 16px;
    }
    .stats-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .meta-grid,
    .form-grid,
    .reschedule-request-grid,
    .reschedule-decision-fields {
      grid-template-columns: 1fr;
    }
    .focus-panel,
    .pppoe-install-head,
    .cable-designer-card {
      grid-template-columns: 1fr;
      display: grid;
    }
    .step-flow {
      grid-template-columns: 1fr;
    }
    .step-flow > div {
      text-align: left;
    }
    .modal-head {
      top: -16px;
      align-items: flex-start;
    }
    .modal-head .btn {
      flex-shrink: 0;
    }
    .stage-actions {
      justify-content: stretch;
    }
    .stage-actions .btn {
      flex: 1 1 100%;
      justify-content: center;
    }
  }
</style>
