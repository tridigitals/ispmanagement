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
  import { formatDateTime, timeAgo } from '$lib/utils/date';
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

  const columns = $derived.by(() => [
    { key: 'select', label: '', width: '44px' },
    { key: 'title', label: $t('admin.network.incidents.columns.incident') || 'Incident' },
    { key: 'type', label: $t('admin.network.incidents.columns.type') || 'Type' },
    { key: 'severity', label: $t('admin.network.incidents.columns.severity') || 'Severity' },
    { key: 'status', label: $t('admin.network.incidents.columns.status') || 'Status' },
    { key: 'seen', label: $t('admin.network.incidents.columns.seen') || 'Last Seen' },
    { key: 'actions', label: '', align: 'right' as const, width: '140px' },
  ]);
  const filteredRows = $derived.by(() =>
    rows.filter((row) => {
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
    }),
  );
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
    try {
      const v = await api.settings.getValue('mikrotik_incident_assignment_email_enabled');
      const norm = (v || '').trim().toLowerCase();
      assignmentEmailEnabled = norm === 'true' || norm === '1' || norm === 'yes' || norm === 'on';
    } catch {
      assignmentEmailEnabled = false;
    }
  }

  async function loadSlaSettings() {
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

  function csvEscape(value: unknown) {
    const str = String(value ?? '');
    if (/[",\r\n]/.test(str)) return `"${str.replaceAll('"', '""')}"`;
    return str;
  }

  function downloadBlob(filename: string, blob: Blob) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  function buildExportFilename(ext: 'csv' | 'xls') {
    const stamp = new Date().toISOString().replace(/[:.]/g, '-');
    return `incidents-${stamp}.${ext}`;
  }

  function exportCsv() {
    const rowsData = exportRows();
    if (!rowsData.length) {
      toast.error($t('admin.network.incidents.export.empty') || 'No data to export');
      return;
    }
    const headers = Object.keys(rowsData[0]);
    const lines = [headers.join(',')];
    for (const row of rowsData) {
      lines.push(headers.map((k) => csvEscape((row as any)[k])).join(','));
    }
    const csv = lines.join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    downloadBlob(buildExportFilename('csv'), blob);
  }

  function htmlEscape(value: unknown) {
    return String(value ?? '')
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#39;');
  }

  function exportExcel() {
    const rowsData = exportRows();
    if (!rowsData.length) {
      toast.error($t('admin.network.incidents.export.empty') || 'No data to export');
      return;
    }
    const headers = Object.keys(rowsData[0]);
    const headHtml = headers.map((h) => `<th>${htmlEscape(h)}</th>`).join('');
    const bodyHtml = rowsData
      .map((row) => `<tr>${headers.map((h) => `<td>${htmlEscape((row as any)[h])}</td>`).join('')}</tr>`)
      .join('');
    const html = `<!doctype html><html><head><meta charset="utf-8" /></head><body><table border="1"><thead><tr>${headHtml}</tr></thead><tbody>${bodyHtml}</tbody></table></body></html>`;
    const blob = new Blob([html], { type: 'application/vnd.ms-excel;charset=utf-8;' });
    downloadBlob(buildExportFilename('xls'), blob);
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
  <div class="head">
    <div>
      <h1 class="title">{$t('admin.network.incidents.title') || 'Network Incidents'}</h1>
      <p class="sub">
        {$t('admin.network.incidents.subtitle') || 'Operational incident records deduplicated from alerts.'}
      </p>
    </div>

    <div class="head-actions">
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
    </div>
  </div>

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

    <div class="filters">
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
    </div>

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
          <div class="actions">
            <button class="icon-btn" type="button" onclick={() => openRouter(item.router_id)} title={$t('common.open') || 'Open'}>
              <Icon name="arrow-right" size={16} />
            </button>
            <button class="icon-btn" type="button" onclick={() => void openDetail(item)} title={$t('common.details') || 'Details'}>
              <Icon name="file-text" size={16} />
            </button>
            {#if item.status !== 'ack' && item.status !== 'resolved' && ($can('manage', 'network_routers'))}
              <button
                class="icon-btn"
                type="button"
                onclick={() => ack(item.id)}
                title={$t('admin.network.alerts.actions.ack') || 'Acknowledge'}
              >
                <Icon name="check" size={16} />
              </button>
            {/if}
            {#if item.status !== 'resolved' && ($can('manage', 'network_routers'))}
              <button
                class="icon-btn"
                type="button"
                onclick={() => resolve(item.id)}
                title={$t('admin.network.alerts.actions.resolve') || 'Resolve'}
              >
                <Icon name="check-circle" size={16} />
              </button>
            {/if}
          </div>
        {:else}
          {item[key] ?? ''}
        {/if}
      {/snippet}
    </Table>
  </div>

  {#if detailOpen && detailIncident}
    {@const incident = detailIncident}
    <button class="drawer-backdrop" type="button" onclick={closeDetail} aria-label={$t('common.close') || 'Close'}></button>
    <aside class="drawer" aria-label={$t('common.details') || 'Details'}>
      <div class="drawer-head">
        <div>
          <div class="drawer-title">{$t('common.details') || 'Details'}</div>
          <div class="drawer-sub">{incident.title}</div>
        </div>
        <button class="icon-btn" type="button" onclick={closeDetail} title={$t('common.close') || 'Close'}>
          <Icon name="x" size={16} />
        </button>
      </div>

      <div class="drawer-body">
        {#if detailLoading}
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
                ? formatDateTime(incident.escalated_at || incident.updated_at, { timeZone: $appSettings.app_timezone })
                : ($t('common.no') || 'No')}
            </span>
          </div>
          <div class="drow"><span class="muted">{$t('admin.network.incidents.columns.seen') || 'Last Seen'}</span><span class="mono">{formatDateTime(incident.last_seen_at, { timeZone: $appSettings.app_timezone })}</span></div>
          <div class="drow">
            <span class="muted">{$t('admin.network.incidents.drawer.email_notify') || 'Email notify'}</span>
            <span class="mono">
              <span class:flag-on={assignmentEmailEnabled} class:flag-off={!assignmentEmailEnabled} class="flag">
                {assignmentEmailEnabled
                  ? $t('admin.network.incidents.drawer.on') || 'On'
                  : $t('admin.network.incidents.drawer.off') || 'Off'}
              </span>
            </span>
          </div>
          <div class="drow">
            <span class="muted">{$t('admin.network.incidents.sla.title') || 'SLA Timer'}</span>
            <span class="mono">
              <span class="sla-badge" class:warn={slaLevel(incident) === 'warn'} class:breach={slaLevel(incident) === 'breach'}>
                {formatOpenDuration(incidentOpenMs(incident))}
              </span>
            </span>
          </div>
          <div class="drow"><span class="muted">{$t('admin.network.incidents.drawer.assignee') || 'Assignee'}</span><span class="mono">{ownerLabel(incident.owner_user_id)}</span></div>
          <div class="drow"><span class="muted">Router</span><span class="mono">{detailRouter?.identity || detailRouter?.name || incident.router_id}</span></div>
          <div class="drow"><span class="muted">Interface</span><span class="mono">{incident.interface_name || '-'}</span></div>
          {#if detailRouter}
            <div class="drow"><span class="muted">Host</span><span class="mono">{detailRouter.host}:{detailRouter.port}</span></div>
            <div class="drow"><span class="muted">Latency</span><span class="mono">{detailRouter.latency_ms == null ? '—' : `${detailRouter.latency_ms} ms`}</span></div>
          {/if}
          {#if detailMetric}
            <div class="drow"><span class="muted">CPU</span><span class="mono">{detailMetric.cpu_load == null ? '—' : `${detailMetric.cpu_load}%`}</span></div>
            <div class="drow"><span class="muted">RX/TX</span><span class="mono">{formatBps(detailMetric.rx_bps)} / {formatBps(detailMetric.tx_bps)}</span></div>
            <div class="drow"><span class="muted">Memory Use</span><span class="mono">{memoryUsePct(detailMetric.total_memory_bytes, detailMetric.free_memory_bytes) == null ? '—' : `${memoryUsePct(detailMetric.total_memory_bytes, detailMetric.free_memory_bytes)}%`}</span></div>
          {/if}
        </div>

        <div class="detail-message">{incident.message}</div>

        <div class="detail-edit">
          <div class="field">
            <label for="incident-owner">{$t('admin.network.incidents.drawer.assignee') || 'Assignee'}</label>
            {#if $can('manage', 'network_routers')}
              <select id="incident-owner" class="input" bind:value={selectedOwnerId}>
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
            {#if $can('manage', 'network_routers')}
              <textarea
                id="incident-notes"
                class="textarea"
                rows="4"
                bind:value={draftNotes}
                placeholder={$t('admin.network.incidents.drawer.notes_placeholder') || 'Add operator notes, handover context, and actions...'}
              ></textarea>
            {:else}
              <div class="readonly">{incident.notes || ($t('common.na') || '—')}</div>
            {/if}
          </div>
          {#if $can('manage', 'network_routers')}
            <div class="save-row">
              <button class="btn ghost" type="button" onclick={saveIncidentMeta} disabled={detailSaving}>
                <Icon name="save" size={16} />
                {detailSaving
                  ? $t('common.saving') || 'Saving...'
                  : $t('admin.network.incidents.drawer.save') || 'Save Notes'}
              </button>
            </div>
            <div class="save-row">
              <button class="btn ghost" type="button" onclick={openNetworkSettings}>
                <Icon name="settings" size={16} />
                {$t('admin.network.incidents.drawer.open_network_settings') || 'Open Network Settings'}
              </button>
            </div>
          {/if}
        </div>

        <div class="runbook">
          <div class="runbook-title">
            {$t('admin.network.incidents.runbook.title') || 'What to do next'}
          </div>
          <div class="runbook-sub">
            {$t('admin.network.incidents.runbook.subtitle') || 'Operator checklist based on incident type.'}
          </div>
          <div class="runbook-list">
            {#each runbookStepsFor(incident) as step}
              <div class="runbook-item">
                <div class="runbook-text">
                  <div class="runbook-step">{step.title}</div>
                  {#if step.detail}
                    <div class="runbook-detail">{step.detail}</div>
                  {/if}
                  {#if step.command}
                    <code class="runbook-command">{step.command}</code>
                  {/if}
                </div>
                <div class="runbook-actions">
                  {#if step.command}
                    <button class="icon-btn" type="button" onclick={() => copyRunbookCommand(step.command!)}>
                      <Icon name="link" size={14} />
                    </button>
                  {/if}
                  {#if $can('manage', 'network_routers')}
                    <button class="icon-btn" type="button" onclick={() => addRunbookStepToNotes(step)}>
                      <Icon name="check" size={14} />
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>

        <div class="timeline">
          <div class="timeline-title">
            {$t('admin.network.incidents.activity.title') || 'Activity Timeline'}
          </div>
          <div class="timeline-list">
            {#each buildIncidentActivity(incident) as event}
              <div class="timeline-item">
                <span class="dot"></span>
                <div class="timeline-content">
                  <div class="timeline-row">
                    <span class="timeline-event">{event.title}</span>
                    <span class="timeline-time">{event.ts}</span>
                  </div>
                  {#if event.detail}
                    <div class="timeline-detail">{event.detail}</div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      </div>

      <div class="drawer-actions">
        <button class="btn ghost" type="button" onclick={() => openRouter(incident.router_id)}>
          <Icon name="arrow-right" size={16} />
          {$t('common.open') || 'Open'}
        </button>
        {#if incident.status !== 'ack' && incident.status !== 'resolved' && ($can('manage', 'network_routers'))}
          <button class="btn ghost" type="button" onclick={() => ack(incident.id)}>
            <Icon name="check" size={16} />
            {$t('admin.network.alerts.actions.ack') || 'Acknowledge'}
          </button>
        {/if}
        {#if incident.status !== 'resolved' && ($can('manage', 'network_routers'))}
          <button class="btn ghost" type="button" onclick={() => resolve(incident.id)}>
            <Icon name="check-circle" size={16} />
            {$t('admin.network.alerts.actions.resolve') || 'Resolve'}
          </button>
        {/if}
      </div>
    </aside>
  {/if}

  {#if simulateOpen}
    <button
      class="drawer-backdrop"
      type="button"
      onclick={closeSimulateDialog}
      aria-label={$t('common.close') || 'Close'}
    ></button>
    <aside class="drawer simulate-drawer" aria-label={$t('admin.network.incidents.actions.simulate') || 'Simulate'}>
      <div class="drawer-head">
        <div>
          <div class="drawer-title">{$t('admin.network.incidents.actions.simulate') || 'Simulate'}</div>
          <div class="drawer-sub">{$t('admin.network.incidents.simulate.subtitle') || 'Create test incident manually'}</div>
        </div>
        <button class="icon-btn" type="button" onclick={closeSimulateDialog} disabled={simulateBusy}>
          <Icon name="x" size={16} />
        </button>
      </div>
      <div class="drawer-body">
        <div class="field">
          <label for="sim-router">{$t('admin.network.incidents.simulate.router') || 'Router'}</label>
          <select id="sim-router" class="input" bind:value={simulateRouterId} disabled={simulateBusy}>
            {#each simulateRouters as router}
              <option value={router.id}>{router.identity || router.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="sim-type">{$t('admin.network.incidents.simulate.type') || 'Incident type'}</label>
          <select id="sim-type" class="input" bind:value={simulateType} disabled={simulateBusy}>
            <option value="offline">offline</option>
            <option value="cpu">cpu</option>
            <option value="latency">latency</option>
            <option value="interface_down">interface_down</option>
          </select>
        </div>
        <div class="field">
          <label for="sim-sev">{$t('admin.network.incidents.simulate.severity') || 'Severity'}</label>
          <select id="sim-sev" class="input" bind:value={simulateSeverity} disabled={simulateBusy}>
            <option value="info">info</option>
            <option value="warning">warning</option>
            <option value="critical">critical</option>
          </select>
        </div>
        <div class="field">
          <label for="sim-iface">{$t('admin.network.incidents.simulate.interface') || 'Interface (optional)'}</label>
          <input
            id="sim-iface"
            class="input"
            type="text"
            bind:value={simulateInterface}
            placeholder="ether1"
            disabled={simulateBusy}
          />
        </div>
        <div class="field">
          <label for="sim-msg">{$t('admin.network.incidents.simulate.message') || 'Message (optional)'}</label>
          <textarea
            id="sim-msg"
            class="textarea"
            rows="4"
            bind:value={simulateMessage}
            placeholder={$t('admin.network.incidents.simulate.message_placeholder') || 'Optional simulation message'}
            disabled={simulateBusy}
          ></textarea>
        </div>
      </div>
      <div class="drawer-actions">
        <button class="btn ghost" type="button" onclick={closeSimulateDialog} disabled={simulateBusy}>
          {$t('common.cancel') || 'Cancel'}
        </button>
        <button class="btn ghost" type="button" onclick={submitSimulateIncident} disabled={simulateBusy || !simulateRouterId}>
          <Icon name="activity" size={16} />
          {simulateBusy
            ? $t('common.saving') || 'Saving...'
            : $t('admin.network.incidents.actions.simulate') || 'Simulate'}
        </button>
      </div>
    </aside>
  {/if}
</div>

<style>
  .page-content {
    padding: 28px;
  }
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }
  .title {
    margin: 0;
    font-size: 1.6rem;
    font-weight: 950;
    color: var(--text-primary);
  }
  .sub {
    margin: 0.35rem 0 0 0;
    color: var(--text-secondary);
  }
  .head-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
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
  .filters {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 10px;
    padding: 12px;
    border-bottom: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
  }
  .control {
    display: grid;
    gap: 6px;
  }
  .control label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-weight: 700;
  }
  .control-spacer {
    height: 1rem;
  }
  .control .input {
    height: 38px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 0 10px;
    outline: none;
  }
  .control .input:focus {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
  }
  .control-actions .btn {
    width: 100%;
    justify-content: center;
    height: 38px;
    padding: 0 10px;
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
  .actions {
    display: inline-flex;
    gap: 6px;
    justify-content: flex-end;
    width: 100%;
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
  .simulate-drawer {
    width: min(500px, 92vw);
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
  .timeline {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 12px;
    background: color-mix(in srgb, var(--bg-card) 70%, transparent);
    display: grid;
    gap: 10px;
  }
  .timeline-title {
    font-size: 0.8rem;
    font-weight: 800;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .timeline-list {
    display: grid;
    gap: 10px;
  }
  .timeline-item {
    display: grid;
    grid-template-columns: 14px 1fr;
    gap: 8px;
    align-items: start;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--accent);
    margin-top: 6px;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .timeline-content {
    display: grid;
    gap: 3px;
  }
  .timeline-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .timeline-event {
    color: var(--text-primary);
    font-weight: 700;
    font-size: 0.88rem;
  }
  .timeline-time {
    color: var(--text-secondary);
    font-size: 0.78rem;
    white-space: nowrap;
  }
  .timeline-detail {
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.4;
  }
  .runbook {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 12px;
    background: color-mix(in srgb, var(--bg-card) 70%, transparent);
    display: grid;
    gap: 8px;
  }
  .runbook-title {
    font-size: 0.82rem;
    font-weight: 800;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .runbook-sub {
    color: var(--text-secondary);
    font-size: 0.8rem;
  }
  .runbook-list {
    display: grid;
    gap: 8px;
  }
  .runbook-item {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 10px;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 10px;
    align-items: start;
  }
  .runbook-step {
    color: var(--text-primary);
    font-weight: 700;
    font-size: 0.86rem;
  }
  .runbook-detail {
    color: var(--text-secondary);
    font-size: 0.8rem;
    margin-top: 4px;
  }
  .runbook-command {
    display: inline-block;
    margin-top: 6px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
    padding: 4px 8px;
    color: var(--text-primary);
    font-size: 0.75rem;
  }
  .runbook-actions {
    display: inline-flex;
    gap: 6px;
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
  @media (max-width: 720px) {
    .analytics {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .filters {
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
    .detail-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
