import type { Geometry } from 'geojson';
import { api } from '$lib/api/client';
import { toast } from '$lib/stores/toast';
import type { NMLink, NMZone } from './networkMapUtils';

export async function saveNetworkNode(
  editingNodeId: string | null,
  payload: {
    name: string;
    node_type: string;
    status: string;
    lat: number;
    lng: number;
  },
) {
  try {
    if (editingNodeId) {
      await api.networkMapping.nodes.update(editingNodeId, payload);
    } else {
      await api.networkMapping.nodes.create(payload);
    }
    toast.success(editingNodeId ? 'Node updated' : 'Node created');
    return true;
  } catch (e: any) {
    toast.error(e?.message || 'Failed to save node');
    return false;
  }
}

export async function saveNetworkLink(
  editingLinkId: string | null,
  payload: {
    name: string;
    link_type: string;
    status: string;
    from_node_id: string;
    to_node_id: string;
    priority: number;
    capacity_mbps?: number;
    utilization_pct?: number;
    loss_db?: number | null;
    latency_ms?: number;
    geometry: Geometry;
  },
) {
  try {
    if (editingLinkId) {
      await api.networkMapping.links.update(editingLinkId, payload);
    } else {
      await api.networkMapping.links.create(payload);
    }
    toast.success(editingLinkId ? 'Link updated' : 'Link created');
    return true;
  } catch (e: any) {
    toast.error(e?.message || 'Failed to save link');
    return false;
  }
}

export async function saveNetworkZone(
  editingZoneId: string | null,
  payload: {
    name: string;
    zone_type: string;
    status: string;
    priority: number;
    geometry: Geometry;
  },
) {
  try {
    if (editingZoneId) {
      await api.networkMapping.zones.update(editingZoneId, payload);
    } else {
      await api.networkMapping.zones.create(payload);
    }
    toast.success(editingZoneId ? 'Zone updated' : 'Zone created');
    return true;
  } catch (e: any) {
    toast.error(e?.message || 'Failed to save zone');
    return false;
  }
}

export async function deleteNetworkNode(id: string) {
  try {
    await api.networkMapping.nodes.delete(id);
    toast.success('Node deleted');
    return true;
  } catch (e: any) {
    toast.error(e?.message || 'Failed to delete node');
    return false;
  }
}

export async function deleteNetworkLink(id: string) {
  try {
    await api.networkMapping.links.delete(id);
    toast.success('Link deleted');
    return true;
  } catch (e: any) {
    toast.error(e?.message || 'Failed to delete link');
    return false;
  }
}

export async function deleteNetworkZone(id: string) {
  try {
    await api.networkMapping.zones.delete(id);
    toast.success('Zone deleted');
    return true;
  } catch (e: any) {
    toast.error(e?.message || 'Failed to delete zone');
    return false;
  }
}

export async function loadNetworkZoneBindings(selectedZoneId: string) {
  try {
    return (await api.networkMapping.zoneBindings.list({
      zone_id: selectedZoneId || undefined,
    })) || [];
  } catch (e: any) {
    toast.error(e?.message || 'Failed to load zone bindings');
    return null;
  }
}

export async function createNetworkZoneBinding(payload: {
  zone_id: string;
  node_id: string;
  is_primary: boolean;
  weight: number;
}) {
  try {
    await api.networkMapping.zoneBindings.create(payload);
    toast.success('Binding created');
    return true;
  } catch (e: any) {
    toast.error(e?.message || 'Failed to create binding');
    return false;
  }
}

export async function deleteNetworkZoneBinding(id: string) {
  try {
    await api.networkMapping.zoneBindings.delete(id);
    toast.success('Binding deleted');
    return true;
  } catch (e: any) {
    toast.error(e?.message || 'Failed to delete binding');
    return false;
  }
}

export function buildLinkDraftForm(row: NMLink, fallbackGeometry: Geometry) {
  return {
    name: row.name || '',
    link_type: row.link_type || 'fiber',
    status: row.status || 'active',
    from_node_id: row.from_node_id || '',
    to_node_id: row.to_node_id || '',
    priority: String(row.priority ?? 100),
    capacity_mbps: row.capacity_mbps == null ? '' : String(row.capacity_mbps),
    utilization_pct: row.utilization_pct == null ? '' : String(row.utilization_pct),
    loss_db: row.loss_db == null ? '' : String(row.loss_db),
    latency_ms: row.latency_ms == null ? '' : String(row.latency_ms),
    geometry: row.geometry || fallbackGeometry,
  };
}

export function buildZoneDraftForm(row: NMZone, fallbackGeometry: Geometry) {
  return {
    name: row.name || '',
    zone_type: row.zone_type || 'coverage',
    status: row.status || 'active',
    priority: '100',
    geometry: row.geometry || fallbackGeometry,
  };
}
