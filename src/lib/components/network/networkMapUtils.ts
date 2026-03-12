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

export type LinkFieldConfig = {
  capacityLabel: string;
  utilizationLabel: string;
  latencyLabel: string;
  lossLabel: string;
  showLoss: boolean;
  helper: string;
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
    { id: 'nm-node-icon-customer', bg: '#06b6d4', type: 'customer_premise' },
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
