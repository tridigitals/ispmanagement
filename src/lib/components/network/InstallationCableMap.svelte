<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type NMNode = {
    id: string;
    name: string;
    node_type: string;
    status: string;
    lat: number;
    lng: number;
    metadata?: Record<string, any>;
  };

  type NMLink = {
    id: string;
    name: string;
    link_type: string;
    status: string;
    from_node_id?: string;
    to_node_id?: string;
    geometry: GeoJSON.Geometry;
    metadata?: Record<string, any>;
  };

  type LinkSnapTarget = {
    linkId: string;
    linkName: string;
    lng: number;
    lat: number;
  };

  type CableDraftState = {
    version: 1;
    drawing: boolean;
    targetNodeId: string;
    targetLink: LinkSnapTarget | null;
    bendPoints: Array<[number, number]>;
    linkName: string;
  };

  const dispatch = createEventDispatcher<{
    saved: { link_id?: string; from_node_id: string; to_node_id: string };
    closed: {};
  }>();

  export let workOrderId: string;
  export let customerId: string;
  export let locationId: string;
  export let preferredTargetNodeId: string | null = null;

  let hostEl: HTMLDivElement | null = null;
  let map: import('maplibre-gl').Map | null = null;
  let maplibre: typeof import('maplibre-gl') | null = null;
  let loading = true;
  let saving = false;
  let drawing = false;
  let sourceNodeId = '';
  let targetNodeId = '';
  let linkName = '';
  let bendPoints: Array<[number, number]> = [];
  let nodes: NMNode[] = [];
  let links: NMLink[] = [];
  let customerCoord: [number, number] | null = null;
  let customerMarker: import('maplibre-gl').Marker | null = null;
  let customerLocation: { id: string; label?: string | null; latitude?: number | null; longitude?: number | null } | null = null;
  let targetLink: LinkSnapTarget | null = null;
  let autoSyncAttempted = false;
  let directCreateAttempted = false;
  let suppressNextMapClick = false;
  let draftRestored = false;

  const SOURCE_NODES = 'icm-nodes';
  const SOURCE_LINKS = 'icm-links';
  const SOURCE_DRAFT = 'icm-draft';
  const SOURCE_DRAFT_POINTS = 'icm-draft-points';
  const DRAFT_STORAGE_VERSION = 1 as const;

  onMount(() => {
    void init();
  });

  onDestroy(() => {
    customerMarker?.remove();
    map?.remove();
  });

  async function init() {
    try {
      const mod = await import('maplibre-gl');
      maplibre = mod;
      if (!hostEl) return;

      map = new mod.Map({
        container: hostEl,
        style: {
          version: 8,
          glyphs: 'https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf',
          sources: {
            osm: {
              type: 'raster',
              tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
              tileSize: 256,
            },
          },
          layers: [{ id: 'base', type: 'raster', source: 'osm' }],
        },
        center: [106.8456, -6.2088],
        zoom: 11,
      });

      map.addControl(new mod.NavigationControl({ visualizePitch: true }), 'top-right');

      map.on('load', async () => {
        if (!map) return;
        map.addSource(SOURCE_LINKS, { type: 'geojson', data: emptyFC() });
        map.addSource(SOURCE_NODES, { type: 'geojson', data: emptyFC() });
        map.addSource(SOURCE_DRAFT, { type: 'geojson', data: emptyFC() });
        map.addSource(SOURCE_DRAFT_POINTS, { type: 'geojson', data: emptyFC() });

        map.addLayer({
          id: 'icm-links',
          type: 'line',
          source: SOURCE_LINKS,
          paint: { 'line-color': '#3f8cff', 'line-width': 2.5, 'line-opacity': 0.9 },
        });

        map.addLayer({
          id: 'icm-nodes',
          type: 'circle',
          source: SOURCE_NODES,
          paint: {
            'circle-radius': ['interpolate', ['linear'], ['zoom'], 9, 6, 14, 9],
            'circle-color': [
              'match',
              ['get', 'node_type'],
              'customer_endpoint',
              '#06b6d4',
              'customer_premise',
              '#06b6d4',
              'junction',
              '#f97316',
              '#16a34a',
            ],
            'circle-stroke-color': '#e2e8f0',
            'circle-stroke-width': 1.4,
          },
        });

        map.addLayer({
          id: 'icm-draft',
          type: 'line',
          source: SOURCE_DRAFT,
          paint: {
            'line-color': '#38bdf8',
            'line-width': 3,
            'line-opacity': 0.95,
            'line-dasharray': [1.4, 1.2],
          },
        });

        map.addLayer({
          id: 'icm-draft-points',
          type: 'circle',
          source: SOURCE_DRAFT_POINTS,
          paint: {
            'circle-radius': [
              'case',
              ['==', ['get', 'kind'], 'target-link'],
              6,
              ['==', ['get', 'kind'], 'source'],
              6,
              4.5,
            ],
            'circle-color': [
              'case',
              ['==', ['get', 'kind'], 'target-link'],
              '#fb7185',
              ['==', ['get', 'kind'], 'source'],
              '#22d3ee',
              '#38bdf8',
            ],
            'circle-stroke-color': '#0b1020',
            'circle-stroke-width': 1.2,
          },
        });

        map.on('click', 'icm-nodes', (e) => {
          if (!drawing || !e.features?.[0]) return;
          suppressNextMapClick = true;
          const nodeId = String(e.features[0].properties?.id || '');
          if (!nodeId) return;
          if (nodeId === sourceNodeId) {
            toast.info('Cable always starts from the customer premise marker.');
            return;
          }
          targetNodeId = nodeId;
          targetLink = null;
          const fromName = nodes.find((n) => n.id === sourceNodeId)?.name || 'Customer';
          const toName = nodes.find((n) => n.id === targetNodeId)?.name || 'Destination';
          if (!linkName.trim()) linkName = `Cable ${fromName} -> ${toName}`;
          refreshDraft();
          toast.success('Destination node selected. Save to create the cable route.');
        });

        map.on('click', 'icm-links', (e) => {
          if (!drawing || !e.features?.[0]) return;
          suppressNextMapClick = true;
          const linkId = String(e.features[0].properties?.id || '');
          const row = links.find((link) => link.id === linkId);
          if (!row) return;
          const snapped = snapPointToGeometry(row.geometry, e.lngLat.lng, e.lngLat.lat);
          if (!snapped) {
            toast.error('Failed to snap to selected cable. Try another point.');
            return;
          }
          targetNodeId = '';
          targetLink = {
            linkId: row.id,
            linkName: row.name || 'Existing cable',
            lng: snapped[0],
            lat: snapped[1],
          };
          const fromName = nodes.find((n) => n.id === sourceNodeId)?.name || 'Customer';
          if (!linkName.trim()) linkName = `Cable ${fromName} -> ${targetLink.linkName}`;
          refreshDraft();
          toast.success('Connection point selected on existing cable. Save will create a junction automatically.');
        });

        map.on('click', (e) => {
          if (!drawing || !sourceNodeId) return;
          if (suppressNextMapClick) {
            suppressNextMapClick = false;
            return;
          }
          const hitFeatureCount =
            map?.queryRenderedFeatures(e.point, {
              layers: ['icm-nodes', 'icm-links'],
            }).length || 0;
          if (hitFeatureCount > 0) return;
          bendPoints = [...bendPoints, [e.lngLat.lng, e.lngLat.lat]];
          refreshDraft();
        });

        await loadData();
      });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to initialize cable map');
    } finally {
      loading = false;
    }
  }

  async function fetchTopologyData() {
    const [nodesRes, linksRes] = await Promise.all([
      api.networkMapping.nodes.list({ page: 1, per_page: 1000 }),
      api.networkMapping.links.list({ page: 1, per_page: 1000 }),
    ]);
    return {
      nodes: (nodesRes.data || []) as NMNode[],
      links: (linksRes.data || []) as NMLink[],
    };
  }

  async function fetchCustomerLocation() {
    const locations = await api.customers.locations.list(customerId);
    const loc = (locations || []).find((x) => x.id === locationId) || null;
    customerLocation = loc || null;
    return loc;
  }

  function draftStorageKey() {
    const workOrderKey = String(workOrderId || '').trim() || 'unknown-work-order';
    return `installation-cable-draft:${workOrderKey}`;
  }

  function persistDraft() {
    if (typeof window === 'undefined') return;
    try {
      const hasMeaningfulDraft =
        drawing ||
        bendPoints.length > 0 ||
        !!targetNodeId ||
        !!targetLink ||
        !!linkName.trim();
      if (!hasMeaningfulDraft) {
        window.localStorage.removeItem(draftStorageKey());
        return;
      }
      const payload: CableDraftState = {
        version: DRAFT_STORAGE_VERSION,
        drawing,
        targetNodeId,
        targetLink,
        bendPoints,
        linkName,
      };
      window.localStorage.setItem(draftStorageKey(), JSON.stringify(payload));
    } catch {
      // non-blocking
    }
  }

  function clearDraftPersistence() {
    if (typeof window === 'undefined') return;
    try {
      window.localStorage.removeItem(draftStorageKey());
    } catch {
      // non-blocking
    }
  }

  function restoreDraft(rows: NMNode[], availableLinks: NMLink[]) {
    if (draftRestored || typeof window === 'undefined') return;
    draftRestored = true;
    try {
      const raw = window.localStorage.getItem(draftStorageKey());
      if (!raw) return;
      const parsed = JSON.parse(raw) as Partial<CableDraftState> | null;
      if (!parsed || parsed.version !== DRAFT_STORAGE_VERSION) return;

      const nextTargetNodeId = String(parsed.targetNodeId || '').trim();
      const restoredTargetNodeId =
        nextTargetNodeId && rows.some((row) => row.id === nextTargetNodeId) ? nextTargetNodeId : '';

      let restoredTargetLink: LinkSnapTarget | null = null;
      if (parsed.targetLink) {
        const candidate = parsed.targetLink;
        if (
          candidate &&
          typeof candidate.linkId === 'string' &&
          availableLinks.some((link) => link.id === candidate.linkId) &&
          Number.isFinite(Number(candidate.lng)) &&
          Number.isFinite(Number(candidate.lat))
        ) {
          restoredTargetLink = {
            linkId: candidate.linkId,
            linkName: String(candidate.linkName || 'Existing cable'),
            lng: Number(candidate.lng),
            lat: Number(candidate.lat),
          };
        }
      }

      bendPoints = Array.isArray(parsed.bendPoints)
        ? parsed.bendPoints
            .filter(
              (point) =>
                Array.isArray(point) &&
                point.length >= 2 &&
                Number.isFinite(Number(point[0])) &&
                Number.isFinite(Number(point[1])),
            )
            .map((point) => [Number(point[0]), Number(point[1])] as [number, number])
        : [];
      linkName = String(parsed.linkName || '').trim();
      targetNodeId = restoredTargetLink ? '' : restoredTargetNodeId;
      targetLink = restoredTargetLink;
      drawing = !!parsed.drawing;
    } catch {
      clearDraftPersistence();
    }
  }

  function resolveCustomerNode(rows: NMNode[]): NMNode | null {
    const exact = rows.find(
      (row) =>
        (row.node_type === 'customer_premise' || row.node_type === 'customer_endpoint') &&
        String(row.metadata?.location_id || '').trim() === locationId,
    );
    if (exact) return exact;
    return (
      rows.find(
        (row) =>
          row.node_type === 'customer_premise' &&
          String(row.metadata?.customer_id || '').trim() === customerId,
      ) || null
    );
  }

  function resolvePreferredTarget(rows: NMNode[], sourceId: string): string {
    const candidate = String(preferredTargetNodeId || '').trim();
    if (!candidate || candidate === sourceId) return '';
    return rows.some((row) => row.id === candidate) ? candidate : '';
  }

  async function loadData() {
    if (!map || !maplibre) return;
    try {
      let fetched = await fetchTopologyData();
      let customerNode = resolveCustomerNode(fetched.nodes);

      if (!customerNode && !autoSyncAttempted) {
        autoSyncAttempted = true;
        try {
          await api.networkMapping.assets.sync();
          fetched = await fetchTopologyData();
          customerNode = resolveCustomerNode(fetched.nodes);
        } catch (e: any) {
          toast.error(e?.message || 'Automatic topology sync failed while preparing customer premise node.');
        }
      }

      if (!customerNode && !directCreateAttempted) {
        directCreateAttempted = true;
        try {
          const loc = await fetchCustomerLocation();
          const lat = Number(loc?.latitude);
          const lng = Number(loc?.longitude);
          if (loc && Number.isFinite(lat) && Number.isFinite(lng)) {
            await api.networkMapping.nodes.create({
              name: String(loc.label || 'Customer Premise').trim() || 'Customer Premise',
              node_type: 'customer_premise',
              status: 'active',
              lat,
              lng,
              metadata: {
                system_managed: true,
                asset_source: 'customer_location',
                asset_type: 'customer_location',
                asset_id: loc.id,
                location_id: loc.id,
                customer_id: customerId,
                location_label: loc.label || 'Customer Premise',
                generated_by: 'installation_cable_map',
              },
            });
            fetched = await fetchTopologyData();
            customerNode = resolveCustomerNode(fetched.nodes);
          }
        } catch (e: any) {
          toast.error(e?.message || 'Automatic customer premise node creation failed.');
        }
      }

      nodes = fetched.nodes;
      links = fetched.links;
      sourceNodeId = customerNode?.id || '';
      restoreDraft(nodes, links);
      if (!drawing) {
        targetNodeId = '';
        targetLink = null;
      } else if (!targetNodeId && !targetLink) {
        targetNodeId = resolvePreferredTarget(nodes, sourceNodeId);
      }

      setSource(SOURCE_NODES, {
        type: 'FeatureCollection',
        features: nodes
          .filter((n) => Number.isFinite(n.lng) && Number.isFinite(n.lat))
          .map((n) => ({
            type: 'Feature',
            geometry: { type: 'Point', coordinates: [n.lng, n.lat] },
            properties: { id: n.id, name: n.name, node_type: n.node_type, status: n.status },
          })),
      });
      setSource(SOURCE_LINKS, {
        type: 'FeatureCollection',
        features: links.map((l) => ({
          type: 'Feature',
          geometry: l.geometry,
          properties: { id: l.id, name: l.name, status: l.status, link_type: l.link_type },
        })),
      });

      await placeCustomerMarker();
      refreshDraft();
      fitToCustomerOrNetwork();

      if (!sourceNodeId) {
        if (!customerLocation) {
          try {
            await fetchCustomerLocation();
          } catch {
            // best effort only
          }
        }
        const hasMapPoint =
          Number.isFinite(Number(customerLocation?.longitude)) &&
          Number.isFinite(Number(customerLocation?.latitude));
        toast.error(
          hasMapPoint
            ? 'Customer premise node could not be prepared automatically. Check topology permission or node creation errors.'
            : 'Customer location does not have map coordinates yet, so customer premise node cannot be created.',
        );
      }
      persistDraft();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load map data');
    }
  }

  async function placeCustomerMarker() {
    if (!map || !maplibre || !customerId || !locationId) return;
    try {
      const loc = customerLocation || (await fetchCustomerLocation());
      if (!loc || !Number.isFinite(loc.longitude) || !Number.isFinite(loc.latitude)) return;
      customerCoord = [Number(loc.longitude), Number(loc.latitude)];
      customerMarker?.remove();
      customerMarker = new maplibre.Marker({ color: '#06b6d4', scale: 1.12 })
        .setLngLat(customerCoord)
        .setPopup(
          new maplibre.Popup({ offset: 10 }).setHTML(
            `<div class="icm-popup"><strong>${escapeHtml(loc.label || 'Customer')}</strong></div>`,
          ),
        )
        .addTo(map);
    } catch {
      // ignore location fetch failures
    }
  }

  function fitToCustomerOrNetwork() {
    if (!map || !maplibre) return;
    if (customerCoord) {
      map.easeTo({ center: customerCoord, zoom: 16, duration: 420 });
      return;
    }
    const pts = nodes
      .filter((n) => Number.isFinite(n.lng) && Number.isFinite(n.lat))
      .map((n) => [n.lng, n.lat] as [number, number]);
    if (pts.length === 0) return;
    const bounds = pts.reduce(
      (acc, p) => acc.extend(p),
      new maplibre.LngLatBounds(pts[0], pts[0]),
    );
    map.fitBounds(bounds, {
      padding: { top: 48, right: 48, bottom: 48, left: 48 },
      maxZoom: 15,
      duration: 460,
    });
  }

  function toggleDraw() {
    if (drawing) {
      cancelDraw();
      return;
    }
    if (!sourceNodeId) {
      toast.error('Customer premise node is required before drawing. Run topology sync first.');
      return;
    }
    drawing = true;
    bendPoints = [];
    targetLink = null;
    targetNodeId = resolvePreferredTarget(nodes, sourceNodeId);
    refreshDraft();
    persistDraft();
    toast.info(
      targetNodeId
        ? 'Route starts from the customer premise. Add bend points, then save or choose another target.'
        : 'Route starts from the customer premise. Add bend points, then click a node or an existing cable.',
    );
  }

  function cancelDraw() {
    drawing = false;
    targetNodeId = '';
    targetLink = null;
    bendPoints = [];
    linkName = '';
    refreshDraft();
    clearDraftPersistence();
  }

  function undoPoint() {
    if (bendPoints.length === 0) return;
    bendPoints = bendPoints.slice(0, -1);
    refreshDraft();
    persistDraft();
  }

  function getNodeCoord(nodeId: string): [number, number] | null {
    const n = nodes.find((x) => x.id === nodeId);
    if (!n) return null;
    if (!Number.isFinite(n.lng) || !Number.isFinite(n.lat)) return null;
    return [n.lng, n.lat];
  }

  function buildDraftLineCoords(): Array<[number, number]> {
    const from = getNodeCoord(sourceNodeId);
    if (!from) return [];
    const coords: Array<[number, number]> = [from, ...bendPoints];
    if (targetLink) {
      coords.push([targetLink.lng, targetLink.lat]);
      return coords;
    }
    if (targetNodeId) {
      const to = getNodeCoord(targetNodeId);
      if (to) coords.push(to);
    }
    return coords;
  }

  function refreshDraft() {
    const coords = buildDraftLineCoords();
    const draftPointFeatures: GeoJSON.Feature[] = bendPoints.map((p) => ({
      type: 'Feature',
      geometry: { type: 'Point', coordinates: p },
      properties: { kind: 'bend' },
    }));

    const sourceCoord = getNodeCoord(sourceNodeId);
    if (drawing && sourceCoord) {
      draftPointFeatures.unshift({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: sourceCoord },
        properties: { kind: 'source' },
      });
    }
    if (drawing && targetLink) {
      draftPointFeatures.push({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: [targetLink.lng, targetLink.lat] },
        properties: { kind: 'target-link' },
      });
    }

    setSource(SOURCE_DRAFT, {
      type: 'FeatureCollection',
      features:
        coords.length >= 2
          ? [{ type: 'Feature', geometry: { type: 'LineString', coordinates: coords }, properties: {} }]
          : [],
    });
    setSource(SOURCE_DRAFT_POINTS, {
      type: 'FeatureCollection',
      features: draftPointFeatures,
    });
    persistDraft();
  }

  function defaultConnectionName(): string {
    const fromName = nodes.find((n) => n.id === sourceNodeId)?.name || 'Customer';
    if (targetLink) return `Cable ${fromName} -> ${targetLink.linkName}`;
    const toName = nodes.find((n) => n.id === targetNodeId)?.name || 'Destination';
    return `Cable ${fromName} -> ${toName}`;
  }

  async function saveLink() {
    if (!sourceNodeId) {
      toast.error('Customer premise node is missing. Sync topology assets first.');
      return;
    }
    if (!targetNodeId && !targetLink) {
      toast.error('Pick a destination node or an existing cable first.');
      return;
    }
    const coords = buildDraftLineCoords();
    if (coords.length < 2) {
      toast.error('Draw a valid cable path first.');
      return;
    }

    saving = true;
    try {
      const baseName = linkName.trim() || defaultConnectionName();
      if (targetLink) {
        const result = await api.networkMapping.links.connectNodeToLink({
          source_node_id: sourceNodeId,
          target_link_id: targetLink.linkId,
          name: baseName,
          link_type: 'fiber',
          status: 'up',
          priority: 100,
          geometry: { type: 'LineString', coordinates: coords },
          split_lat: targetLink.lat,
          split_lng: targetLink.lng,
          junction_name: `${targetLink.linkName} Junction`,
          junction_node_type: 'junction',
          metadata: {
            created_from: 'installation_cable_map',
            work_order_id: workOrderId,
            customer_id: customerId,
            location_id: locationId,
          },
        });
        toast.success('Cable route saved and connected through a junction');
        dispatch('saved', {
          link_id: result?.created_connection_link?.id,
          from_node_id: sourceNodeId,
          to_node_id: result?.junction_node?.id || '',
        });
      } else {
        const saved = await api.networkMapping.links.create({
          name: baseName,
          link_type: 'fiber',
          status: 'up',
          from_node_id: sourceNodeId,
          to_node_id: targetNodeId,
          priority: 100,
          capacity_mbps: null,
          utilization_pct: null,
          loss_db: null,
          latency_ms: null,
          geometry: { type: 'LineString', coordinates: coords },
          metadata: {
            created_from: 'installation_cable_map',
            work_order_id: workOrderId,
            customer_id: customerId,
            location_id: locationId,
          },
        });
        toast.success('Cable route saved');
        dispatch('saved', {
          link_id: saved?.id,
          from_node_id: sourceNodeId,
          to_node_id: targetNodeId,
        });
      }
      await loadData();
      cancelDraw();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save cable route');
    } finally {
      saving = false;
    }
  }

  function parseGeometryCoords(geometry: GeoJSON.Geometry): Array<[number, number]> {
    if (!geometry) return [];
    if (geometry.type === 'LineString') {
      return ((geometry.coordinates as number[][]) || [])
        .filter((point) => Array.isArray(point) && point.length >= 2)
        .map((point) => [Number(point[0]), Number(point[1])]);
    }
    if (geometry.type === 'MultiLineString') {
      const merged: Array<[number, number]> = [];
      for (const line of (geometry.coordinates as number[][][]) || []) {
        for (const point of line || []) {
          if (!Array.isArray(point) || point.length < 2) continue;
          const candidate: [number, number] = [Number(point[0]), Number(point[1])];
          const prev = merged[merged.length - 1];
          if (prev && prev[0] === candidate[0] && prev[1] === candidate[1]) continue;
          merged.push(candidate);
        }
      }
      return merged;
    }
    return [];
  }

  function snapPointToGeometry(
    geometry: GeoJSON.Geometry,
    lng: number,
    lat: number,
  ): [number, number] | null {
    const coords = parseGeometryCoords(geometry);
    if (coords.length < 2) return null;

    let best: { point: [number, number]; distanceSq: number } | null = null;
    for (let i = 0; i < coords.length - 1; i += 1) {
      const a = coords[i];
      const b = coords[i + 1];
      const abx = b[0] - a[0];
      const aby = b[1] - a[1];
      const denom = abx * abx + aby * aby;
      const t =
        denom <= 1e-12
          ? 0
          : Math.max(
              0,
              Math.min(1, ((lng - a[0]) * abx + (lat - a[1]) * aby) / denom),
            );
      const candidate: [number, number] = [a[0] + abx * t, a[1] + aby * t];
      const dx = lng - candidate[0];
      const dy = lat - candidate[1];
      const distanceSq = dx * dx + dy * dy;
      if (!best || distanceSq < best.distanceSq) {
        best = { point: candidate, distanceSq };
      }
    }
    return best?.point || null;
  }

  function setSource(sourceId: string, data: GeoJSON.FeatureCollection) {
    if (!map?.getSource(sourceId)) return;
    const src = map.getSource(sourceId) as import('maplibre-gl').GeoJSONSource | undefined;
    src?.setData(data as any);
  }

  function emptyFC(): GeoJSON.FeatureCollection {
    return { type: 'FeatureCollection', features: [] };
  }

  function escapeHtml(input: unknown): string {
    return String(input ?? '-')
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#039;');
  }
