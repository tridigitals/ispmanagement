import type { FeatureCollection, Geometry } from 'geojson';

export type NMNode = {
  id: string;
  name: string;
  node_type: string;
  status: string;
  lat: number;
  lng: number;
  metadata?: Record<string, any>;
};

export type NMLink = {
  id: string;
  name: string;
  link_type: string;
  status: string;
  from_node_id?: string;
  to_node_id?: string;
  priority?: number;
  capacity_mbps?: number | null;
  utilization_pct?: number | null;
  loss_db?: number | null;
  latency_ms?: number | null;
  geometry: Geometry;
};

export type NMZone = {
  id: string;
  name: string;
  zone_type: string;
  status: string;
  geometry: Geometry;
};

export type NMRouter = {
  id: string;
  name: string;
  host: string;
  port: number;
  is_online: boolean;
  enabled: boolean;
  identity?: string | null;
  ros_version?: string | null;
  latency_ms?: number | null;
  latitude?: number | null;
  longitude?: number | null;
};

export type CustomerPppoeVisualState = 'connected' | 'disconnected' | 'neutral';

export type LinkFieldConfig = {
  capacityLabel: string;
  utilizationLabel: string;
  latencyLabel: string;
  lossLabel: string;
  showLoss: boolean;
  helper: string;
};

export type NetworkMapPopupActionKey =
  | 'connect'
  | 'edit'
  | 'delete'
  | 'open-router'
  | 'open-customer'
  | 'open-service';

export type NetworkMapPopupActionModel = {
  key: NetworkMapPopupActionKey;
  label: string;
  tone: 'primary' | 'secondary' | 'danger';
};

export type NetworkMapPopupSummaryItem = {
  label: string;
  value: string;
  tone?: 'ok' | 'warn' | 'muted' | 'danger';
};

export type NetworkMapPopupStatusChip = {
  label: string;
  value: string;
  tone?: 'ok' | 'warn' | 'muted' | 'danger';
};

export type NetworkMapPopupModel = {
  variant?: 'default' | 'workflow-service';
  kicker: string;
  title: string;
  subtitle: string;
  statusText: string;
  tone: 'ok' | 'warn' | 'muted';
  contextText: string;
  statusChips?: NetworkMapPopupStatusChip[];
  summaryItems: NetworkMapPopupSummaryItem[];
  detailPairs: Array<{ label: string; value: string }>;
  actions: NetworkMapPopupActionModel[];
};

export type RouterPopupNodeContext = {
  zoneName?: string;
  sourceLabel?: string;
  topologyLabel?: string;
};

export const nodeTypeOptions = [
  { label: 'Core', value: 'core' },
  { label: 'POP', value: 'pop' },
  { label: 'OLT', value: 'olt' },
  { label: 'Router', value: 'router' },
  { label: 'Switch', value: 'switch' },
  { label: 'Tower', value: 'tower' },
  { label: 'AP', value: 'ap' },
  { label: 'ODC', value: 'odc' },
  { label: 'ODP', value: 'odp' },
  { label: 'Splitter', value: 'splitter' },
  { label: 'Junction', value: 'junction' },
  { label: 'Customer Premise', value: 'customer_premise' },
];

export const linkTypeOptions = [
  { label: 'Fiber', value: 'fiber' },
  { label: 'Wireless PTP', value: 'wireless_ptp' },
  { label: 'Wireless PTMP', value: 'wireless_ptmp' },
  { label: 'LAN', value: 'lan' },
  { label: 'VLAN Tunnel', value: 'vlan_tunnel' },
  { label: 'Backhaul', value: 'backhaul' },
];

export const linkStatusOptions = [
  { label: 'Planning', value: 'planning' },
  { label: 'Up', value: 'up' },
  { label: 'Down', value: 'down' },
  { label: 'Degraded', value: 'degraded' },
  { label: 'Maintenance', value: 'maintenance' },
  { label: 'Retired', value: 'retired' },
];

export function nodeTypeLabel(value: string | null | undefined) {
  const normalized = String(value || '').trim();
  if (normalized === 'customer_endpoint') return 'Customer Premise';
  return nodeTypeOptions.find((option) => option.value === normalized)?.label || normalized || '-';
}

export function getLinkFieldConfig(linkType: string): LinkFieldConfig {
  switch (linkType) {
    case 'fiber':
      return {
        capacityLabel: 'Capacity (Mbps)',
        utilizationLabel: 'Utilization (%)',
        latencyLabel: 'Latency (ms)',
        lossLabel: 'Optical Loss (dB)',
        showLoss: true,
        helper: 'Fiber links track optical loss and latency for quality monitoring.',
      };
    case 'wireless_ptp':
    case 'wireless_ptmp':
      return {
        capacityLabel: 'Throughput Capacity (Mbps)',
        utilizationLabel: 'Channel Utilization (%)',
        latencyLabel: 'Latency (ms)',
        lossLabel: 'Signal Loss (dB)',
        showLoss: true,
        helper: 'Wireless links track channel utilization, latency, and signal loss.',
      };
    case 'lan':
      return {
        capacityLabel: 'Port Capacity (Mbps)',
        utilizationLabel: 'Port Utilization (%)',
        latencyLabel: 'Latency (ms)',
        lossLabel: 'Loss (dB)',
        showLoss: false,
        helper: 'LAN links focus on port capacity and utilization.',
      };
    case 'vlan_tunnel':
      return {
        capacityLabel: 'Tunnel Capacity (Mbps)',
        utilizationLabel: 'Tunnel Utilization (%)',
        latencyLabel: 'Tunnel Latency (ms)',
        lossLabel: 'Loss (dB)',
        showLoss: false,
        helper: 'VLAN tunnels focus on tunnel throughput and latency.',
      };
    case 'backhaul':
      return {
        capacityLabel: 'Backhaul Capacity (Mbps)',
        utilizationLabel: 'Backhaul Utilization (%)',
        latencyLabel: 'Backhaul Latency (ms)',
        lossLabel: 'Backhaul Loss (dB)',
        showLoss: true,
        helper: 'Backhaul links should track end-to-end latency and link loss.',
      };
    default:
      return {
        capacityLabel: 'Capacity (Mbps)',
        utilizationLabel: 'Utilization (%)',
        latencyLabel: 'Latency (ms)',
        lossLabel: 'Loss (dB)',
        showLoss: true,
        helper: 'Link quality metrics adapt based on selected type.',
      };
  }
}

