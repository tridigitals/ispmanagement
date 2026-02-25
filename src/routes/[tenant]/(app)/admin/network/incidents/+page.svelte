<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import AlertsIncidentsSwitch from '$lib/components/network/AlertsIncidentsSwitch.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import RowActionButtons from '$lib/components/network/RowActionButtons.svelte';
  import IncidentDetailDrawer from '$lib/components/network/IncidentDetailDrawer.svelte';
  import IncidentSimulateDrawer from '$lib/components/network/IncidentSimulateDrawer.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { exportCsvRows, exportExcelRows } from '$lib/utils/tabularExport';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { user, tenant } from '$lib/stores/auth';
  import { get } from 'svelte/store';
  import type { TeamMember } from '$lib/api/client';

  type IncidentRow = {
    id: string;
    tenant_id: string;
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
    first_seen_at?: string;
    acked_at?: string | null;
    acked_by?: string | null;
    last_seen_at: string;
    resolved_at?: string | null;
    updated_at: string;
  };
  type RouterRow = {
    id: string;
    name: string;
    host: string;
    port: number;
    identity?: string | null;
    ros_version?: string | null;
    is_online?: boolean;
    latency_ms?: number | null;
    last_seen_at?: string | null;
    last_error?: string | null;
  };
  type RouterMetricRow = {
    ts: string;
    cpu_load?: number | null;
    rx_bps?: number | null;
    tx_bps?: number | null;
    free_memory_bytes?: number | null;
    total_memory_bytes?: number | null;
  };

  let loading = $state(true);
  let refreshing = $state(false);
  let rows = $state<IncidentRow[]>([]);
  let detailOpen = $state(false);
  let detailLoading = $state(false);
  let detailIncident = $state<IncidentRow | null>(null);
  let detailRouter = $state<RouterRow | null>(null);
  let detailMetric = $state<RouterMetricRow | null>(null);
  let detailSaving = $state(false);
  let selectedOwnerId = $state('');
  let draftNotes = $state('');
  let activeOnly = $state(true);
  let isMobile = $state(false);
  let teamMembers = $state<TeamMember[]>([]);
  let filterAssignee = $state('all');
  let filterStatus = $state('all');
  let filterSeverity = $state('all');
  let filterType = $state('all');
  let filterSort = $state('last_seen_desc');
  let filterFrom = $state('');
  let filterTo = $state('');
  let assignmentEmailEnabled = $state(false);
  let slaWarnMinutes = $state(30);
  let slaBreachMinutes = $state(120);
  let selectedIds = $state<string[]>([]);
  let bulkAssigneeId = $state('');
  let bulkBusy = $state(false);
  let simulateOpen = $state(false);
  let simulateBusy = $state(false);
  let simulateRouters = $state<RouterRow[]>([]);
  let simulateRouterId = $state('');
  let simulateType = $state('offline');
  let simulateSeverity = $state('warning');
  let simulateInterface = $state('');
  let simulateMessage = $state('');
  let exportMenuOpen = $state(false);
  let escalationRunBusy = $state(false);
  let nowMs = $state(Date.now());
  let refreshHandle: any = null;
  let slaTickHandle: any = null;
  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);
  let canUseTenantSettings = $derived($can('read', 'settings') || $can('update', 'settings'));

  const columns = $derived.by(() => [
    { key: 'select', label: '', width: '44px' },
    { key: 'title', label: $t('admin.network.incidents.columns.incident') || 'Incident' },
    { key: 'type', label: $t('admin.network.incidents.columns.type') || 'Type' },
    { key: 'severity', label: $t('admin.network.incidents.columns.severity') || 'Severity' },
    { key: 'status', label: $t('admin.network.incidents.columns.status') || 'Status' },
    { key: 'seen', label: $t('admin.network.incidents.columns.seen') || 'Last Seen' },
    { key: 'actions', label: '', align: 'right' as const, width: '140px' },
  ]);
  const incidentTypeOptions = $derived.by(() =>
    Array.from(new Set(rows.map((row) => row.incident_type).filter(Boolean))).sort((a, b) => a.localeCompare(b)),
  );

  const filteredRows = $derived.by(() => {
    const severityWeight = (severity: string) => {
      if (severity === 'critical') return 3;
      if (severity === 'warning') return 2;
      if (severity === 'info') return 1;
      return 0;
    };

    const list = rows.filter((row) => {
      if (filterAssignee === 'unassigned' && row.owner_user_id) return false;
      if (filterAssignee !== 'all' && filterAssignee !== 'unassigned' && row.owner_user_id !== filterAssignee) {
        return false;
      }
      if (filterStatus !== 'all' && row.status !== filterStatus) return false;
      if (filterSeverity !== 'all' && row.severity !== filterSeverity) return false;
      if (filterType !== 'all' && row.incident_type !== filterType) return false;

      const seenTs = new Date(row.last_seen_at).getTime();
      if (Number.isNaN(seenTs)) return false;

      if (filterFrom) {
        const fromTs = new Date(`${filterFrom}T00:00:00`).getTime();
        if (!Number.isNaN(fromTs) && seenTs < fromTs) return false;
      }
      if (filterTo) {
        const toTs = new Date(`${filterTo}T23:59:59.999`).getTime();
        if (!Number.isNaN(toTs) && seenTs > toTs) return false;
      }

      return true;
    });

    list.sort((a, b) => {
      const aLastSeen = new Date(a.last_seen_at).getTime() || 0;
      const bLastSeen = new Date(b.last_seen_at).getTime() || 0;

      if (filterSort === 'last_seen_asc') return aLastSeen - bLastSeen;
      if (filterSort === 'severity_desc') {
        const bySeverity = severityWeight(b.severity) - severityWeight(a.severity);
        if (bySeverity !== 0) return bySeverity;
        return bLastSeen - aLastSeen;
      }
      if (filterSort === 'open_duration_desc') {
        const byOpenMs = incidentOpenMs(b) - incidentOpenMs(a);
        if (byOpenMs !== 0) return byOpenMs;
        return bLastSeen - aLastSeen;
      }
      return bLastSeen - aLastSeen;
    });

    return list;
  });
  const selectedRows = $derived.by(() => rows.filter((r) => selectedIds.includes(r.id)));
  const selectedCount = $derived(selectedIds.length);
  const filteredIds = $derived.by(() => filteredRows.map((r) => r.id));
  const allFilteredSelected = $derived.by(
    () => filteredIds.length > 0 && filteredIds.every((id) => selectedIds.includes(id)),
  );
  const canBulkAckCount = $derived.by(
    () => selectedRows.filter((r) => r.status !== 'ack' && r.status !== 'resolved').length,
  );
  const canBulkResolveCount = $derived.by(
    () => selectedRows.filter((r) => r.status !== 'resolved').length,
  );
  const analytics = $derived.by(() => {
    const open = rows.filter((r) => r.status === 'open').length;
    const ack = rows.filter((r) => r.status === 'ack').length;
    const inProgress = rows.filter((r) => r.status === 'in_progress').length;
    const resolved = rows.filter((r) => r.status === 'resolved').length;

    const mttaSamples = rows
      .filter((r) => r.first_seen_at && r.acked_at)
      .map((r) => new Date(r.acked_at as string).getTime() - new Date(r.first_seen_at as string).getTime())
      .filter((ms) => Number.isFinite(ms) && ms >= 0);

    const mttrSamples = rows
      .filter((r) => r.first_seen_at && r.resolved_at)
      .map((r) => new Date(r.resolved_at as string).getTime() - new Date(r.first_seen_at as string).getTime())
      .filter((ms) => Number.isFinite(ms) && ms >= 0);

    const mtta = mttaSamples.length ? Math.round(mttaSamples.reduce((a, b) => a + b, 0) / mttaSamples.length) : null;
    const mttr = mttrSamples.length ? Math.round(mttrSamples.reduce((a, b) => a + b, 0) / mttrSamples.length) : null;

    const since24h = Date.now() - 24 * 60 * 60 * 1000;
    const typeCounts = new Map<string, number>();
    for (const row of rows) {
      const t = new Date(row.last_seen_at || row.updated_at).getTime();
      if (!Number.isFinite(t) || t < since24h) continue;
      typeCounts.set(row.incident_type, (typeCounts.get(row.incident_type) || 0) + 1);
    }
    let topType = '';
    let topTypeCount = 0;
    for (const [k, v] of typeCounts) {
      if (v > topTypeCount) {
        topType = k;
        topTypeCount = v;
      }
    }

    return { open, ack, inProgress, resolved, mtta, mttr, topType, topTypeCount };
  });

  onMount(() => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }
    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 1024px)');
      const sync = () => (isMobile = mq.matches);
      sync();
      try {
        mq.addEventListener('change', sync);
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
      }
    }
    void load();
    void loadAssignmentEmailSetting();
    void loadSlaSettings();
    if ($can('manage', 'network_routers')) {
      void loadTeamMembers();
    }
    refreshHandle = setInterval(() => void refreshSilent(), 5000);
    slaTickHandle = setInterval(() => {
      nowMs = Date.now();
    }, 30000);

    if (typeof window !== 'undefined') {
      const onKey = (e: KeyboardEvent) => {
        if (e.key === 'Escape' && detailOpen) closeDetail();
        if (e.key === 'Escape' && exportMenuOpen) exportMenuOpen = false;
      };
      window.addEventListener('keydown', onKey);
      return () => window.removeEventListener('keydown', onKey);
    }
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
    if (slaTickHandle) clearInterval(slaTickHandle);
  });

  async function load() {
    loading = true;
    try {
      rows = (await api.mikrotik.incidents.list({ activeOnly, limit: 500 })) as any;
      selectedIds = selectedIds.filter((id) => rows.some((r) => r.id === id));
      const target = get(page).url.searchParams.get('incident');
      if (target && !detailOpen) {
        const found = rows.find((r) => r.id === target);
        if (found) void openDetail(found);
      }
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function refreshSilent() {
    if (refreshing) return;
    refreshing = true;
    try {
      rows = (await api.mikrotik.incidents.list({ activeOnly, limit: 500 })) as any;
      selectedIds = selectedIds.filter((id) => rows.some((r) => r.id === id));
    } catch {
      // ignore background refresh
    } finally {
      refreshing = false;
    }
  }

  async function loadTeamMembers() {
    try {
      teamMembers = await api.team.list();
    } catch {
      // non-blocking
    }
  }

  async function loadAssignmentEmailSetting() {
    if (!canUseTenantSettings) {
      assignmentEmailEnabled = false;
      return;
    }
    try {
      const v = await api.settings.getValue('mikrotik_incident_assignment_email_enabled');
      const norm = (v || '').trim().toLowerCase();
      assignmentEmailEnabled = norm === 'true' || norm === '1' || norm === 'yes' || norm === 'on';
    } catch {
      assignmentEmailEnabled = false;
    }
  }

  async function loadSlaSettings() {
    if (!canUseTenantSettings) {
      slaWarnMinutes = 30;
      slaBreachMinutes = 120;
      return;
    }
    try {
      const [warnRaw, breachRaw] = await Promise.all([
        api.settings.getValue('mikrotik_incident_sla_warn_minutes'),
        api.settings.getValue('mikrotik_incident_sla_breach_minutes'),
      ]);
      const parsedWarn = Number.parseInt((warnRaw || '').trim(), 10);
      const parsedBreach = Number.parseInt((breachRaw || '').trim(), 10);
      const warn = Number.isFinite(parsedWarn) && parsedWarn > 0 ? parsedWarn : 30;
      const breach = Number.isFinite(parsedBreach) && parsedBreach > 0 ? parsedBreach : 120;
      slaWarnMinutes = warn;
      slaBreachMinutes = breach > warn ? breach : warn * 2;
    } catch {
      slaWarnMinutes = 30;
      slaBreachMinutes = 120;
    }
  }

  async function openSimulateDialog() {
    if (simulateRouters.length === 0) {
      try {
        simulateRouters = ((await api.mikrotik.routers.list()) || []) as RouterRow[];
      } catch (e: any) {
        toast.error(e?.message || e);
        return;
      }
    }
    if (!simulateRouterId && simulateRouters.length > 0) {
      simulateRouterId = simulateRouters[0].id;
    }
    simulateOpen = true;
  }

  function closeSimulateDialog() {
    if (simulateBusy) return;
    simulateOpen = false;
  }

  function toggleExportMenu() {
    exportMenuOpen = !exportMenuOpen;
  }

  async function submitSimulateIncident() {
    if (!simulateRouterId) {
      toast.error($t('admin.network.incidents.simulate.select_router') || 'Select router first');
      return;
    }
    simulateBusy = true;
    try {
      await api.mikrotik.incidents.simulate({
        routerId: simulateRouterId,
        incidentType: simulateType,
        severity: simulateSeverity,
        interfaceName: simulateInterface.trim() || null,
        message: simulateMessage.trim() || null,
      });
      toast.success($t('admin.network.incidents.simulate.created') || 'Simulated incident created');
      simulateOpen = false;
      simulateInterface = '';
      simulateMessage = '';
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      simulateBusy = false;
    }
  }

  async function runAutoEscalationNow() {
    if (escalationRunBusy) return;
    escalationRunBusy = true;
    try {
      const res = await api.mikrotik.incidents.runAutoEscalation();
      const count = Number(res?.escalated ?? 0);
      toast.success(
        $t('admin.network.incidents.toasts.auto_escalation_done', { values: { count } }) ||
          `${count} incident(s) escalated`,
      );
      await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      escalationRunBusy = false;
    }
  }

  function typeLabel(tpe: string) {
    if (tpe === 'offline') return $t('admin.network.alerts.types.offline') || 'Offline';
    if (tpe === 'cpu') return $t('admin.network.alerts.types.cpu') || 'CPU';
    if (tpe === 'latency') return $t('admin.network.alerts.types.latency') || 'Latency';
    return tpe;
  }

  function severityLabel(sev: string) {
    if (sev === 'critical') return $t('admin.network.alerts.severity.critical') || 'Critical';
    if (sev === 'warning') return $t('admin.network.alerts.severity.warning') || 'Warning';
    return $t('admin.network.alerts.severity.info') || 'Info';
  }

  async function ack(id: string) {
    try {
      await api.mikrotik.incidents.ack(id);
      toast.success($t('admin.network.alerts.toasts.acked') || 'Alert acknowledged');
      await load();
      if (detailIncident?.id === id) {
        detailIncident = rows.find((r) => r.id === id) || null;
      }
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function resolve(id: string) {
    try {
      await api.mikrotik.incidents.resolve(id);
      toast.success($t('admin.network.alerts.toasts.resolved') || 'Alert resolved');
      await load();
      if (detailIncident?.id === id) {
        detailIncident = rows.find((r) => r.id === id) || null;
      }
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function openRouter(routerId: string) {
    goto($page.url.pathname.replace(/\/admin\/network\/incidents\/?$/, `/admin/network/routers/${routerId}`));
  }

  async function openDetail(item: IncidentRow) {
    detailOpen = true;
    detailLoading = true;
    detailIncident = item;
    detailRouter = null;
    detailMetric = null;
    selectedOwnerId = item.owner_user_id || '';
    draftNotes = item.notes || '';
    if (typeof window !== 'undefined') {
      const u = new URL(window.location.href);
      u.searchParams.set('incident', item.id);
      history.replaceState({}, '', `${u.pathname}${u.search}${u.hash}`);
    }
    try {
      const [router, metrics] = await Promise.all([
        api.mikrotik.routers.get(item.router_id),
        api.mikrotik.routers.metrics(item.router_id, 1),
      ]);
      detailRouter = (router || null) as RouterRow | null;
      detailMetric = Array.isArray(metrics) && metrics.length > 0 ? (metrics[0] as RouterMetricRow) : null;
    } catch {
      // keep incident detail visible even if router snapshot fails
    } finally {
      detailLoading = false;
    }
  }

  function ownerLabel(ownerUserId?: string | null) {
    if (!ownerUserId) return $t('admin.network.incidents.drawer.unassigned') || 'Unassigned';
    const member = teamMembers.find((m) => m.user_id === ownerUserId || m.id === ownerUserId);
    if (!member) return ownerUserId;
    return `${member.name} (${member.email})`;
  }

  function resetFilters() {
    filterAssignee = 'all';
    filterStatus = 'all';
    filterSeverity = 'all';
    filterType = 'all';
    filterSort = 'last_seen_desc';
    filterFrom = '';
    filterTo = '';
  }

  function toggleSelected(id: string) {
    selectedIds = selectedIds.includes(id)
      ? selectedIds.filter((x) => x !== id)
      : [...selectedIds, id];
  }

  async function focusStatus(status: 'open' | 'ack' | 'in_progress' | 'resolved') {
    if (status === 'resolved' && activeOnly) {
      activeOnly = false;
      await load();
    }
    filterStatus = status;
  }

  function formatDurationCompact(ms?: number | null) {
    if (ms == null || !Number.isFinite(ms) || ms < 0) return '—';
    return formatOpenDuration(ms);
  }

  function focusTopType() {
    if (!analytics.topType) return;
    filterSeverity = 'all';
    filterStatus = 'all';
    filterType = analytics.topType;
  }

  function toggleSelectAllFiltered() {
    const ids = filteredIds;
    if (!ids.length) return;
    if (allFilteredSelected) {
      selectedIds = selectedIds.filter((id) => !ids.includes(id));
      return;
    }
    selectedIds = Array.from(new Set([...selectedIds, ...ids]));
  }

  function clearSelection() {
    selectedIds = [];
    bulkAssigneeId = '';
  }

  function buildIncidentActivity(incident: IncidentRow) {
    const items: Array<{ ts: string; title: string; detail?: string }> = [];
    const fmt = (v?: string | null) =>
      v
        ? formatDateTime(v, {
            timeZone: $appSettings.app_timezone,
          })
        : null;

    if (incident.first_seen_at) {
      items.push({
        ts: incident.first_seen_at,
        title: $t('admin.network.incidents.activity.first_seen') || 'First detected',
      });
    }

    if (incident.acked_at) {
      items.push({
        ts: incident.acked_at,
        title: $t('admin.network.incidents.activity.acked') || 'Acknowledged',
        detail: incident.acked_by || undefined,
      });
    }

    if (incident.owner_user_id) {
      items.push({
        ts: incident.updated_at,
        title: $t('admin.network.incidents.activity.assigned') || 'Assigned',
        detail: ownerLabel(incident.owner_user_id),
      });
    }

    if (incident.notes && incident.notes.trim()) {
      items.push({
        ts: incident.updated_at,
        title: $t('admin.network.incidents.activity.notes_updated') || 'Notes updated',
      });
    }

    if (incident.is_auto_escalated) {
      items.push({
        ts: incident.escalated_at || incident.updated_at,
        title: $t('admin.network.incidents.activity.auto_escalated') || 'Auto escalated to critical',
      });
    }

    if (incident.resolved_at) {
      items.push({
        ts: incident.resolved_at,
        title: $t('admin.network.incidents.activity.resolved') || 'Resolved',
      });
    }

    items.push({
      ts: incident.last_seen_at,
      title: $t('admin.network.incidents.activity.last_seen') || 'Last seen',
    });

    return items
      .filter((it) => !!it.ts)
      .sort((a, b) => new Date(b.ts).getTime() - new Date(a.ts).getTime())
      .map((it) => ({ ...it, ts: fmt(it.ts) || '' }));
  }

  type RunbookStep = {
    title: string;
    detail?: string;
    command?: string;
  };

  function runbookStepsFor(incident: IncidentRow): RunbookStep[] {
    const iface = incident.interface_name || '<interface>';
    if (incident.incident_type === 'offline') {
      return [
        {
          title: $t('admin.network.incidents.runbook.offline.step1') || 'Verify power and physical link',
          detail:
            $t('admin.network.incidents.runbook.offline.step1_desc') ||
            'Check device power source, cable, and upstream connectivity.',
        },
        {
          title: $t('admin.network.incidents.runbook.offline.step2') || 'Test management reachability',
          command: 'ping <router-ip> -c 5',
        },
        {
          title: $t('admin.network.incidents.runbook.offline.step3') || 'Validate API service/port',
          command: 'nc -zv <router-ip> 8728',
        },
      ];
    }

    if (incident.incident_type === 'cpu') {
      return [
        {
          title: $t('admin.network.incidents.runbook.cpu.step1') || 'Identify top resource usage',
          command: '/system/resource/print',
        },
        {
          title: $t('admin.network.incidents.runbook.cpu.step2') || 'Review active traffic/interface load',
          command: '/interface/monitor-traffic',
        },
        {
          title: $t('admin.network.incidents.runbook.cpu.step3') || 'Check recent log spikes',
          command: '/log/print where topics~"error|critical"',
        },
      ];
    }

    if (incident.incident_type === 'latency') {
      return [
        {
          title: $t('admin.network.incidents.runbook.latency.step1') || 'Measure packet loss and RTT',
          command: 'ping <router-ip> -c 20',
        },
        {
          title: $t('admin.network.incidents.runbook.latency.step2') || 'Inspect WAN interface counters',
          command: `/interface/print where name="${iface}"`,
        },
        {
          title: $t('admin.network.incidents.runbook.latency.step3') || 'Check upstream path health',
          detail:
            $t('admin.network.incidents.runbook.latency.step3_desc') ||
            'Validate provider uplink and intermediate hops.',
        },
      ];
    }

    if (incident.incident_type === 'interface_down') {
      return [
        {
          title: $t('admin.network.incidents.runbook.interface_down.step1') || 'Verify interface state',
          command: `/interface/print where name="${iface}"`,
        },
        {
          title: $t('admin.network.incidents.runbook.interface_down.step2') || 'Check link flap/down counter',
          command: `/interface/monitor-traffic "${iface}" once`,
        },
        {
          title: $t('admin.network.incidents.runbook.interface_down.step3') || 'Inspect transceiver/cable',
          detail:
            $t('admin.network.incidents.runbook.interface_down.step3_desc') ||
            'Swap cable/SFP if needed and re-check link state.',
        },
      ];
    }

    return [
      {
        title: $t('admin.network.incidents.runbook.generic.step1') || 'Collect context from router',
        detail: $t('admin.network.incidents.runbook.generic.step1_desc') || 'Check current status and latest logs.',
      },
      {
        title: $t('admin.network.incidents.runbook.generic.step2') || 'Stabilize service impact',
        detail:
          $t('admin.network.incidents.runbook.generic.step2_desc') ||
          'Apply mitigation and monitor for recovery.',
      },
    ];
  }

  async function copyRunbookCommand(command: string) {
    try {
      await navigator.clipboard.writeText(command);
      toast.success($t('admin.network.incidents.runbook.copied') || 'Command copied');
    } catch {
      toast.error($t('admin.network.incidents.runbook.copy_failed') || 'Failed to copy command');
    }
  }

  function addRunbookStepToNotes(step: RunbookStep) {
    const line = `- [x] ${step.title}`;
    if ((draftNotes || '').includes(line)) return;
    draftNotes = draftNotes?.trim() ? `${draftNotes.trim()}\n${line}` : line;
  }

  async function saveIncidentMeta() {
    if (!detailIncident) return;
    detailSaving = true;
    try {
      const updated = await api.mikrotik.incidents.update(detailIncident.id, {
        ownerUserId: selectedOwnerId || null,
        notes: draftNotes.trim() ? draftNotes.trim() : null,
      });
      const merged: IncidentRow = {
        ...detailIncident,
        owner_user_id: updated?.owner_user_id ?? null,
        notes: updated?.notes ?? null,
        status: updated?.status ?? detailIncident.status,
        updated_at: updated?.updated_at ?? detailIncident.updated_at,
      };
      detailIncident = merged;
      rows = rows.map((r) => (r.id === merged.id ? { ...r, ...merged } : r));
      toast.success($t('common.saved') || 'Saved');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      detailSaving = false;
    }
  }

  function closeDetail() {
    detailOpen = false;
    detailIncident = null;
    if (typeof window !== 'undefined') {
      const u = new URL(window.location.href);
      u.searchParams.delete('incident');
      history.replaceState({}, '', `${u.pathname}${u.search}${u.hash}`);
    }
  }

  function networkRoute(to: 'noc' | 'alerts' | 'incidents') {
    return `${tenantPrefix}/admin/network/${to}`;
  }

  function openNetworkSettings() {
    goto(`${tenantPrefix}/admin/settings?tab=network`);
  }

  function formatBps(bps?: number | null) {
    if (bps == null) return $t('common.na') || '—';
    const abs = Math.abs(bps);
    const units = ['bps', 'Kbps', 'Mbps', 'Gbps'];
    let u = 0;
    let v = abs;
    while (v >= 1000 && u < units.length - 1) {
      v /= 1000;
      u++;
    }
    const s = `${v >= 10 || u === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[u]}`;
    return bps < 0 ? `-${s}` : s;
  }

  function memoryUsePct(total?: number | null, free?: number | null) {
    if (!total || total <= 0 || free == null) return null;
    const used = total - free;
    return Math.max(0, Math.min(100, Math.round((used / total) * 100)));
  }

  function incidentStartAt(row: IncidentRow) {
    return row.first_seen_at || row.updated_at || row.last_seen_at;
  }

  function incidentOpenMs(row: IncidentRow) {
    const started = new Date(incidentStartAt(row)).getTime();
    if (Number.isNaN(started)) return 0;
    return Math.max(0, nowMs - started);
  }

  function formatOpenDuration(ms: number) {
    const totalMinutes = Math.floor(ms / 60000);
    const days = Math.floor(totalMinutes / (24 * 60));
    const hours = Math.floor((totalMinutes % (24 * 60)) / 60);
    const minutes = totalMinutes % 60;
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
  }

  function slaLevel(row: IncidentRow) {
    if (row.status === 'resolved') return 'ok';
    const ms = incidentOpenMs(row);
    const warnMs = slaWarnMinutes * 60 * 1000;
    const breachMs = slaBreachMinutes * 60 * 1000;
    if (ms >= breachMs) return 'breach';
    if (ms >= warnMs) return 'warn';
    return 'ok';
  }

  function exportRows() {
    return filteredRows.map((row) => ({
      id: row.id,
      title: row.title || '',
      message: row.message || '',
      incident_type: row.incident_type || '',
      severity: row.severity || '',
      status: row.status || '',
      assignee: ownerLabel(row.owner_user_id),
      interface_name: row.interface_name || '',
      auto_escalated: row.is_auto_escalated ? 'yes' : 'no',
      escalated_at: row.escalated_at
        ? formatDateTime(row.escalated_at, { timeZone: $appSettings.app_timezone })
        : '',
      router_id: row.router_id || '',
      sla_open_for: formatOpenDuration(incidentOpenMs(row)),
      first_seen_at: row.first_seen_at
        ? formatDateTime(row.first_seen_at, { timeZone: $appSettings.app_timezone })
        : '',
      last_seen_at: row.last_seen_at
        ? formatDateTime(row.last_seen_at, { timeZone: $appSettings.app_timezone })
        : '',
      resolved_at: row.resolved_at
        ? formatDateTime(row.resolved_at, { timeZone: $appSettings.app_timezone })
        : '',
      notes: row.notes || '',
    }));
  }

  function exportCsv() {
    const rowsData = exportRows();
    if (!rowsData.length) {
      toast.error($t('admin.network.incidents.export.empty') || 'No data to export');
      return;
    }
    exportCsvRows(rowsData, 'incidents');
  }

  function exportExcel() {
    const rowsData = exportRows();
    if (!rowsData.length) {
      toast.error($t('admin.network.incidents.export.empty') || 'No data to export');
      return;
    }
    exportExcelRows(rowsData, 'incidents');
  }

  async function bulkAck() {
    const targets = selectedRows
      .filter((r) => r.status !== 'ack' && r.status !== 'resolved')
      .map((r) => r.id);
    if (!targets.length) return;
    bulkBusy = true;
    try {
      const result = await Promise.allSettled(targets.map((id) => api.mikrotik.incidents.ack(id)));
      const ok = result.filter((r) => r.status === 'fulfilled').length;
      const fail = result.length - ok;
      if (ok > 0) toast.success(`${ok} incident(s) acknowledged`);
      if (fail > 0) toast.error(`${fail} incident(s) failed to acknowledge`);
      await load();
      clearSelection();
    } finally {
      bulkBusy = false;
    }
  }

  async function bulkResolve() {
    const targets = selectedRows.filter((r) => r.status !== 'resolved').map((r) => r.id);
    if (!targets.length) return;
    bulkBusy = true;
    try {
      const result = await Promise.allSettled(targets.map((id) => api.mikrotik.incidents.resolve(id)));
      const ok = result.filter((r) => r.status === 'fulfilled').length;
      const fail = result.length - ok;
      if (ok > 0) toast.success(`${ok} incident(s) resolved`);
      if (fail > 0) toast.error(`${fail} incident(s) failed to resolve`);
      await load();
      clearSelection();
    } finally {
      bulkBusy = false;
    }
  }

  async function bulkAssign() {
    const assignee = bulkAssigneeId.trim();
    if (!assignee || !selectedRows.length) return;
    bulkBusy = true;
    try {
      const result = await Promise.allSettled(
        selectedRows.map((row) =>
          api.mikrotik.incidents.update(row.id, {
            ownerUserId: assignee,
            notes: row.notes || null,
          }),
        ),
      );
      const ok = result.filter((r) => r.status === 'fulfilled').length;
      const fail = result.length - ok;
      if (ok > 0) toast.success(`${ok} incident(s) assigned`);
      if (fail > 0) toast.error(`${fail} incident(s) failed to assign`);
      await load();
      clearSelection();
    } finally {
      bulkBusy = false;
    }
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={$t('admin.network.incidents.title') || 'Network Incidents'}
    subtitle={$t('admin.network.incidents.subtitle') || 'Operational incident records deduplicated from alerts.'}
  >
    {#snippet actions()}
      <AlertsIncidentsSwitch
        current="incidents"
        nocHref={networkRoute('noc')}
        alertsHref={networkRoute('alerts')}
        incidentsHref={networkRoute('incidents')}
      />

      <button
        class="btn ghost"
        type="button"
        onclick={() => {
          activeOnly = !activeOnly;
          void load();
        }}
        title={$t('admin.network.incidents.actions.toggle') || 'Toggle active/resolved'}
      >
        <Icon name={activeOnly ? 'filter' : 'archive'} size={16} />
        {activeOnly
          ? $t('admin.network.incidents.actions.active') || 'Active'
          : $t('admin.network.incidents.actions.all') || 'All'}
      </button>

      <button class="btn ghost" type="button" onclick={load} title={$t('common.refresh') || 'Refresh'}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      {#if $can('manage', 'network_routers')}
        <button class="btn ghost" type="button" onclick={runAutoEscalationNow} disabled={escalationRunBusy}>
          <Icon name="shield-alert" size={16} />
          {escalationRunBusy
            ? $t('common.processing') || 'Processing...'
            : $t('admin.network.incidents.actions.run_auto_escalation') || 'Run Auto Escalation'}
        </button>
      {/if}
      <div class="export-wrap">
        <button class="btn ghost" type="button" onclick={toggleExportMenu}>
          <Icon name="download" size={16} />
          {$t('admin.network.incidents.export.title') || 'Export'}
          <Icon name="chevron-down" size={14} />
        </button>
        {#if exportMenuOpen}
          <button
            class="export-backdrop"
            type="button"
            onclick={() => {
              exportMenuOpen = false;
            }}
            aria-label={$t('common.close') || 'Close'}
          ></button>
          <div class="export-menu">
            <button
              class="export-item"
              type="button"
              onclick={() => {
                exportMenuOpen = false;
                exportCsv();
              }}
            >
              {$t('admin.network.incidents.export.csv') || 'Export CSV'}
            </button>
            <button
              class="export-item"
              type="button"
              onclick={() => {
                exportMenuOpen = false;
                exportExcel();
              }}
            >
              {$t('admin.network.incidents.export.excel') || 'Export Excel'}
            </button>
          </div>
        {/if}
      </div>
      {#if $can('manage', 'network_routers')}
        <button class="btn ghost" type="button" onclick={openSimulateDialog}>
          <Icon name="activity" size={16} />
          {$t('admin.network.incidents.actions.simulate') || 'Simulate'}
        </button>
      {/if}
    {/snippet}
  </NetworkPageHeader>

  <div class="table-wrap">
    <div class="analytics">
      <button class="analytic-card" type="button" onclick={() => void focusStatus('open')}>
        <div class="label">{$t('admin.network.incidents.analytics.open') || 'Open'}</div>
        <div class="value">{analytics.open}</div>
      </button>
      <button class="analytic-card" type="button" onclick={() => void focusStatus('ack')}>
        <div class="label">{$t('admin.network.incidents.analytics.ack') || 'Ack'}</div>
        <div class="value">{analytics.ack}</div>
      </button>
      <button class="analytic-card" type="button" onclick={() => void focusStatus('in_progress')}>
        <div class="label">{$t('admin.network.incidents.analytics.in_progress') || 'In Progress'}</div>
        <div class="value">{analytics.inProgress}</div>
      </button>
      <button class="analytic-card" type="button" onclick={() => void focusStatus('resolved')}>
        <div class="label">{$t('admin.network.incidents.analytics.resolved') || 'Resolved'}</div>
        <div class="value">{analytics.resolved}</div>
      </button>
      <div class="analytic-card">
        <div class="label">{$t('admin.network.incidents.analytics.mtta') || 'MTTA'}</div>
        <div class="value">{formatDurationCompact(analytics.mtta)}</div>
      </div>
      <div class="analytic-card">
        <div class="label">{$t('admin.network.incidents.analytics.mttr') || 'MTTR'}</div>
        <div class="value">{formatDurationCompact(analytics.mttr)}</div>
      </div>
      <button class="analytic-card" type="button" onclick={focusTopType} disabled={!analytics.topType}>
        <div class="label">{$t('admin.network.incidents.analytics.top_type_24h') || 'Top Type (24h)'}</div>
        <div class="value">{analytics.topType ? `${typeLabel(analytics.topType)} (${analytics.topTypeCount})` : '—'}</div>
      </button>
    </div>

    <NetworkFilterPanel>
      <div class="control">
        <label for="inc-filter-assignee">{$t('admin.network.incidents.filters.assignee') || 'Assignee'}</label>
        <select id="inc-filter-assignee" class="input" bind:value={filterAssignee}>
          <option value="all">{$t('admin.network.incidents.filters.all_assignees') || 'All assignees'}</option>
          <option value="unassigned">
            {$t('admin.network.incidents.filters.unassigned') || 'Unassigned'}
          </option>
          {#each teamMembers as member}
            <option value={member.user_id}>{member.name}</option>
          {/each}
        </select>
      </div>

      <div class="control">
        <label for="inc-filter-status">{$t('admin.network.incidents.columns.status') || 'Status'}</label>
        <select id="inc-filter-status" class="input" bind:value={filterStatus}>
          <option value="all">{$t('admin.network.incidents.filters.all_status') || 'All status'}</option>
          <option value="open">open</option>
          <option value="in_progress">in_progress</option>
          <option value="ack">ack</option>
          <option value="resolved">resolved</option>
        </select>
      </div>

      <div class="control">
        <label for="inc-filter-severity">{$t('admin.network.incidents.columns.severity') || 'Severity'}</label>
        <select id="inc-filter-severity" class="input" bind:value={filterSeverity}>
          <option value="all">{$t('admin.network.incidents.filters.all_severity') || 'All severity'}</option>
          <option value="info">{severityLabel('info')}</option>
          <option value="warning">{severityLabel('warning')}</option>
          <option value="critical">{severityLabel('critical')}</option>
        </select>
      </div>

      <div class="control">
        <label for="inc-filter-type">{$t('admin.network.incidents.columns.type') || 'Type'}</label>
        <select id="inc-filter-type" class="input" bind:value={filterType}>
          <option value="all">{$t('admin.network.incidents.filters.all_types') || 'All types'}</option>
          {#each incidentTypeOptions as typeOption}
            <option value={typeOption}>{typeLabel(typeOption)}</option>
          {/each}
        </select>
      </div>

      <div class="control">
        <label for="inc-filter-sort">{$t('admin.network.incidents.filters.sort') || 'Sort'}</label>
        <select id="inc-filter-sort" class="input" bind:value={filterSort}>
          <option value="last_seen_desc">
            {$t('admin.network.incidents.filters.sort_last_seen_desc') || 'Last seen (newest)'}
          </option>
          <option value="last_seen_asc">
            {$t('admin.network.incidents.filters.sort_last_seen_asc') || 'Last seen (oldest)'}
          </option>
          <option value="severity_desc">
            {$t('admin.network.incidents.filters.sort_severity_desc') || 'Severity (highest)'}
          </option>
          <option value="open_duration_desc">
            {$t('admin.network.incidents.filters.sort_open_duration_desc') || 'Open duration (longest)'}
          </option>
        </select>
      </div>

      <div class="control">
        <label for="inc-filter-from">{$t('admin.network.incidents.filters.from') || 'From'}</label>
        <input id="inc-filter-from" class="input" type="date" bind:value={filterFrom} />
      </div>

      <div class="control">
        <label for="inc-filter-to">{$t('admin.network.incidents.filters.to') || 'To'}</label>
        <input id="inc-filter-to" class="input" type="date" bind:value={filterTo} />
      </div>

      <div class="control control-actions">
        <div class="control-spacer" aria-hidden="true"></div>
        <button class="btn ghost" type="button" onclick={resetFilters}>
          <Icon name="x-circle" size={14} />
          {$t('admin.network.incidents.filters.reset') || 'Reset'}
        </button>
      </div>
    </NetworkFilterPanel>

    {#if $can('manage', 'network_routers')}
      <div class="bulk-bar">
        <label class="bulk-select-all">
          <input type="checkbox" checked={allFilteredSelected} onchange={toggleSelectAllFiltered} />
          <span>{$t('admin.network.incidents.bulk.select_filtered') || 'Select filtered'}</span>
        </label>

        <span class="bulk-count">
          {$t('admin.network.incidents.bulk.selected', { values: { count: selectedCount } }) ||
            `${selectedCount} selected`}
        </span>

        <div class="bulk-actions">
          <button class="btn ghost" type="button" disabled={bulkBusy || canBulkAckCount === 0} onclick={bulkAck}>
            <Icon name="check" size={14} />
            {$t('admin.network.incidents.bulk.ack') || 'Bulk Ack'}
          </button>
          <button
            class="btn ghost"
            type="button"
            disabled={bulkBusy || canBulkResolveCount === 0}
            onclick={bulkResolve}
          >
            <Icon name="check-circle" size={14} />
            {$t('admin.network.incidents.bulk.resolve') || 'Bulk Resolve'}
          </button>
          <select class="input bulk-assign" bind:value={bulkAssigneeId} disabled={bulkBusy || selectedCount === 0}>
            <option value="">{($t('admin.network.incidents.bulk.assign_placeholder') || 'Assign to...')}</option>
            {#each teamMembers as member}
              <option value={member.user_id}>{member.name}</option>
            {/each}
          </select>
          <button
            class="btn ghost"
            type="button"
            disabled={bulkBusy || selectedCount === 0 || !bulkAssigneeId}
            onclick={bulkAssign}
          >
            <Icon name="users" size={14} />
            {$t('admin.network.incidents.bulk.assign') || 'Bulk Assign'}
          </button>
          <button class="btn ghost" type="button" disabled={bulkBusy || selectedCount === 0} onclick={clearSelection}>
            <Icon name="x-circle" size={14} />
            {$t('admin.network.incidents.bulk.clear') || 'Clear'}
          </button>
        </div>
      </div>
    {/if}

    <Table
      {columns}
      data={filteredRows}
      keyField="id"
      {loading}
      pagination={true}
      pageSize={10}
      searchable={true}
      searchPlaceholder={$t('admin.network.incidents.search') || 'Search incidents...'}
      mobileView={isMobile ? 'card' : 'scroll'}
      emptyText={$t('admin.network.incidents.empty') || 'No incidents'}
    >
      {#snippet cell({ item, key }: any)}
        {#if key === 'select'}
          {#if $can('manage', 'network_routers')}
            <input
              type="checkbox"
              class="row-check"
              checked={selectedIds.includes(item.id)}
              onchange={() => toggleSelected(item.id)}
            />
          {/if}
        {:else if key === 'title'}
          <div class="cell-title">
            <div class="row-top">
              <button class="name-link" type="button" onclick={() => void openDetail(item)}>{item.title}</button>
              <span class="chip mono">{item.interface_name || '-'}</span>
            </div>
            <div class="muted">{item.message}</div>
          </div>
        {:else if key === 'type'}
          <span class="chip mono">{typeLabel(item.incident_type)}</span>
        {:else if key === 'severity'}
          <div class="severity-cell">
            <span class="pill" class:critical={item.severity === 'critical'} class:warn={item.severity === 'warning'}>
              {severityLabel(item.severity)}
            </span>
            {#if item.is_auto_escalated}
              <span class="pill auto-escalated">{$t('admin.network.incidents.labels.auto_escalated') || 'Auto Escalated'}</span>
            {/if}
          </div>
        {:else if key === 'status'}
          <div class="status-cell">
            <span class="pill" class:ack={item.status === 'ack'} class:resolved={item.status === 'resolved'}>
              {item.status}
            </span>
            <span class="sla-badge" class:warn={slaLevel(item) === 'warn'} class:breach={slaLevel(item) === 'breach'}>
              {$t('admin.network.incidents.sla.open_for') || 'Open for'} {formatOpenDuration(incidentOpenMs(item))}
            </span>
          </div>
        {:else if key === 'seen'}
          <span class="muted" title={formatDateTime(item.last_seen_at, { timeZone: $appSettings.app_timezone })}>
            {timeAgo(item.last_seen_at)}
          </span>
        {:else if key === 'actions'}
          <RowActionButtons
            fullWidth={true}
            onOpen={() => openRouter(item.router_id)}
            showDetail={true}
            onDetail={() => void openDetail(item)}
            showAcknowledge={item.status !== 'ack' && item.status !== 'resolved' && $can('manage', 'network_routers')}
            onAcknowledge={() => ack(item.id)}
            showResolve={item.status !== 'resolved' && $can('manage', 'network_routers')}
            onResolve={() => resolve(item.id)}
          />
        {:else}
          {item[key] ?? ''}
        {/if}
      {/snippet}
    </Table>
  </div>

  <IncidentDetailDrawer
    open={detailOpen}
    incident={detailIncident}
    loading={detailLoading}
    router={detailRouter}
    metric={detailMetric}
    teamMembers={teamMembers}
    selectedOwnerId={selectedOwnerId}
    draftNotes={draftNotes}
    saving={detailSaving}
    canManage={$can('manage', 'network_routers')}
    emailNotifyEnabled={assignmentEmailEnabled}
    slaState={detailIncident ? slaLevel(detailIncident) : 'ok'}
    slaOpenDuration={detailIncident ? formatOpenDuration(incidentOpenMs(detailIncident)) : '—'}
    appTimezone={$appSettings.app_timezone}
    runbookSteps={detailIncident ? runbookStepsFor(detailIncident) : []}
    activityItems={detailIncident ? buildIncidentActivity(detailIncident) : []}
    {ownerLabel}
    {typeLabel}
    {severityLabel}
    {formatDateTime}
    {formatBps}
    {memoryUsePct}
    onClose={closeDetail}
    onOpenRouter={openRouter}
    onAcknowledge={ack}
    onResolve={resolve}
    onSave={saveIncidentMeta}
    onOpenNetworkSettings={openNetworkSettings}
    onOwnerChange={(value) => {
      selectedOwnerId = value;
    }}
    onNotesChange={(value) => {
      draftNotes = value;
    }}
    onCopyRunbookCommand={copyRunbookCommand}
    onAddRunbookStep={addRunbookStepToNotes}
  />

  <IncidentSimulateDrawer
    open={simulateOpen}
    busy={simulateBusy}
    routers={simulateRouters}
    routerId={simulateRouterId}
    incidentType={simulateType}
    severity={simulateSeverity}
    interfaceName={simulateInterface}
    message={simulateMessage}
    onClose={closeSimulateDialog}
    onSubmit={submitSimulateIncident}
    onRouterChange={(value) => {
      simulateRouterId = value;
    }}
    onTypeChange={(value) => {
      simulateType = value;
    }}
    onSeverityChange={(value) => {
      simulateSeverity = value;
    }}
    onInterfaceChange={(value) => {
      simulateInterface = value;
    }}
    onMessageChange={(value) => {
      simulateMessage = value;
    }}
  />
</div>

<style>
  .page-content {
    padding: 28px;
    max-width: 1460px;
    margin: 0 auto;
  }
  .export-wrap {
    position: relative;
  }
  .export-backdrop {
    position: fixed;
    inset: 0;
    border: 0;
    background: transparent;
    z-index: 55;
  }
  .export-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    min-width: 190px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
    overflow: hidden;
    z-index: 56;
  }
  .export-item {
    width: 100%;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    text-align: left;
    padding: 10px 12px;
    font-size: 0.86rem;
    font-weight: 700;
    cursor: pointer;
  }
  .export-item:hover {
    background: var(--bg-hover);
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    font-weight: 800;
    cursor: pointer;
  }
  .btn:hover {
    background: var(--bg-hover);
  }
  .table-wrap {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.2);
  }
  .analytics {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 10px;
    padding: 12px;
    border-bottom: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 82%, transparent);
  }
  .analytic-card {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 10px;
    text-align: left;
    display: grid;
    gap: 4px;
    min-height: 66px;
  }
  button.analytic-card {
    cursor: pointer;
  }
  button.analytic-card:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border-color));
  }
  .analytic-card .label {
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .analytic-card .value {
    color: var(--text-primary);
    font-size: 0.96rem;
    font-weight: 850;
    line-height: 1.2;
  }
  .bulk-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 80%, transparent);
    flex-wrap: wrap;
  }
  .bulk-select-all {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 700;
  }
  .bulk-count {
    color: var(--text-secondary);
    font-size: 0.84rem;
    font-weight: 700;
  }
  .bulk-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .bulk-assign {
    min-width: 170px;
    height: 38px;
  }
  .row-check {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }
  .cell-title {
    display: grid;
    gap: 6px;
  }
  .row-top {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .name-link {
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font-weight: 850;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .name-link:hover {
    color: var(--accent);
  }
  .chip {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 3px 8px;
    font-size: 0.76rem;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }
  .pill {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 4px 10px;
    font-size: 0.78rem;
    font-weight: 850;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    text-transform: capitalize;
  }
  .pill.warn {
    border-color: color-mix(in srgb, var(--color-warning) 45%, var(--border-color));
    color: var(--color-warning);
  }
  .pill.critical {
    border-color: color-mix(in srgb, var(--color-danger) 45%, var(--border-color));
    color: var(--color-danger);
  }
  .pill.ack {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border-color));
  }
  .pill.resolved {
    opacity: 0.75;
  }
  .severity-cell {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .pill.auto-escalated {
    border-color: color-mix(in srgb, var(--color-danger) 60%, var(--border-color));
    color: color-mix(in srgb, var(--color-danger) 90%, #fff 10%);
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    text-transform: none;
  }
  .status-cell {
    display: grid;
    gap: 6px;
    justify-items: start;
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
  .input {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 10px 12px;
    outline: none;
  }
  .input:focus {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
  }
  @media (max-width: 720px) {
    .analytics {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .bulk-bar {
      align-items: stretch;
    }
    .bulk-actions {
      width: 100%;
    }
    .bulk-assign {
      min-width: 0;
      flex: 1;
    }
  }
</style>