</script>

<div class="icm-wrap">
  <div class="icm-toolbar">
    <button class="btn ghost mini" type="button" onclick={toggleDraw} disabled={loading || saving}>
      <Icon name="map-pin" size={14} />
      {drawing ? 'Drawing active' : 'Draw Cable'}
    </button>
    <button
      class="btn ghost mini"
      type="button"
      onclick={undoPoint}
      disabled={!drawing || bendPoints.length === 0 || saving}
    >
      <Icon name="arrow-left" size={14} />
      Undo
    </button>
    <button class="btn ghost mini" type="button" onclick={cancelDraw} disabled={!drawing || saving}>
      <Icon name="x-circle" size={14} />
      Cancel
    </button>
    <button
      class="btn mini"
      type="button"
      onclick={saveLink}
      disabled={saving || !drawing || !sourceNodeId || (!targetNodeId && !targetLink)}
    >
      <Icon name="save" size={14} />
      {saving ? 'Saving...' : 'Save Link'}
    </button>
  </div>

  <div class="icm-hints">
    {#if drawing}
      <span>
        Route starts from the customer marker. Click map for bend points, then click a node or an
        existing cable to finish.
      </span>
    {:else}
      <span>Start Draw Cable to create a route from the customer premise.</span>
    {/if}
    <div class="icm-badges">
      {#if customerCoord}
        <span class="tag">Customer marker focused</span>
      {/if}
      {#if sourceNodeId}
        <span class="tag ok">Customer node ready</span>
      {/if}
      {#if targetNodeId}
        <span class="tag">Target node selected</span>
      {:else if targetLink}
        <span class="tag warn">Existing cable selected</span>
      {/if}
    </div>
  </div>

  <div class="icm-map" bind:this={hostEl}></div>
</div>

<style>
  .icm-wrap {
    display: grid;
    gap: 8px;
  }
  .icm-toolbar {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }
  .icm-hints {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 0.82rem;
    color: #9fb0cc;
  }
  .icm-badges {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .tag {
    border: 1px solid #245d79;
    color: #8fe3ff;
    background: rgba(6, 182, 212, 0.14);
    border-radius: 999px;
    padding: 2px 8px;
    font-weight: 700;
  }
  .tag.ok {
    border-color: #166534;
    color: #bbf7d0;
    background: rgba(34, 197, 94, 0.15);
  }
  .tag.warn {
    border-color: #9a3412;
    color: #fdba74;
    background: rgba(249, 115, 22, 0.15);
  }
  .icm-map {
    width: 100%;
    height: clamp(420px, 62vh, 720px);
    border: 1px solid #2d3f61;
    border-radius: 12px;
    overflow: hidden;
    background: #08101d;
  }
  :global(.icm-map .maplibregl-ctrl-group button) {
    width: 28px;
    height: 28px;
  }
</style>