export function escapeHtml(input: unknown): string {
  return String(input ?? '-')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}

export function statusTone(statusRaw: unknown): 'ok' | 'warn' | 'muted' {
  const s = String(statusRaw || '').toLowerCase();
  if (s === 'active' || s === 'up') return 'ok';
  if (s === 'maintenance' || s === 'degraded') return 'warn';
  return 'muted';
}

function drawNodePictogram(
  ctx: CanvasRenderingContext2D,
  type: string,
  cx: number,
  cy: number,
  size: number,
) {
  const s = size;
  ctx.save();
  ctx.strokeStyle = '#ffffff';
  ctx.fillStyle = '#ffffff';
  ctx.lineWidth = Math.max(2, s * 0.11);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';

  switch (type) {
    case 'router': {
      ctx.strokeRect(cx - s * 0.48, cy - s * 0.26, s * 0.96, s * 0.52);
      ctx.beginPath();
      ctx.arc(cx - s * 0.2, cy, s * 0.05, 0, Math.PI * 2);
      ctx.arc(cx, cy, s * 0.05, 0, Math.PI * 2);
      ctx.arc(cx + s * 0.2, cy, s * 0.05, 0, Math.PI * 2);
      ctx.fill();
      break;
    }
    case 'switch': {
      ctx.strokeRect(cx - s * 0.5, cy - s * 0.26, s, s * 0.52);
      ctx.beginPath();
      ctx.moveTo(cx - s * 0.35, cy - s * 0.06);
      ctx.lineTo(cx + s * 0.35, cy - s * 0.06);
      ctx.moveTo(cx - s * 0.35, cy + s * 0.08);
      ctx.lineTo(cx + s * 0.35, cy + s * 0.08);
      ctx.stroke();
      break;
    }
    case 'tower': {
      ctx.beginPath();
      ctx.moveTo(cx, cy - s * 0.52);
      ctx.lineTo(cx - s * 0.28, cy + s * 0.42);
      ctx.lineTo(cx + s * 0.28, cy + s * 0.42);
      ctx.closePath();
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(cx - s * 0.18, cy + s * 0.12);
      ctx.lineTo(cx + s * 0.18, cy + s * 0.12);
      ctx.moveTo(cx - s * 0.11, cy - s * 0.12);
      ctx.lineTo(cx + s * 0.11, cy - s * 0.12);
      ctx.stroke();
      break;
    }
    case 'ap': {
      ctx.beginPath();
      ctx.arc(cx, cy + s * 0.22, s * 0.05, 0, Math.PI * 2);
      ctx.fill();
      ctx.beginPath();
      ctx.arc(cx, cy + s * 0.22, s * 0.22, -Math.PI * 0.95, -Math.PI * 0.05);
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(cx, cy + s * 0.22, s * 0.36, -Math.PI * 0.9, -Math.PI * 0.1);
      ctx.stroke();
      break;
    }
    case 'olt': {
      ctx.strokeRect(cx - s * 0.5, cy - s * 0.34, s, s * 0.68);
      for (let i = -1; i <= 1; i++) {
        ctx.beginPath();
        ctx.arc(cx + i * s * 0.2, cy - s * 0.08, s * 0.045, 0, Math.PI * 2);
        ctx.fill();
        ctx.beginPath();
        ctx.moveTo(cx + i * s * 0.22, cy + s * 0.12);
        ctx.lineTo(cx + i * s * 0.22, cy + s * 0.24);
        ctx.stroke();
      }
      break;
    }
    case 'splitter': {
      ctx.beginPath();
      ctx.moveTo(cx - s * 0.45, cy);
      ctx.lineTo(cx - s * 0.05, cy);
      ctx.moveTo(cx - s * 0.05, cy);
      ctx.lineTo(cx + s * 0.3, cy - s * 0.22);
      ctx.moveTo(cx - s * 0.05, cy);
      ctx.lineTo(cx + s * 0.3, cy + s * 0.22);
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(cx - s * 0.05, cy, s * 0.06, 0, Math.PI * 2);
      ctx.fill();
      break;
    }
    case 'junction': {
      ctx.beginPath();
      ctx.arc(cx, cy, s * 0.08, 0, Math.PI * 2);
      ctx.fill();
      ctx.beginPath();
      ctx.moveTo(cx - s * 0.34, cy);
      ctx.lineTo(cx - s * 0.1, cy);
      ctx.moveTo(cx + s * 0.1, cy);
      ctx.lineTo(cx + s * 0.34, cy);
      ctx.moveTo(cx, cy - s * 0.34);
      ctx.lineTo(cx, cy - s * 0.1);
      ctx.moveTo(cx, cy + s * 0.1);
      ctx.lineTo(cx, cy + s * 0.34);
      ctx.stroke();
      break;
    }
    case 'odc': {
      ctx.strokeRect(cx - s * 0.34, cy - s * 0.38, s * 0.68, s * 0.76);
      ctx.beginPath();
      ctx.moveTo(cx, cy - s * 0.26);
      ctx.lineTo(cx, cy + s * 0.26);
      ctx.moveTo(cx - s * 0.18, cy);
      ctx.lineTo(cx + s * 0.18, cy);
      ctx.stroke();
      break;
    }
    case 'odp': {
      ctx.beginPath();
      ctx.arc(cx, cy - s * 0.06, s * 0.18, 0, Math.PI * 2);
      ctx.stroke();
      ctx.strokeRect(cx - s * 0.22, cy + s * 0.08, s * 0.44, s * 0.24);
      break;
    }
    case 'pop': {
      ctx.strokeRect(cx - s * 0.4, cy - s * 0.4, s * 0.8, s * 0.8);
      ctx.beginPath();
      for (let r = -1; r <= 1; r++) {
        for (let c = -1; c <= 1; c++) {
          ctx.rect(
            cx + c * s * 0.18 - s * 0.035,
            cy + r * s * 0.18 - s * 0.035,
            s * 0.07,
            s * 0.07,
          );
        }
      }
      ctx.fill();
      break;
    }
    case 'core': {
      ctx.beginPath();
      ctx.arc(cx, cy, s * 0.14, 0, Math.PI * 2);
      ctx.fill();
      ctx.beginPath();
      ctx.moveTo(cx - s * 0.42, cy);
      ctx.lineTo(cx - s * 0.18, cy);
      ctx.moveTo(cx + s * 0.18, cy);
      ctx.lineTo(cx + s * 0.42, cy);
      ctx.moveTo(cx, cy - s * 0.42);
      ctx.lineTo(cx, cy - s * 0.18);
      ctx.moveTo(cx, cy + s * 0.18);
      ctx.lineTo(cx, cy + s * 0.42);
      ctx.stroke();
      break;
    }
    case 'customer_premise':
    case 'customer_endpoint': {
      ctx.beginPath();
      ctx.moveTo(cx - s * 0.42, cy + s * 0.08);
      ctx.lineTo(cx, cy - s * 0.34);
      ctx.lineTo(cx + s * 0.42, cy + s * 0.08);
      ctx.stroke();
      ctx.strokeRect(cx - s * 0.3, cy + s * 0.08, s * 0.6, s * 0.34);
      break;
    }
    default: {
      ctx.strokeRect(cx - s * 0.48, cy - s * 0.26, s * 0.96, s * 0.52);
      break;
    }
  }

  ctx.restore();
}

