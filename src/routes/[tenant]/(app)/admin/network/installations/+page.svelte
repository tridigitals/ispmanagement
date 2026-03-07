<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can, token, user } from '$lib/stores/auth';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';
  import {
    api,
    type AuditLog,
    type FileRecord,
    type InstallationWorkOrderView,
    type TeamMember,
    type WorkOrderRescheduleRequestView,
  } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import { formatDateTime } from '$lib/utils/date';
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
    formNotes = '';
    detailOpen = true;
    checkCable = false;
    checkOnt = false;
    checkPppoe = false;
    checkSpeed = false;
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
      const extra = formNotes.trim();
      const checklist = buildChecklistNote();
      const photos = buildInstallationPhotosNote();
      const note = [extra, checklist, photos]
        .filter((part) => part && part.trim().length > 0)
        .join('\n\n');
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
      activeRow.subscription_status === 'suspended' &&
      !activeRow.subscription_starts_at
    );
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

  function markActiveOnsiteStepDone() {
    setOnsiteTaskChecked(onsiteActiveIndex, true);
    if (onsiteActiveIndex < checklistTotal - 1) {
      focusOnsiteStep(onsiteActiveIndex + 1);
    }
  }

  function tr(key: string, fallback: string) {
    const value = $t(key);
    return value && value !== key ? value : fallback;
  }

  async function createInvoiceFromDetail() {
    if (!activeRow || creatingInvoiceId) return;
    creatingInvoiceId = activeRow.id;
    try {
      const invoice = await api.payment.createInvoiceForCustomerSubscription(activeRow.subscription_id);
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
                  {#if $can('manage', 'work_orders') && row.status === 'in_progress'}
                    <button class="btn success" onclick={(e) => { e.stopPropagation(); setStatus(row, 'complete'); }} disabled={busyId === row.id || !canOperateRow(row)}>
                      {tr('common.complete', 'Complete')}
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
          <div class:active-step={effectiveStep >= 3}>3. {tr('admin.network.installations.step_onsite', 'On-site')}</div>
          <div class:active-step={effectiveStep >= 4}>4. {tr('admin.network.installations.step_activate', 'Activate')}</div>
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
          <strong class="meta-value">{activeRow.subscription_status || '-'}</strong>
        </article>
        <article class="meta-item">
          <span class="meta-label">{tr('common.assignee', 'Assignee')}</span>
          <strong class="meta-value">{activeRow.assigned_to_name || '-'}</strong>
        </article>
      </div>

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
            <p class="step-help">{tr('admin.network.installations.step_onsite_help', 'Complete onsite checklist. Progress updates automatically.')}</p>
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
                    initialFromNodeId={activeRow.selected_node_id}
                    on:saved={handleCableMapSaved}
                  />
                </div>
              {/if}
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
            <div class="modal-actions">
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
              <button class="btn success" onclick={completeFromDetail} disabled={busyId === activeRow.id || !canCompleteActive}>
                {tr('common.complete', 'Complete')}
              </button>
            </div>
          {:else if activeRow.status === 'in_progress' && effectiveStep === 4}
            <h3>{tr('admin.network.installations.step_activate', 'Activate')}</h3>
            <p class="step-help">{tr('admin.network.installations.step_active_help', 'Checklist complete. Activate service now.')}</p>
            <div class="activation-ready">
              <div>{tr('admin.network.installations.checklist', 'Installation Checklist')}: <strong>{checklistDoneCount}/{checklistTotal}</strong></div>
              <div>{tr('common.schedule', 'Schedule')}: <strong>{activeRow.scheduled_at ? formatDateTime(activeRow.scheduled_at) : '-'}</strong></div>
            </div>
            <label class="notes">
              {tr('common.notes', 'Notes')}
              <textarea rows="4" bind:value={formNotes} placeholder={tr('admin.network.installations.notes_placeholder', 'Technician notes and onsite findings')}></textarea>
            </label>
            <div class="modal-actions">
              <button class="btn success" onclick={completeFromDetail} disabled={busyId === activeRow.id || !canCompleteActive}>
                {tr('common.complete', 'Complete')}
              </button>
            </div>
          {:else if isClosedState}
            <h3>{tr('admin.network.installations.final_state', 'Final State')}</h3>
            <p class="step-help">
              {activeRow.status === 'completed'
                ? isAwaitingFirstPayment
                  ? tr(
                      'admin.network.installations.final_waiting_payment',
                      'Installation is complete. Service is waiting first payment before activation.',
                    )
                  : tr('admin.network.installations.final_completed', 'Installation has been completed and service is active.')
                : tr('admin.network.installations.final_cancelled', 'Installation has been cancelled.')}
            </p>
            {#if activeRow.status === 'completed' && isAwaitingFirstPayment}
              <div class="modal-actions">
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

      {#if activeRow.notes}
        <div class="history">
          <h3>{tr('admin.network.installations.history', 'Latest Notes')}</h3>
          <pre>{activeRow.notes}</pre>
        </div>
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
    border-radius: 999px;
    padding: 8px 10px;
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
    padding: 14px;
    display: grid;
    gap: 10px;
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
    border: 1px dashed #334766;
    border-radius: 10px;
    padding: 10px;
    display: grid;
    gap: 6px;
    color: #cfe0ff;
    font-size: 0.9rem;
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
    border-radius: 10px;
    background: #0c162a;
    padding: 10px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
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
    .step-flow {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
