import type { Geometry } from 'geojson';
import { asNumber, parseGeometryText, type LinkFieldConfig } from './networkMapUtils';
import {
  createNetworkZoneBinding,
  deleteNetworkLink,
  deleteNetworkNode,
  deleteNetworkZone,
  deleteNetworkZoneBinding,
  loadNetworkZoneBindings,
  saveNetworkLink,
  saveNetworkNode,
  saveNetworkZone,
} from './networkMapActions';
import { toast } from '$lib/stores/toast';

export async function submitNodeCrud(args: {
  editingNodeId: string | null;
  nodeForm: {
    name: string;
    node_type: string;
    status: string;
    lat: string;
    lng: string;
  };
}) {
  const lat = asNumber(args.nodeForm.lat);
  const lng = asNumber(args.nodeForm.lng);
  if (!args.nodeForm.name.trim() || !args.nodeForm.node_type.trim() || lat === undefined || lng === undefined) {
    toast.error('Please fill name, type, latitude and longitude');
    return false;
  }

  return await saveNetworkNode(args.editingNodeId, {
    name: args.nodeForm.name.trim(),
    node_type: args.nodeForm.node_type.trim(),
    status: args.nodeForm.status || 'active',
    lat,
    lng,
  });
}

export async function submitLinkCrud(args: {
  editingLinkId: string | null;
  linkForm: {
    name: string;
    link_type: string;
    status: string;
    from_node_id: string;
    to_node_id: string;
    priority: string;
    capacity_mbps: string;
    utilization_pct: string;
    loss_db: string;
    latency_ms: string;
    geometryText: string;
  };
  linkFieldConfig: LinkFieldConfig;
  hasExistingLinkBetweenNodes: (fromNodeId: string, toNodeId: string, excludeLinkId?: string | null) => boolean;
}) {
  if (!args.linkForm.name.trim() || !args.linkForm.from_node_id || !args.linkForm.to_node_id) {
    toast.error('Please fill name and endpoint nodes');
    return false;
  }
  if (args.linkForm.from_node_id === args.linkForm.to_node_id) {
    toast.error('Source and destination must be different nodes.');
    return false;
  }
  if (
    args.hasExistingLinkBetweenNodes(
      args.linkForm.from_node_id,
      args.linkForm.to_node_id,
      args.editingLinkId,
    )
  ) {
    toast.error('A link between these nodes already exists.');
    return false;
  }

  let geometry: Geometry;
  try {
    geometry = parseGeometryText(args.linkForm.geometryText);
  } catch (e: any) {
    toast.error(e?.message || 'Geometry JSON is invalid');
    return false;
  }

  return await saveNetworkLink(args.editingLinkId, {
    name: args.linkForm.name.trim(),
    link_type: args.linkForm.link_type || 'fiber',
    status: args.linkForm.status || 'up',
    from_node_id: args.linkForm.from_node_id,
    to_node_id: args.linkForm.to_node_id,
    priority: Number.parseInt(args.linkForm.priority || '100', 10),
    capacity_mbps: asNumber(args.linkForm.capacity_mbps),
    utilization_pct: asNumber(args.linkForm.utilization_pct),
    loss_db: args.linkFieldConfig.showLoss ? asNumber(args.linkForm.loss_db) : null,
    latency_ms: asNumber(args.linkForm.latency_ms),
    geometry,
  });
}

export async function submitZoneCrud(args: {
  editingZoneId: string | null;
  zoneForm: {
    name: string;
    zone_type: string;
    status: string;
    priority: string;
    geometryText: string;
  };
}) {
  if (!args.zoneForm.name.trim()) {
    toast.error('Zone name is required');
    return false;
  }

  let geometry: Geometry;
  try {
    geometry = parseGeometryText(args.zoneForm.geometryText);
  } catch (e: any) {
    toast.error(e?.message || 'Geometry JSON is invalid');
    return false;
  }

  return await saveNetworkZone(args.editingZoneId, {
    name: args.zoneForm.name.trim(),
    zone_type: args.zoneForm.zone_type || 'coverage',
    status: args.zoneForm.status || 'active',
    priority: Number.parseInt(args.zoneForm.priority || '100', 10),
    geometry,
  });
}

export async function removeCrud(args: {
  type: 'node' | 'link' | 'zone' | 'binding';
  id: string;
}) {
  if (args.type === 'node') return await deleteNetworkNode(args.id);
  if (args.type === 'link') return await deleteNetworkLink(args.id);
  if (args.type === 'zone') return await deleteNetworkZone(args.id);
  return await deleteNetworkZoneBinding(args.id);
}

export async function loadZoneBindingsCrud(selectedZoneId: string) {
  return await loadNetworkZoneBindings(selectedZoneId);
}

export async function createZoneBindingCrud(args: {
  zone_id: string;
  node_id: string;
  is_primary: boolean;
  weight: string;
}) {
  if (!args.zone_id || !args.node_id) {
    toast.error('Please choose zone and node');
    return false;
  }

  return await createNetworkZoneBinding({
    zone_id: args.zone_id,
    node_id: args.node_id,
    is_primary: args.is_primary,
    weight: Number.parseInt(args.weight || '100', 10),
  });
}