function buildNodeIconImage(bg: string, type: string): ImageData {
  const size = 64;
  const c = document.createElement('canvas');
  c.width = size;
  c.height = size;
  const ctx = c.getContext('2d');
  if (!ctx) return new ImageData(size, size);

  const r = size / 2;
  ctx.clearRect(0, 0, size, size);
  ctx.beginPath();
  ctx.arc(r, r + 1.2, r - 2, 0, Math.PI * 2);
  ctx.fillStyle = 'rgba(15, 23, 42, 0.35)';
  ctx.fill();

  ctx.beginPath();
  ctx.arc(r, r, r - 3, 0, Math.PI * 2);
  ctx.fillStyle = bg;
  ctx.fill();
  ctx.lineWidth = 2.6;
  ctx.strokeStyle = 'rgba(255,255,255,0.92)';
  ctx.stroke();

  drawNodePictogram(ctx, type, r, r, size * 0.56);
  return ctx.getImageData(0, 0, size, size);
}

export function ensureNodeTypeIconsRegistered(map: import('maplibre-gl').Map | null) {
  if (!map) return;
  const defs: Array<{ id: string; bg: string; type: string }> = [
    { id: 'nm-node-icon-core', bg: '#4f46e5', type: 'core' },
    { id: 'nm-node-icon-pop', bg: '#0ea5e9', type: 'pop' },
    { id: 'nm-node-icon-olt', bg: '#22c55e', type: 'olt' },
    { id: 'nm-node-icon-router', bg: '#3b82f6', type: 'router' },
    { id: 'nm-node-icon-switch', bg: '#2563eb', type: 'switch' },
    { id: 'nm-node-icon-tower', bg: '#f59e0b', type: 'tower' },
    { id: 'nm-node-icon-ap', bg: '#ef4444', type: 'ap' },
    { id: 'nm-node-icon-odc', bg: '#0f766e', type: 'odc' },
    { id: 'nm-node-icon-odp', bg: '#14b8a6', type: 'odp' },
    { id: 'nm-node-icon-splitter', bg: '#a855f7', type: 'splitter' },
    { id: 'nm-node-icon-junction', bg: '#f97316', type: 'junction' },
    { id: 'nm-node-icon-customer', bg: '#111827', type: 'customer_premise' },
    { id: 'nm-node-icon-customer-connected', bg: '#16a34a', type: 'customer_premise' },
    { id: 'nm-node-icon-customer-disconnected', bg: '#dc2626', type: 'customer_premise' },
    { id: 'nm-node-icon-customer-neutral', bg: '#111827', type: 'customer_premise' },
  ];
  for (const d of defs) {
    if (!map.hasImage(d.id)) {
      map.addImage(d.id, buildNodeIconImage(d.bg, d.type), { pixelRatio: 2 });
    }
  }
}

export function isCustomerNodeType(nodeType: string) {
  return nodeType === 'customer_endpoint' || nodeType === 'customer_premise';
}

export function getCustomerPppoeVisualState(
  row: Pick<NMNode, 'metadata'> | null | undefined,
): CustomerPppoeVisualState {
  const raw = String(row?.metadata?.pppoe_visual_state || '').trim().toLowerCase();
  if (raw === 'connected' || raw === 'disconnected') return raw;
  return 'neutral';
}

export function getCustomerNodeIconId(state: CustomerPppoeVisualState | string) {
  if (state === 'connected') return 'nm-node-icon-customer-connected';
  if (state === 'disconnected') return 'nm-node-icon-customer-disconnected';
  return 'nm-node-icon-customer-neutral';
}

export function isSystemManagedNode(row: NMNode | null | undefined) {
  return !!row?.metadata?.system_managed;
}

export function systemManagedNodeSourceLabel(row: NMNode | null | undefined) {
  const source = String(row?.metadata?.asset_source || row?.metadata?.asset_type || '').trim();
  if (source === 'mikrotik_router') return 'Router map';
  if (source === 'customer_location') return 'Customer location map';
  return source ? 'Synced asset' : '';
}

function buildSyncedAssetKeySet(rows: NMNode[]) {
  const keys = new Set<string>();
  for (const row of rows || []) {
    const assetType = String(row.metadata?.asset_type || '').trim();
    const assetId = String(row.metadata?.asset_id || '').trim();
    if (assetType && assetId) keys.add(`${assetType}:${assetId}`);
  }
  return keys;
}

export function filterRoutersForOverlay(rows: NMRouter[], nodes: NMNode[]) {
  const syncedKeys = buildSyncedAssetKeySet(nodes);
  return (rows || []).filter((row) => !syncedKeys.has(`mikrotik_router:${row.id}`));
}

function normalizedStatus(value: unknown) {
  return String(value || '')
    .trim()
    .toLowerCase();
}

function metadataNumber(row: NMNode | null | undefined, keys: string[]) {
  for (const key of keys) {
    const raw = row?.metadata?.[key];
    const value = Number(raw);
    if (Number.isFinite(value) && value > 0) return value;
  }
  return 0;
}

function hasRiskStatus(statusRaw: unknown) {
  const status = normalizedStatus(statusRaw);
  return (
    status === 'down' ||
    status === 'degraded' ||
    status === 'maintenance' ||
    status === 'inactive' ||
    status === 'offline' ||
    status === 'critical' ||
    status === 'warning'
  );
}

export function hasServiceMetadata(row: NMNode | null | undefined) {
  return !!(
    row?.metadata?.service_id ||
    row?.metadata?.service_name ||
    row?.metadata?.service_type ||
    row?.metadata?.service_label
  );
}

export function countNodesAtRisk(rows: NMNode[]) {
  return (rows || []).filter((row) => hasRiskStatus(row.status)).length;
}

export function countDegradedLinks(rows: NMLink[]) {
  return (rows || []).filter((row) => {
    if (normalizedStatus(row.status) === 'retired') return false;
    const health = computeLinkHealth(row);
    return health.tone !== 'good';
  }).length;
}

export function countImpactedServices(rows: NMNode[]) {
  let total = 0;
  for (const row of rows || []) {
    const explicitCount = metadataNumber(row, [
      'impacted_services',
      'service_count',
      'services_affected',
      'customer_services',
      'affected_services',
    ]);

    if (explicitCount > 0) {
      total += explicitCount;
      continue;
    }

    if (isCustomerNodeType(row.node_type) && hasRiskStatus(row.status)) {
      total += 1;
    }
  }
  return total;
}

function popupAction(
  key: NetworkMapPopupActionKey,
  label: string,
  tone: NetworkMapPopupActionModel['tone'] = 'secondary',
): NetworkMapPopupActionModel {
  return { key, label, tone };
}

function popupStatusText(status: string) {
  return String(status || '-').trim() || '-';
}

function popupTitleText(value: unknown) {
  const text = String(value || '').trim();
  if (!text) return '';
  return text
    .replaceAll('_', ' ')
    .replace(/\s+/g, ' ')
    .toLowerCase()
    .replace(/\b\w/g, (letter) => letter.toUpperCase())
    .replace(/\bPppoe\b/g, 'PPPoE');
}

function workflowServiceTypeText(value: unknown) {
  const text = String(value || '').trim();
  if (!text || text === '-') return '';
  if (text.toLowerCase().includes('pppoe')) return 'PPPoE';
  return popupTitleText(text);
}

function popupToneFromSubscriptionStatus(statusRaw: unknown): NetworkMapPopupSummaryItem['tone'] {
  const normalized = normalizedStatus(statusRaw);
  if (normalized === 'active' || normalized === 'grace_active') return 'ok';
  if (
    normalized === 'pending_installation' ||
    normalized === 'installation_done_awaiting_payment' ||
    normalized === 'maintenance'
  ) {
    return 'warn';
  }
  return normalized === 'suspended' || normalized === 'inactive' || normalized === 'cancelled'
    ? 'muted'
    : statusTone(String(statusRaw || ''));
}

function popupToneFromRouterState(
  router: Pick<NMRouter, 'is_online' | 'enabled'>,
): NetworkMapPopupModel['tone'] {
  if (router.is_online) return 'ok';
  if (router.enabled) return 'warn';
  return 'muted';
}

function popupStatusFromRouterState(router: Pick<NMRouter, 'is_online' | 'enabled'>) {
  if (router.is_online) return 'online';
  if (router.enabled) return 'offline';
  return 'disabled';
}

function metadataText(row: NMNode | null | undefined, keys: string[]) {
  for (const key of keys) {
    const value = String(row?.metadata?.[key] || '').trim();
    if (value) return value;
  }
  return '';
}

function metadataNumberText(row: NMNode | null | undefined, keys: string[]) {
  const value = metadataNumber(row, keys);
  return value > 0 ? String(value) : '';
}

function normalizePopupValue(value: string | number | null | undefined, fallback = '-') {
  const text = String(value ?? '').trim();
  return text || fallback;
}

function popupContextText(node: NMNode) {
  const managedSource = systemManagedNodeSourceLabel(node);
  if (hasServiceMetadata(node)) return 'Customer service endpoint';
  if (node.node_type === 'odp') return 'ODP distribution point';
  if (node.node_type === 'odc') return 'ODC distribution cabinet';
  if (node.node_type === 'olt') return 'Optical line terminal';
  if (node.node_type === 'pop') return 'Point of presence hub';
  if (node.node_type === 'core') return 'Core backbone node';
  if (node.node_type === 'router') return managedSource || 'Routing node';
  if (node.node_type === 'switch') return 'Switching node';
  if (node.node_type === 'tower') return 'Tower distribution site';
  if (node.node_type === 'ap') return 'Wireless access point';
  if (node.node_type === 'splitter') return 'Optical splitter point';
  if (node.node_type === 'junction') return 'Physical junction node';
  if (isCustomerNodeType(node.node_type)) return 'Customer premise endpoint';
  return managedSource || `${nodeTypeLabel(node.node_type)} asset`;
}

function popupKickerForNode(nodeType: string) {
  const normalized = String(nodeType || '').trim().toLowerCase();
  if (normalized === 'odp') return 'ODP';
  if (normalized === 'odc') return 'ODC';
  if (normalized === 'olt') return 'OLT';
  if (normalized === 'pop') return 'POP';
  if (normalized === 'ap') return 'AP';
  if (normalized === 'core') return 'Core';
  if (normalized === 'customer_premise' || normalized === 'customer_endpoint') return 'Customer';
  const label = nodeTypeLabel(nodeType);
  return label === '-' ? 'Node' : label;
}

function buildNodeSummaryItems(node: NMNode): NetworkMapPopupSummaryItem[] {
  const services = metadataNumberText(node, [
    'service_count',
    'customer_services',
    'affected_services',
    'impacted_services',
  ]);
  const splitters = metadataNumberText(node, ['splitter_count', 'splitters']);
  const customers = metadataNumberText(node, ['customer_count', 'subscriber_count']);
  const ports = metadataNumberText(node, ['port_count', 'ports', 'used_ports']);

  if (node.node_type === 'odp') {
    return [
      { label: 'Services', value: normalizePopupValue(services) },
      { label: 'Splitters', value: normalizePopupValue(splitters) },
    ];
  }

  if (node.node_type === 'odc') {
    return [
      {
        label: 'ODP',
        value: normalizePopupValue(metadataNumberText(node, ['odp_count', 'distribution_points'])),
      },
      { label: 'Services', value: normalizePopupValue(services) },
    ];
  }

  if (node.node_type === 'olt') {
    return [
      { label: 'Ports', value: normalizePopupValue(ports) },
      { label: 'Services', value: normalizePopupValue(services || customers) },
    ];
  }

  if (node.node_type === 'router' || node.node_type === 'switch') {
    return [
      { label: 'Ports', value: normalizePopupValue(ports) },
      { label: 'Services', value: normalizePopupValue(services || customers) },
    ];
  }

  if (node.node_type === 'ap' || node.node_type === 'tower') {
    return [
      { label: 'Clients', value: normalizePopupValue(customers || services) },
      { label: 'Status', value: popupStatusText(node.status), tone: statusTone(node.status) },
    ];
  }

  if (node.node_type === 'splitter') {
    return [
      { label: 'Branches', value: normalizePopupValue(splitters || ports) },
      { label: 'Services', value: normalizePopupValue(services || customers) },
    ];
  }

  if (node.node_type === 'junction') {
    return [
      {
        label: 'Links',
        value: normalizePopupValue(metadataNumberText(node, ['link_count', 'connections'])),
      },
      { label: 'Status', value: popupStatusText(node.status), tone: statusTone(node.status) },
    ];
  }

  if (isCustomerNodeType(node.node_type)) {
    return [
      { label: 'Services', value: normalizePopupValue(services || '1') },
      { label: 'Status', value: popupStatusText(node.status), tone: statusTone(node.status) },
    ];
  }

  return [
    { label: 'Services', value: normalizePopupValue(services || customers) },
    { label: 'Status', value: popupStatusText(node.status), tone: statusTone(node.status) },
  ];
}

function buildNodeDetailPairs(node: NMNode) {
  const managedSource = systemManagedNodeSourceLabel(node);
  const zoneName = metadataText(node, ['zone_name', 'coverage_zone_name', 'zone_label']);
  const parentName = metadataText(node, [
    'parent_node_name',
    'upstream_node_name',
    'parent_name',
    'uplink_name',
  ]);
  const packageName = metadataText(node, ['package_name', 'package_label']);
  const accountName = metadataText(node, ['pppoe_username', 'username', 'account_username']);
  const hostName = metadataText(node, ['host', 'management_host', 'ip_address']);
  const coverageName = metadataText(node, ['coverage_name', 'coverage_area', 'area_name']);

  if (node.node_type === 'odp') {
    return [
      { label: 'Zone', value: normalizePopupValue(zoneName) },
      { label: 'Upstream', value: normalizePopupValue(parentName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
    ];
  }

  if (node.node_type === 'odc') {
    return [
      { label: 'Upstream', value: normalizePopupValue(parentName) },
      { label: 'Zone', value: normalizePopupValue(zoneName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
    ];
  }

  if (hasServiceMetadata(node)) {
    return [
      {
        label: 'Customer',
        value: normalizePopupValue(metadataText(node, ['customer_name', 'customer_label'])),
      },
      { label: 'Package', value: normalizePopupValue(packageName) },
      { label: 'Account', value: normalizePopupValue(accountName) },
      {
        label: 'Service',
        value: normalizePopupValue(
          metadataText(node, ['service_type', 'service_kind']),
          nodeTypeLabel(node.node_type),
        ),
      },
    ];
  }

  if (node.node_type === 'router') {
    return [
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
      { label: 'Host', value: normalizePopupValue(hostName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
      ...(zoneName ? [{ label: 'Zone', value: zoneName }] : []),
    ];
  }

  if (node.node_type === 'switch') {
    return [
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
      { label: 'Zone', value: normalizePopupValue(zoneName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
      ...(parentName ? [{ label: 'Uplink', value: parentName }] : []),
    ];
  }

  if (node.node_type === 'ap' || node.node_type === 'tower') {
    return [
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
      { label: 'Coverage', value: normalizePopupValue(coverageName || zoneName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
      ...(parentName ? [{ label: 'Upstream', value: parentName }] : []),
    ];
  }

  if (node.node_type === 'splitter') {
    return [
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
      { label: 'Upstream', value: normalizePopupValue(parentName) },
      { label: 'Zone', value: normalizePopupValue(zoneName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
    ];
  }

  if (node.node_type === 'junction') {
    return [
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
      { label: 'Zone', value: normalizePopupValue(zoneName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
      ...(parentName ? [{ label: 'Upstream', value: parentName }] : []),
    ];
  }

  if (node.node_type === 'pop' || node.node_type === 'core' || node.node_type === 'olt') {
    return [
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
      { label: 'Zone', value: normalizePopupValue(zoneName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
      ...(parentName ? [{ label: 'Upstream', value: parentName }] : []),
    ];
  }

  if (isCustomerNodeType(node.node_type)) {
    return [
      { label: 'Type', value: nodeTypeLabel(node.node_type) },
      { label: 'Zone', value: normalizePopupValue(zoneName || coverageName) },
      { label: 'Source', value: normalizePopupValue(managedSource) },
      ...(accountName ? [{ label: 'Account', value: accountName }] : []),
    ];
  }

  return [
    { label: 'Type', value: nodeTypeLabel(node.node_type) },
    { label: 'Status', value: popupStatusText(node.status) },
    ...(managedSource ? [{ label: 'Source', value: managedSource }] : []),
    ...(zoneName ? [{ label: 'Zone', value: zoneName }] : []),
  ];
}

export function buildNodePopupModel(node: NMNode): NetworkMapPopupModel {
  return {
    kicker: popupKickerForNode(node.node_type),
    title: String(node.name || node.id || 'Node').trim(),
    subtitle: nodeTypeLabel(node.node_type),
    statusText: popupStatusText(node.status),
    tone: statusTone(node.status),
    contextText: popupContextText(node),
    summaryItems: buildNodeSummaryItems(node),
    detailPairs: buildNodeDetailPairs(node),
    actions: [
      popupAction('connect', 'Connect', 'primary'),
      ...(isSystemManagedNode(node) ? [] : [popupAction('edit', 'Edit')]),
    ],
  };
}

export function buildServicePopupModel(node: NMNode): NetworkMapPopupModel {
  const serviceName = String(
    node.metadata?.service_name ||
      node.metadata?.service_label ||
      node.name ||
      node.id ||
      'Service',
  ).trim();
  const customerName = String(
    node.metadata?.customer_name || node.metadata?.customer_label || '',
  ).trim();
  const serviceType = String(
    node.metadata?.service_type || node.metadata?.service_kind || '',
  ).trim();
  const customerId = String(node.metadata?.customer_id || '').trim();
  const serviceId = String(node.metadata?.service_id || node.metadata?.subscription_id || '').trim();
  const accountName = metadataText(node, ['pppoe_username', 'username', 'account_username']);
  const packageName = metadataText(node, ['package_name', 'package_label', 'service_label']);
  const subscriptionStatusRaw =
    metadataText(node, ['subscription_status', 'service_status']) || String(node.status || '');
  const subscriptionStatusLabel = popupTitleText(popupStatusText(subscriptionStatusRaw));
  const normalizedServiceType = normalizePopupValue(serviceType || nodeTypeLabel(node.node_type));
  const serviceTypeLabel = workflowServiceTypeText(normalizedServiceType);
  const pppState = getCustomerPppoeVisualState(node);
  const pppDisabled = Boolean(node.metadata?.pppoe_disabled);
  const hasPppAccount = !!accountName && accountName !== '-';
  const pppChip =
    pppState === 'connected'
      ? { value: 'PPP Online', tone: 'ok' as const }
      : pppState === 'disconnected'
        ? { value: 'PPP Offline', tone: 'danger' as const }
        : hasPppAccount
          ? { value: pppDisabled ? 'PPP Disabled' : 'PPP Standby', tone: 'muted' as const }
          : { value: 'PPP Belum Ada', tone: 'muted' as const };
  const pppNeedsAttention = pppState !== 'connected';
  const contextText =
    pppState === 'connected'
      ? `${serviceTypeLabel || 'Service'} customer is currently online on Mikrotik.`
      : pppState === 'disconnected'
        ? `${serviceTypeLabel || 'Service'} account exists, but there is no active PPP session on Mikrotik.`
        : hasPppAccount
          ? 'PPPoE account is stored, but access is not active on Mikrotik right now.'
          : 'PPPoE account has not been provisioned on Mikrotik yet.';
  const primaryActionKey = pppNeedsAttention ? 'open-service' : 'open-customer';
  const primaryAction =
    primaryActionKey === 'open-service'
      ? customerId && serviceId
        ? [popupAction('open-service', 'Service', 'primary')]
        : []
      : customerId
        ? [popupAction('open-customer', 'Customer', 'primary')]
        : [];
  const secondaryActions =
    primaryActionKey === 'open-service'
      ? [
          ...(customerId ? [popupAction('open-customer', 'Customer')] : []),
          ...(primaryAction.length === 0 && customerId && serviceId
            ? [popupAction('open-service', 'Service')]
            : []),
        ]
      : [
          ...(customerId && serviceId ? [popupAction('open-service', 'Service')] : []),
          ...(primaryAction.length === 0 && customerId ? [popupAction('open-customer', 'Customer')] : []),
        ];
  const actions = [...primaryAction, ...secondaryActions, popupAction('connect', 'Connect')];

  return {
    variant: 'workflow-service',
    kicker: 'Service',
    title: serviceName,
    subtitle: customerName || nodeTypeLabel(node.node_type),
    statusText: popupStatusText(subscriptionStatusRaw),
    tone: statusTone(node.status),
    contextText,
    statusChips: [
      {
        label: 'Subscription',
        value: subscriptionStatusLabel || popupStatusText(subscriptionStatusRaw),
        tone: popupToneFromSubscriptionStatus(subscriptionStatusRaw),
      },
      {
        label: 'Mikrotik PPP',
        value: pppChip.value,
        tone: pppChip.tone,
      },
    ],
    summaryItems: [
      { label: 'Customer', value: normalizePopupValue(customerName) },
      {
        label: 'Account',
        value: hasPppAccount ? normalizePopupValue(accountName) : 'Belum ada akun PPP',
        tone: pppChip.tone,
      },
    ],
    detailPairs: [
      {
        label: 'Package',
        value: normalizePopupValue(packageName),
      },
      {
        label: 'Service',
        value: normalizedServiceType,
      },
      {
        label: 'Status',
        value: popupStatusText(subscriptionStatusRaw),
      },
    ],
    actions,
  };
}

export function buildLinkPopupModel(link: NMLink): NetworkMapPopupModel {
  const health = computeLinkHealth(link);
  const endpoints = `${String(link.from_node_id || '-')} -> ${String(link.to_node_id || '-')}`;
  const capacity = link.capacity_mbps != null ? `${link.capacity_mbps} Mbps` : '-';
  const latency = link.latency_ms != null ? `${link.latency_ms} ms` : '-';
  const utilization = link.utilization_pct != null ? `${link.utilization_pct}%` : '-';
  const loss = link.loss_db != null ? `${link.loss_db} dB` : '-';
  return {
    kicker: 'Link',
    title: String(link.name || link.id || 'Link').trim(),
    subtitle: endpoints,
    statusText: popupStatusText(link.status),
    tone: health.tone === 'good' ? 'ok' : health.tone === 'warn' ? 'warn' : 'muted',
    contextText: `${String(link.link_type || 'link')} transport path`,
    summaryItems: [
      {
        label: 'Health',
        value: String(health.score),
        tone: health.tone === 'good' ? 'ok' : health.tone === 'warn' ? 'warn' : 'muted',
      },
      { label: 'Capacity', value: capacity },
      { label: 'Latency', value: latency },
    ],
    detailPairs: [
      { label: 'Type', value: normalizePopupValue(link.link_type) },
      { label: 'Endpoints', value: endpoints },
      { label: 'Status', value: popupStatusText(link.status) },
      { label: 'Utilization', value: utilization },
      { label: 'Loss', value: loss },
    ],
    actions: [popupAction('delete', 'Delete', 'danger')],
  };
}

export function buildRouterPopupModel(router: NMRouter): NetworkMapPopupModel {
  const statusText = popupStatusFromRouterState(router);
  const tone = popupToneFromRouterState(router);
  const endpoint = `${router.host}:${router.port}`;
  return {
    kicker: 'Mikrotik',
    title: String(router.identity || router.name || router.id || 'Router').trim(),
    subtitle: String(router.name || router.host || router.id || 'Router').trim(),
    statusText,
    tone,
    contextText: router.is_online
      ? 'Mikrotik control-plane session is reachable for this router.'
      : router.enabled
        ? 'Mikrotik inventory exists, but the device is not responding right now.'
        : 'Mikrotik inventory entry is disabled and will not be polled.',
    summaryItems: [
      {
        label: 'Connectivity',
        value: router.is_online ? 'Live' : router.enabled ? 'Down' : 'Disabled',
        tone,
      },
      {
        label: 'Latency',
        value: router.latency_ms != null ? `${router.latency_ms} ms` : '-',
        tone,
      },
      {
        label: 'Access',
        value: router.enabled ? 'Enabled' : 'Disabled',
        tone: router.enabled ? (router.is_online ? 'ok' : 'warn') : 'muted',
      },
    ],
    detailPairs: [
      { label: 'Endpoint', value: endpoint },
      { label: 'RouterOS', value: normalizePopupValue(router.ros_version) },
      { label: 'Identity', value: normalizePopupValue(router.identity || router.name) },
      { label: 'Inventory', value: normalizePopupValue(router.name) },
    ],
    actions: [popupAction('open-router', 'Open Router', 'primary')],
  };
}

export function findLiveRouterForNode(node: NMNode, routerRows: NMRouter[]): NMRouter | null {
  const assetId = String(node.metadata?.asset_id || '').trim();
  const nodeId = String(node.id || '').trim();
  const nodeName = String(node.name || '').trim().toLowerCase();
  const assetSource = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();

  if (assetSource !== 'mikrotik_router' && node.node_type !== 'router') return null;

  return (
    routerRows.find((row) => String(row.id || '').trim() === assetId) ||
    routerRows.find((row) => String(row.id || '').trim() === nodeId) ||
    routerRows.find((row) => String(row.name || '').trim().toLowerCase() === nodeName) ||
    routerRows.find((row) => String(row.identity || '').trim().toLowerCase() === nodeName) ||
    null
  );
}

export function buildRouterPopupModelFromNode(
  node: NMNode,
  router: NMRouter,
): NetworkMapPopupModel {
  const zoneName = metadataText(node, ['zone_name', 'coverage_zone_name', 'zone_label']);
  const sourceLabel = systemManagedNodeSourceLabel(node);
  const baseModel = buildRouterPopupModel(router);

  return {
    ...baseModel,
    subtitle: String(node.name || baseModel.subtitle).trim(),
    detailPairs: [
      ...baseModel.detailPairs,
      ...(sourceLabel ? [{ label: 'Source', value: sourceLabel }] : []),
      ...(zoneName ? [{ label: 'Zone', value: zoneName }] : []),
    ],
  };
}

export function buildZonePopupModel(zone: NMZone): NetworkMapPopupModel {
  return {
    kicker: 'Zone',
    title: String(zone.name || zone.id || 'Zone').trim(),
    subtitle: String(zone.zone_type || '-'),
    statusText: popupStatusText(zone.status),
    tone: statusTone(zone.status),
    contextText:
      normalizedStatus(zone.status) === 'active'
        ? 'Zone coverage is available'
        : `Zone status is ${popupStatusText(zone.status).toLowerCase()}`,
    summaryItems: [
      { label: 'Status', value: popupStatusText(zone.status), tone: statusTone(zone.status) },
    ],
    detailPairs: [
      { label: 'Type', value: String(zone.zone_type || '-') },
      { label: 'Status', value: popupStatusText(zone.status) },
    ],
    actions: [],
  };
}

export function summarizeZoneRisk(rows: NMZone[]) {
  const byStatus: Record<string, number> = {};
  let atRisk = 0;

  for (const row of rows || []) {
    const status = normalizedStatus(row.status) || 'unknown';
    byStatus[status] = (byStatus[status] || 0) + 1;
    if (hasRiskStatus(status)) atRisk += 1;
  }

  return {
    total: (rows || []).length,
    atRisk,
    healthy: Math.max(0, (rows || []).length - atRisk),
    byStatus,
  };
}

export function computeLinkHealth(row: NMLink): { score: number; tone: 'good' | 'warn' | 'bad' } {
  const statusRaw = String(row.status || '').toLowerCase();
  if (statusRaw === 'down' || statusRaw === 'retired') return { score: 5, tone: 'bad' };
  let score = 100;

  if (statusRaw === 'maintenance') score -= 32;
  if (statusRaw === 'degraded') score -= 20;
  if (statusRaw === 'planning') score -= 10;
  if (statusRaw === 'inactive') score -= 12;

  const util = row.utilization_pct ?? null;
  const latency = row.latency_ms ?? null;
  const loss = row.loss_db ?? null;
  if (util != null) {
    if (util >= 90) score -= 40;
    else if (util >= 75) score -= 20;
    else if (util >= 60) score -= 10;
  }
  if (latency != null) {
    if (latency > 40) score -= 15;
    else if (latency > 20) score -= 8;
  }
  if (loss != null) {
    if (loss > 3) score -= 25;
    else if (loss > 1) score -= 12;
    else if (loss > 0.3) score -= 6;
  }

  score = Math.max(0, Math.min(100, score));
  const tone: 'good' | 'warn' | 'bad' = score >= 80 ? 'good' : score >= 60 ? 'warn' : 'bad';
  return { score, tone };
}

export function nodesToFeatureCollection(rows: NMNode[]): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: (rows || []).map((row) => ({
      type: 'Feature',
      geometry: { type: 'Point', coordinates: [row.lng, row.lat] },
      properties: {
        id: row.id,
        name: row.name,
        node_type: row.node_type,
        status: row.status,
        system_managed: !!row.metadata?.system_managed,
        asset_source: String(row.metadata?.asset_source || ''),
      },
    })),
  };
}

export function linksToFeatureCollection(rows: NMLink[]): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: (rows || []).map((row) => {
      const health = computeLinkHealth(row);
      return {
        type: 'Feature',
        geometry: row.geometry,
        properties: {
          id: row.id,
          name: row.name,
          link_type: row.link_type,
          status: row.status,
          health_score: health.score,
          health_tone: health.tone,
        },
      };
    }),
  };
}

export function customersToFeatureCollection(rows: NMNode[]): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: (rows || [])
      .filter((row) => isCustomerNodeType(row.node_type))
      .map((row) => ({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: [row.lng, row.lat] },
        properties: {
          id: row.id,
          name: row.name,
          node_type: row.node_type,
          status: row.status,
          pppoe_visual_state: getCustomerPppoeVisualState(row),
          system_managed: !!row.metadata?.system_managed,
          asset_source: String(row.metadata?.asset_source || ''),
        },
      })),
  };
}

export function routersToFeatureCollection(rows: NMRouter[]): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: (rows || [])
      .filter((row) => row.latitude != null && row.longitude != null)
      .map((row) => ({
        type: 'Feature',
        geometry: {
          type: 'Point',
          coordinates: [Number(row.longitude), Number(row.latitude)],
        },
        properties: {
          id: row.id,
          name: row.name,
          host: row.host,
          port: row.port,
          is_online: !!row.is_online,
          latency_ms: row.latency_ms ?? null,
        },
      })),
  };
}

export function zonesToFeatureCollection(rows: NMZone[]): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: (rows || []).map((row) => ({
      type: 'Feature',
      geometry: row.geometry,
      properties: {
        id: row.id,
        name: row.name,
        zone_type: row.zone_type,
        status: row.status,
      },
    })),
  };
}

export function asNumber(input: string): number | undefined {
  const value = Number.parseFloat(input);
  return Number.isFinite(value) ? value : undefined;
}

export function prettyGeometry(value: Geometry): string {
  return JSON.stringify(value, null, 2);
}

export function parseGeometryText(text: string): Geometry {
  const parsed = JSON.parse(text || '{}');
  if (!parsed || typeof parsed !== 'object' || !('type' in parsed)) {
    throw new Error('Geometry JSON is invalid');
  }
  return parsed as Geometry;
}
